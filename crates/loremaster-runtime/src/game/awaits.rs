//! Awaits: a rule stops and waits for the host (SE-5, SE-6, XP-3).
//!
//! The position of the rule is data in `State` (`Awaiting`), not a Rust stack,
//! so a save in the middle of a rule restores. The rest of the turn runs as
//! usual; `resume` runs the rest of the paused rule, without a new turn.

use super::Game;
use crate::output::Choice;
use crate::rng;
use crate::state::{AwaitRequest, Awaiting};
use crate::world::CEffect;
use crate::{BlockKind, LogEntry, Turn};
use loremaster_model::{AwaitSpec, Dice, Id, Value};
use std::collections::BTreeMap;

/// What an `await` effect does to the rest of its rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AwaitStep {
    /// The session answered (a roll): the rule goes on.
    Answered,
    /// The rule pauses until `resume`.
    Paused,
    /// Another await is open: the rest of this rule does not run.
    Skipped,
}

/// The answers of a walk: the host walked the player there, or did not.
pub const ARRIVED: &str = "arrived";
pub const REFUSED: &str = "refused";

impl Game {
    /// Start the await at effect `next - 1` of `rule`. A roll that the
    /// session answers itself does not pause. While another await is open,
    /// the rest of the rule is skipped: its effects need the answer.
    pub(crate) fn begin_await(
        &mut self,
        turn: &mut Turn,
        rule: &Id,
        next: usize,
        spec: &AwaitSpec,
    ) -> AwaitStep {
        if self.state.awaiting.is_some() {
            // One await at a time: the first one of a turn wins.
            turn.log.push(LogEntry {
                kind: "await".into(),
                text: format!("{rule}: skipped, another await is open"),
            });
            return AwaitStep::Skipped;
        }
        if let Some(dice) = spec.roll.as_deref().and_then(Dice::parse)
            && (!self.host_rolls || !self.forced.totals.is_empty())
        {
            let roll = spec.roll.as_deref().unwrap_or_default();
            // A forced total (a walkthrough's `rolls:`) comes before the RNG.
            let total = match self.forced.totals.pop_front() {
                Some(n) if (dice.min()..=dice.max()).contains(&n) => n,
                Some(n) => {
                    self.forced.misfits.push(format!(
                        "the forced roll {n} is not in the range of {roll} ({} to {})",
                        dice.min(),
                        dice.max()
                    ));
                    self.roll_dice(dice)
                }
                None => self.roll_dice(dice),
            };
            turn.log.push(LogEntry {
                kind: "roll".into(),
                text: format!("{roll} = {total}"),
            });
            if let Some(into) = &spec.into {
                self.set_fact(turn, into, Value::Int(total));
            }
            return AwaitStep::Answered;
        }
        let Some(request) = AwaitRequest::of(spec) else {
            return AwaitStep::Answered;
        };
        turn.log.push(LogEntry {
            kind: "await".into(),
            text: format!("{rule} waits: {}", request.describe()),
        });
        self.state.awaiting = Some(Awaiting {
            id: format!("{rule}#{}@{}", next - 1, self.state.turns),
            rule: rule.clone(),
            next,
            request,
            into: spec.into.clone(),
            locals: BTreeMap::new(),
        });
        AwaitStep::Paused
    }

    /// The rule that waits for the host, if any.
    pub fn awaiting(&self) -> Option<&Awaiting> {
        self.state.awaiting.as_ref()
    }

    /// Answer the open await and run the rest of its rule. The answer must
    /// fit the request: an option id, a result of the action, a number in the
    /// range of the dice, or `arrived` / `refused` for a walk.
    pub fn resume(&mut self, id: &str, answer: Value) -> Result<Turn, String> {
        let Some(open) = self.state.awaiting.clone() else {
            return Err("nothing waits for an answer".into());
        };
        if open.id != id {
            return Err(format!("the open await is `{}`, not `{id}`", open.id));
        }
        if self.state.ended {
            return Err("the story has ended".into());
        }
        open.request.accepts(&answer)?;
        if let Some(into) = &open.into {
            self.check_answer_type(into, &answer)?;
        }
        let mut turn = Turn::default();
        self.state.awaiting = None;
        if let Some(into) = &open.into {
            self.set_fact(&mut turn, into, answer.clone());
        }
        if let AwaitRequest::WalkTo { room } = &open.request
            && answer == Value::Text(ARRIVED.into())
        {
            self.move_player(&mut turn, room.clone(), 1);
        }
        self.continue_rule(&mut turn, &open.rule, open.next);
        Ok(self.finish(turn))
    }

