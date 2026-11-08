//! Combat (P5-03): the rules of the built-in combat, for the `combat:` effect.
//!
//! The round logic is here and the author gives only data, so it also works
//! in builds that can not register a Rust behavior (the web player and
//! `loremaster test`). The position is an ordinary `Awaiting` and the rest is
//! in `State`, so a save in the middle of a combat restores.

use super::Game;
use super::awaits::AwaitStep;
use crate::state::{AwaitOptionInfo, AwaitRequest, CombatState, LastRound};
use crate::{BlockKind, PLAYER, Turn};
use loremaster_model::{CombatSpec, Dice, Id, Value};
use std::collections::BTreeMap;

pub(crate) const ATTACK_TEXT: &str = "Attack";
pub(crate) const LUCK_TEXT: &str = "Test your Luck";
const ATTACK: &str = "attack";
const ESCAPE: &str = "escape";
const LUCK: &str = "luck";
const GO_ON: &str = "go_on";

/// Damage of a hit, and of an escape.
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
    ) -> AwaitStep {
        if self.state.awaiting.is_some() {
            turn.log.push(crate::LogEntry {
                kind: "await".into(),
                text: format!("{rule}: skipped, another await is open"),
            });
            return AwaitStep::Skipped;
        }
        self.state.combat = Some(CombatState {
            spec: spec.clone(),
            round: 0,
            last: None,
            pending: None,
            log: Vec::new(),
        });
        self.combat_go_on(turn, rule, next)
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
            Value::Text(a) if a == LUCK => {
                self.combat_luck(turn);
                self.combat_go_on(turn, rule, next)
            }
            Value::Text(a) if a == GO_ON => {
                if let Some(hit) = self.pending_hit() {
                    self.combat_damage(turn, &hit, HIT);
                }
                self.combat_go_on(turn, rule, next)
            }
            _ => self.combat_round(turn, rule, next),
        }
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

    fn living(&self, spec: &CombatSpec) -> Vec<Id> {
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

    fn pending_hit(&mut self) -> Option<Id> {
        self.state.combat.as_mut().and_then(|c| c.pending.take())
    }

    /// The end check, then the next round's choice.
    fn combat_go_on(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        if self.int("stamina") <= 0 {
            return self.combat_leave(turn, spec.loss);
        }
        if self.living(&spec).is_empty() {
            return self.combat_leave(turn, spec.win);
        }
        let mut options = vec![(ATTACK, ATTACK_TEXT)];
        if spec.escape.is_some() {
            options.push((ESCAPE, "Escape"));
        }
        self.combat_ask(turn, rule, next, &options)
    }

    fn combat_ask(
        &mut self,
        turn: &mut Turn,
        rule: &Id,
        next: usize,
        options: &[(&str, &str)],
    ) -> AwaitStep {
        let options = (options.iter())
            .map(|(id, text)| AwaitOptionInfo {
                id: Id::new(*id),
                text: (*text).to_string(),
            })
            .collect();
        self.open_await(turn, rule, next, AwaitRequest::Choice { options }, None)
    }

    fn combat_leave(&mut self, turn: &mut Turn, room: Id) -> AwaitStep {
        self.state.combat = None;
        self.move_player(turn, room, 1);
        AwaitStep::Answered
    }

    fn combat_escape(&mut self, turn: &mut Turn) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        self.combat_say(turn, format!("You escape, and lose {HIT} STAMINA."));
        self.combat_damage(turn, &Id::new(PLAYER), HIT);
        let room = match spec.escape {
            Some(room) if self.int("stamina") > 0 => room,
            _ => spec.loss,
        };
        self.combat_leave(turn, room)
    }

    /// One round: the enemies roll, then the player. The player attacks the
    /// first living enemy; the others of an all-at-once combat only attack.
    fn combat_round(&mut self, turn: &mut Turn, rule: &Id, next: usize) -> AwaitStep {
        let Some(spec) = self.state.combat.as_ref().map(|c| c.spec.clone()) else {
            return AwaitStep::Answered;
        };
        let mut fighting = self.living(&spec);
        if !spec.together {
            fighting.truncate(1);
        }
        let Some(target) = fighting.first().cloned() else {
            return self.combat_go_on(turn, rule, next);
        };
        let mut enemies = BTreeMap::new();
        for e in &fighting {
            let total = self.take_roll(turn, ROLL, "2d6");
            enemies.insert(e.clone(), total + self.int(&format!("{e}.skill")));
        }
        let player = self.take_roll(turn, ROLL, "2d6") + self.int("skill");
        let mut round = 0;
        if let Some(c) = &mut self.state.combat {
            c.round += 1;
            round = c.round;
            c.last = Some(LastRound {
                player,
                enemies: enemies.clone(),
            });
        }
        // A Luck test changes the last hit of the round; the others count at once.
        let mut hit: Option<Id> = None;
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
                && let Some(earlier) = hit.replace(new_hit)
            {
                self.combat_damage(turn, &earlier, HIT);
            }
        }
        let Some(hit) = hit else {
            return self.combat_go_on(turn, rule, next);
        };
        // No test where it can not help: a hit that already kills the enemy.
        let helps = hit.as_str() == PLAYER || self.int(&format!("{hit}.stamina")) > HIT;
        if helps && self.int("luck") > 0 && self.int("stamina") > 0 {
            if let Some(c) = &mut self.state.combat {
                c.pending = Some(hit);
            }
            return self.combat_ask(turn, rule, next, &[(LUCK, LUCK_TEXT), (GO_ON, "Go on")]);
        }
        self.combat_damage(turn, &hit, HIT);
        self.combat_go_on(turn, rule, next)
    }

    /// The Luck test on the pending hit: it costs 1 LUCK and changes the damage.
    fn combat_luck(&mut self, turn: &mut Turn) {
        let Some(hit) = self.pending_hit() else {
            return;
        };
        let luck = self.int("luck");
        let roll = self.take_roll(turn, ROLL, "2d6");
        let lucky = roll <= luck;
        self.set_fact(turn, "luck", Value::Int((luck - 1).max(0)));
        let word = if lucky { "lucky" } else { "unlucky" };
        self.combat_say(
            turn,
            format!("Test your Luck: {roll} against {luck}: {word}."),
        );
        let damage = match (hit.as_str() == PLAYER, lucky) {
            (true, true) => HIT - 1,
            (true, false) => HIT + 1,
            (false, true) => HIT + 2,
            (false, false) => HIT - 1,
        };
        self.combat_damage(turn, &hit, damage);
    }

    fn combat_damage(&mut self, turn: &mut Turn, target: &Id, damage: i64) {
        let fact = if target.as_str() == PLAYER {
            "stamina".to_string()
        } else {
            format!("{target}.stamina")
        };
        self.set_fact(turn, &fact, Value::Int((self.int(&fact) - damage).max(0)));
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
