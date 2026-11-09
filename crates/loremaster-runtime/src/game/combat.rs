//! Combat (P5-03): the rules of the built-in combat, for the `combat:` effect.
//!
//! The round logic is here and the author gives only data, so it also works
//! in builds that can not register a Rust behavior (the web player and
//! `loremaster test`). The position is an ordinary `Awaiting` and the rest is
//! in `State`, so a save in the middle of a combat restores.
//!
//! Special rules that react to a round are rules on the combat events
//! (#230): `combat_hit` before a hit counts, `combat_round` at the end of a
//! round and `combat_end` when the combat is over. A rule on `combat_hit`
//! can change `combat_hit.damage`, and `end_combat` leaves to any room.

use super::Game;
use super::awaits::AwaitStep;
use crate::state::{AwaitOptionInfo, AwaitRequest, CombatState, LastRound};
use crate::{BlockKind, PLAYER, Turn};
use loremaster_model::{COMBAT_END, COMBAT_HIT, COMBAT_ROUND, CombatSpec, Dice, Id, Value};
use std::collections::BTreeMap;

pub(crate) const ATTACK_TEXT: &str = "Attack";
pub(crate) const LUCK_TEXT: &str = "Test your Luck";
const ATTACK: &str = "attack";
const ESCAPE: &str = "escape";
const LUCK: &str = "luck";
const GO_ON: &str = "go_on";
/// The prefix of the option to attack another enemy than the first.
const TARGET: &str = "attack.";
/// Enemy properties for special rules: the STAMINA a hit of the enemy
/// costs the player, and the STAMINA a hit of the player costs the enemy.
const DEALS: &str = "deals";
const TAKES: &str = "takes";

/// Damage of a hit without `deals` or `takes`, and of an escape.
const HIT: i64 = 2;
const ROLL: Dice = Dice {
    count: 2,
    sides: 6,
    bonus: 0,
};
/// The rounds that the log keeps.
const LOG_LINES: usize = 8;

impl Game {
    /// Start the combat at effect `next - 1` of `rule`. While another await
    /// is open, the rest of the rule is skipped, as for an `await`.
    pub(crate) fn begin_combat(
        &mut self,
        turn: &mut Turn,
        rule: &Id,
        next: usize,
        spec: &CombatSpec,
        depth: u8,
    ) -> AwaitStep {
        if self.state.awaiting.is_some() {
            turn.log.push(crate::LogEntry {
                kind: "await".into(),
                text: format!("{rule}: skipped, another await is open"),
            });
            return AwaitStep::Skipped;
        }
        self.combat_exit = None;
        self.state.combat = Some(CombatState {
            spec: spec.clone(),
            round: 0,
            last: None,
            pending: None,
            by: None,
            target: None,
            rolls: Vec::new(),
            log: Vec::new(),
        });
        self.combat_go_on(turn, rule, next, depth)
    }

    /// Handle the answer to a combat await. `Answered` means the combat is
    /// over and the rule goes on.
    pub(crate) fn combat_answer(
        &mut self,
        turn: &mut Turn,
        rule: &Id,
        next: usize,
        answer: &Value,
    ) -> AwaitStep {
        match answer {
            Value::Text(a) if a == ESCAPE => self.combat_escape(turn),
            Value::Text(a) if a == LUCK => self.combat_luck(turn, rule, next),
            Value::Text(a) if a == GO_ON => {
                if let Some(hit) = self.pending_hit() {
                    let damage = self.hit_damage(&hit);
                    self.combat_hit(turn, &hit, damage);
                }
                self.combat_round_over(turn, rule, next)
            }
            // A total that the host rolled, for the round or the Luck test.
            Value::Int(total) => {
                let luck = (self.state.combat.as_mut()).is_some_and(|c| {
                    c.rolls.push(*total);
                    c.pending.is_some()
                });
                if luck {
                    self.combat_luck(turn, rule, next)
                } else {
                    self.combat_round(turn, rule, next)
                }
            }
            // `attack`, or `attack.<enemy>` for a target of an all-at-once combat.
            Value::Text(a) => {
                let target = a.strip_prefix(TARGET).map(Id::new);
                if let Some(c) = &mut self.state.combat {
                    c.target = target;
                }
                self.combat_round(turn, rule, next)
            }
            _ => self.combat_round(turn, rule, next),
        }
    }

    /// The STAMINA that the hit on `hit` costs: 2, or the `takes` of the
    /// enemy hit, or the `deals` of the enemy that hit the player.
    fn hit_damage(&self, hit: &Id) -> i64 {
        let prop =
            |e: &Id, p: &str| (self.state.facts.get(&format!("{e}.{p}"))).and_then(Value::as_int);
        let by = self.state.combat.as_ref().and_then(|c| c.by.as_ref());
        let damage = if hit.as_str() == PLAYER {
            by.and_then(|e| prop(e, DEALS))
        } else {
            prop(hit, TAKES)
        };
        damage.unwrap_or(HIT).max(0)
    }

