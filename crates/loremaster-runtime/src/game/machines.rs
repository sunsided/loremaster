//! Machine steps with `on: next` (P5-02): they run by themselves, one
//! after another, in the same turn. A loop of them ends at the step limit.

use super::Game;
use crate::{BlockKind, LogEntry, Turn};
use loremaster_model::{EventPattern, Id};

/// The most `on: next` steps in one turn, without `step_limit` in the story.
pub(crate) const DEFAULT_STEP_LIMIT: u32 = 100;

impl Game {
    /// Run the `on: next` steps of a machine until none holds in its state.
    /// The first step that holds, in file order, runs. An await pauses the
    /// chain; [`Game::resume`] goes on with it.
    pub(crate) fn run_next(&mut self, turn: &mut Turn, machine: &Id, depth: u8) {
        let world = self.world.clone();
        let limit = world.story.step_limit.unwrap_or(DEFAULT_STEP_LIMIT);
        while !self.stopped && !self.state.ended {
            let next = world.rule_order.iter().copied().find(|&i| {
                let rule = &world.story.rules[i];
                rule.on == EventPattern::Next
                    && rule.machine.as_ref() == Some(machine)
                    && self.eval().holds_opt(&world.rules[i].when)
            });
            let Some(i) = next else { return };
            if turn.machine_steps >= limit {
                // One note per turn, also when more machines hit the limit.
                if !std::mem::replace(&mut turn.step_limit_noted, true) {
                    let text = format!(
                        "the machine `{machine}` stopped after {limit} automatic steps in this turn (`step_limit`)"
                    );
                    turn.log.push(LogEntry {
                        kind: "limit".into(),
                        text: text.clone(),
                    });
                    turn.say(BlockKind::Note, format!("({text})"));
                }
                return;
            }
            turn.machine_steps += 1;
            let rule = &world.story.rules[i];
            turn.log.push(LogEntry {
                kind: "rule".into(),
                text: rule.id.to_string(),
            });
            self.state.fired.insert(rule.id.clone());
            if !self.run_effects(turn, i, 0, depth) {
                return;
            }
        }
    }
}