    /// Run the effects of `rule` from position `from` on.
    pub(crate) fn continue_rule(&mut self, turn: &mut Turn, rule: &Id, from: usize) {
        let world = self.world.clone();
        let Some(i) = world.story.rules.iter().position(|r| r.id == *rule) else {
            return;
        };
        let outer = std::mem::replace(&mut self.stopped, false);
        // After the answer, the `on: next` steps of a machine go on.
        if self.run_effects(turn, i, from, 1)
            && let Some(m) = &world.story.rules[i].machine
        {
            self.run_next(turn, m, 1);
        }
        self.stopped = outer;
    }

    /// Run the effects of rule `i` from position `from` on. False when the
    /// rule pauses at an await, or skips the rest of it.
    pub(crate) fn run_effects(
        &mut self,
        turn: &mut Turn,
        i: usize,
        from: usize,
        depth: u8,
    ) -> bool {
        let world = self.world.clone();
        let rule = &world.story.rules[i].id;
        for (k, effect) in world.rules[i].then.iter().enumerate().skip(from) {
            if let CEffect::Await(spec) = effect {
                if self.begin_await(turn, rule, k + 1, spec) != AwaitStep::Answered {
                    return false;
                }
                continue;
            }
            self.apply(turn, rule, effect, depth);
            if self.stopped || self.state.ended {
                break;
            }
        }
        true
    }

    /// The answers of the open await as choices, for the hosts that show a
    /// numbered list: the options of a choice, the results of an action, and
    /// `arrived` / `refused` for a walk. A roll that the host answers has none.
    pub(crate) fn await_choices(&self) -> Vec<Choice> {
        let Some(open) = &self.state.awaiting else {
            return Vec::new();
        };
        open.request
            .answers()
            .into_iter()
            .enumerate()
            .map(|(index, (_, text))| Choice { index, text })
            .collect()
    }

    /// Typed input while a rule waits: a number of the list, or an answer
    /// as written (an option id or its text, a result, a rolled number).
    pub(crate) fn await_input(&mut self, turn: &mut Turn, text: &str) {
        let Some(open) = self.state.awaiting.clone() else {
            return;
        };
        let text = text.trim();
        let answers = open.request.answers();
        let by_number = text
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| answers.get(i))
            .map(|(v, _)| v.clone());
        let by_name = answers
            .iter()
            .find(|(v, label)| {
                label.eq_ignore_ascii_case(text) || v.to_string().eq_ignore_ascii_case(text)
            })
            .map(|(v, _)| v.clone());
        let rolled = match &open.request {
            AwaitRequest::Roll { .. } => text.parse::<i64>().ok().map(Value::Int),
            _ => None,
        };
        match by_number.or(by_name).or(rolled) {
            Some(answer) => match self.resume(&open.id, answer) {
                Ok(done) => {
                    turn.blocks.extend(done.blocks);
                    turn.log.extend(done.log);
                }
                Err(why) => turn.say(BlockKind::Response, why),
            },
            None => turn.say(
                BlockKind::Response,
                format!("The story waits: {}.", open.request.describe()),
            ),
        }
    }

    /// The answer must fit a declared variable: a number for `int`, a
    /// value of the list for an enum, true or false for `bool`. A name that
    /// is no variable is a property and takes any value.
    fn check_answer_type(&self, into: &str, answer: &Value) -> Result<(), String> {
        use loremaster_model::VarType;
        let Some(var) = self.world.story.var(into) else {
            return Ok(());
        };
        let fits = match (&var.ty, answer) {
            (VarType::Int, Value::Int(_)) | (VarType::Bool, Value::Bool(_)) => true,
            (VarType::Enum(values), Value::Text(t)) => values.contains(t),
            _ => false,
        };
        if fits {
            Ok(())
        } else {
            Err(format!("`{into}` cannot hold the answer `{answer}`"))
        }
    }

    fn roll_dice(&mut self, dice: Dice) -> i64 {
        let sides = dice.sides as usize;
        let mut total = dice.bonus;
        for _ in 0..dice.count {
            total += rng::below(&mut self.state.rng, sides) as i64 + 1;
        }
        total
    }
}