    /// The host rolls this combat's dice and fewer than `needed` totals are in.
    fn combat_waits_for_rolls(&self, needed: usize) -> bool {
        self.host_rolls
            && self.forced.totals.is_empty()
            && (self.state.combat.as_ref()).is_some_and(|c| c.rolls.len() < needed)
    }

    fn combat_roll_await(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        let request = AwaitRequest::Roll {
            dice: "2d6".into(),
            min: ROLL.min(),
            max: ROLL.max(),
        };
        self.open_await(turn, rule, next, request, None)
    }

    /// How many enemies roll in the next round.
    pub(crate) fn combat_fighters(&self) -> usize {
        self.state.combat.as_ref().map_or(0, |c| {
            let living = self.living(&c.spec).len();
            if c.spec.together {
                living
            } else {
                living.min(1)
            }
        })
    }

    fn int(&self, name: &str) -> i64 {
        match self.state.facts.get(name) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        }
    }

    pub(super) fn living(&self, spec: &CombatSpec) -> Vec<Id> {
        (spec.enemies.iter())
            .filter(|e| self.int(&format!("{e}.stamina")) > 0)
            .cloned()
            .collect()
    }

    fn enemy_name(&self, enemy: &Id) -> String {
        (self.world.story.thing(enemy.as_str()))
            .map_or_else(|| enemy.to_string(), |t| t.name.clone())
    }

    fn combat_say(&mut self, turn: &mut Turn, line: String) {
        if let Some(c) = &mut self.state.combat {
            c.log.push(line.clone());
            if c.log.len() > LOG_LINES {
                c.log.remove(0);
            }
        }
        turn.say(BlockKind::Narration, line);
    }

    /// A roll of the combat, with the name for the dice display.
    fn named_roll(&mut self, turn: &mut Turn, who: String) -> i64 {
        let host = (self.state.combat.as_mut())
            .filter(|c| !c.rolls.is_empty())
            .map(|c| c.rolls.remove(0));
        let total = match host {
            Some(total) => Self::record_roll(turn, "2d6", total, Vec::new()),
            None => self.take_roll(turn, ROLL, "2d6"),
        };
        if let Some(r) = turn.rolls.last_mut() {
            r.who = Some(who);
        }
        total
    }

    fn pending_hit(&mut self) -> Option<Id> {
        self.state.combat.as_mut().and_then(|c| c.pending.take())
    }

    /// The end check, then the next round's choice.
    fn combat_go_on(&mut self, turn: &mut Turn, rule: &Id, next: usize, depth: u8) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        if let Some(room) = self.combat_exit.take() {
            return self.combat_leave(turn, room, "ended", depth);
        }
        if self.int("stamina") <= 0 {
            return self.combat_leave(turn, spec.loss, "loss", depth);
        }
        if self.living(&spec).is_empty() {
            return self.combat_leave(turn, spec.win, "win", depth);
        }
        // All at once, the player chooses the target; `attack` is the first.
        let living = self.living(&spec);
        let mut options: Vec<(String, String)> = if spec.together && living.len() > 1 {
            (living.iter().enumerate())
                .map(|(i, e)| {
                    let id = if i == 0 {
                        ATTACK.to_string()
                    } else {
                        format!("{TARGET}{e}")
                    };
                    (id, format!("{ATTACK_TEXT} {}", self.enemy_name(e)))
                })
                .collect()
        } else {
            vec![(ATTACK.into(), ATTACK_TEXT.into())]
        };
        if spec.escape.is_some() {
            options.push((ESCAPE.into(), "Escape".into()));
        }
        self.combat_ask(turn, rule, next, options)
    }

    fn combat_ask(
        &mut self,
        turn: &mut Turn,
        rule: &Id,
        next: usize,
        options: Vec<(String, String)>,
    ) -> AwaitStep {
        let options = (options.into_iter())
            .map(|(id, text)| AwaitOptionInfo {
                id: Id::new(id),
                text,
            })
            .collect();
        self.open_await(turn, rule, next, AwaitRequest::Choice { options }, None)
    }

    /// End the combat in `room`, then send `combat_end` with the outcome.
    fn combat_leave(&mut self, turn: &mut Turn, room: Id, outcome: &str, depth: u8) -> AwaitStep {
        let enemy = (self.state.combat.take()).and_then(|c| c.spec.enemies.first().cloned());
        self.move_player(turn, room, depth);
        if self.combat_heard(COMBAT_END) {
            let args = [
                (
                    "enemy",
                    Value::Text(enemy.map(|e| e.to_string()).unwrap_or_default()),
                ),
                ("outcome", Value::Text(outcome.into())),
            ];
            self.combat_event(turn, COMBAT_END, args, depth);
        }
        AwaitStep::Answered
    }

    /// Does a rule listen for the combat event? Without one, the combat
    /// sets no facts for it.
    fn combat_heard(&self, name: &str) -> bool {
        self.world.combat_heard.contains(name)
    }

    /// Keep the arguments of a combat event as `event.arg`, as for a host
    /// event, and run the rules that listen for it.
    fn combat_event<const N: usize>(
        &mut self,
        turn: &mut Turn,
        name: &str,
        args: [(&str, Value); N],
        depth: u8,
    ) {
        for (arg, value) in args {
            self.set_fact(turn, &format!("{name}.{arg}"), value);
        }
        turn.log.push(crate::LogEntry {
            kind: "event".into(),
            text: name.to_string(),
        });
        self.fire(turn, &crate::Event::Custom(Id::new(name)), depth);
    }

    /// A hit: the rules on `combat_hit` can change its damage, then it counts.
    fn combat_hit(&mut self, turn: &mut Turn, hit: &Id, damage: i64) {
        if !self.combat_heard(COMBAT_HIT) {
            return self.combat_damage(turn, hit, damage);
        }
        let c = self.state.combat.as_ref();
        let round = c.map_or(0, |c| c.round);
        let enemy = if hit.as_str() == PLAYER {
            c.and_then(|c| c.by.clone()).unwrap_or_else(|| hit.clone())
        } else {
            hit.clone()
        };
        let args = [
            ("enemy", Value::Text(enemy.to_string())),
            ("target", Value::Text(hit.to_string())),
            ("damage", Value::Int(damage)),
            ("round", Value::Int(i64::from(round))),
        ];
        self.combat_event(turn, COMBAT_HIT, args, 1);
        let damage = self.int(&format!("{COMBAT_HIT}.damage")).max(0);
        self.combat_damage(turn, hit, damage);
    }

    /// The round is over: `combat_round` (its arguments are set when the
    /// round rolls), then the end check.
    fn combat_round_over(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        if self.combat_exit.is_none() && self.combat_heard(COMBAT_ROUND) {
            turn.log.push(crate::LogEntry {
                kind: "event".into(),
                text: COMBAT_ROUND.to_string(),
            });
            self.fire(turn, &crate::Event::Custom(Id::new(COMBAT_ROUND)), 1);
        }
        self.combat_go_on(turn, rule, next, 1)
    }

    fn combat_escape(&mut self, turn: &mut Turn) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        self.combat_say(turn, format!("You escape, and lose {HIT} STAMINA."));
        self.combat_damage(turn, &Id::new(PLAYER), HIT);
        let (room, outcome) = match spec.escape {
            Some(room) if self.int("stamina") > 0 => (room, "escape"),
            _ => (spec.loss, "loss"),
        };
        self.combat_leave(turn, room, outcome, 1)
    }

    /// One round: the enemies roll, then the player. The player attacks the
    /// chosen enemy, or the first living one; the others of an all-at-once
    /// combat only attack. When the host rolls, the round first waits for
    /// one total per roll.
    fn combat_round(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        let mut fighting = self.living(&spec);
        if !spec.together {
            fighting.truncate(1);
        }
        let chosen = (self.state.combat.as_ref())
            .and_then(|c| c.target.clone())
            .filter(|t| fighting.contains(t));
        let Some(target) = chosen.or_else(|| fighting.first().cloned()) else {
            return self.combat_go_on(turn, rule, next, 1);
        };
        if self.combat_waits_for_rolls(fighting.len() + 1) {
            return self.combat_roll_await(turn, rule, next);
        }
        let mut enemies = BTreeMap::new();
        for e in &fighting {
            let name = self.enemy_name(e);
            let total = self.named_roll(turn, name);
            enemies.insert(
                e.clone(),
                total.saturating_add(self.int(&format!("{e}.skill"))),
            );
        }
        let player = self
            .named_roll(turn, "You".into())
            .saturating_add(self.int("skill"));
        let mut round = 0;
        if let Some(c) = &mut self.state.combat {
            c.round += 1;
            round = c.round;
            c.last = Some(LastRound {
                player,
                enemies: enemies.clone(),
            });
            c.target = None;
        }
        if self.combat_heard(COMBAT_ROUND) {
            let args = [
                ("enemy", Value::Text(target.to_string())),
                ("round", Value::Int(i64::from(round))),
                ("won", Value::Bool(player > enemies[&target])),
            ];
            for (arg, value) in args {
                self.set_fact(turn, &format!("{COMBAT_ROUND}.{arg}"), value);
            }
        }
        // A Luck test changes the last hit of the round; the others count at once.
        let mut hit: Option<(Id, Id)> = None;
        for e in &fighting {
            let name = self.enemy_name(e);
            let enemy = enemies[e];
            let (new_hit, what) = if enemy > player {
                (Some(Id::new(PLAYER)), format!("{name} hits you."))
            } else if player > enemy && *e == target {
                (Some(e.clone()), format!("You hit {name}."))
            } else {
                (None, format!("You and {name} hold each other off."))
            };
            self.combat_say(
                turn,
                format!("Round {round}: you {player}, {name} {enemy}. {what}"),
            );
            if let Some(new_hit) = new_hit
                && let Some(earlier) = hit.replace((new_hit, e.clone()))
            {
                self.combat_strike(turn, earlier);
            }
        }
        let Some((hit, by)) = hit else {
            return self.combat_round_over(turn, rule, next);
        };
        // A rule on an earlier hit of the round ended the combat: the whole
        // round still counts, without a Luck test, then the combat ends.
        if self.combat_exit.is_some() {
            self.combat_strike(turn, (hit, by));
            return self.combat_go_on(turn, rule, next, 1);
        }
        if let Some(c) = &mut self.state.combat {
            c.by = Some(by);
        }
        // No test where it can not help: a hit that already kills the enemy,
        // a wound that kills the player even when lucky, or one that costs nothing.
        let damage = self.hit_damage(&hit);
        let helps = if hit.as_str() == PLAYER {
            damage > 0 && self.int("stamina") > damage - 1
        } else {
            self.int(&format!("{hit}.stamina")) > damage
        };
        if helps && self.int("luck") > 0 && self.int("stamina") > 0 {
            if let Some(c) = &mut self.state.combat {
                c.pending = Some(hit);
            }
            let options = [(LUCK, LUCK_TEXT), (GO_ON, "Go on")];
            let options = options.map(|(id, text)| (id.to_string(), text.to_string()));
            return self.combat_ask(turn, rule, next, options.into());
        }
        self.combat_hit(turn, &hit, damage);
        self.combat_round_over(turn, rule, next)
    }

    /// A hit of the round that no Luck test can change: `(hit, by)`.
    fn combat_strike(&mut self, turn: &mut Turn, (hit, by): (Id, Id)) {
        if let Some(c) = &mut self.state.combat {
            c.by = Some(by);
        }
        let damage = self.hit_damage(&hit);
        self.combat_hit(turn, &hit, damage);
    }

    /// The Luck test on the pending hit: it costs 1 LUCK and changes the
    /// damage. When the host rolls, it first waits for the total.
    fn combat_luck(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        if self.combat_waits_for_rolls(1) {
            return self.combat_roll_await(turn, rule, next);
        }
        let Some(hit) = self.pending_hit() else {
            return self.combat_go_on(turn, rule, next, 1);
        };
        let luck = self.int("luck");
        let roll = self.named_roll(turn, "LUCK".into());
        let lucky = roll <= luck;
        self.set_fact(turn, "luck", Value::Int((luck - 1).max(0)));
        let word = if lucky { "lucky" } else { "unlucky" };
        if let Some(r) = turn.rolls.last_mut() {
            r.target = Some(luck);
            r.result = Some(word.to_string());
        }
        self.combat_say(
            turn,
            format!("Test your Luck: {roll} against {luck}: {word}."),
        );
        let base = self.hit_damage(&hit);
        let damage = match (hit.as_str() == PLAYER, lucky) {
            (true, true) => base - 1,
            (true, false) => base + 1,
            (false, true) => base + 2,
            (false, false) => base - 1,
        };
        self.combat_hit(turn, &hit, damage.max(0));
        self.combat_round_over(turn, rule, next)
    }

    fn combat_damage(&mut self, turn: &mut Turn, target: &Id, damage: i64) {
        let fact = if target.as_str() == PLAYER {
            "stamina".to_string()
        } else {
            format!("{target}.stamina")
        };
        self.set_fact(
            turn,
            &fact,
            Value::Int(self.int(&fact).saturating_sub(damage).max(0)),
        );
    }

    /// The combat as the panel shows it.
    pub(crate) fn combat_view(&self) -> Option<crate::CombatView> {
        let c = self.state.combat.as_ref()?;
        let strength = |id: &Id| c.last.as_ref().and_then(|l| l.enemies.get(id)).copied();
        Some(crate::CombatView {
            round: c.round,
            enemies: (c.spec.enemies.iter())
                .map(|e| crate::EnemyView {
                    id: e.clone(),
                    name: self.enemy_name(e),
                    skill: self.int(&format!("{e}.skill")),
                    stamina: self.int(&format!("{e}.stamina")),
                    strength: strength(e),
                })
                .collect(),
            player_strength: c.last.as_ref().map(|l| l.player),
            log: c.log.clone(),
        })
    }
}
