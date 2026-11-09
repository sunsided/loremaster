//! The hint ladder (CK-6, UI-1): hints per room, one level at a time, from
//! the walkthrough files (US-05, US-41).
//!
//! The steps of the walkthroughs with an `id` or `hints` (CK-4) make a
//! [`HintBook`]. A host gives it to a session with
//! [`Session::set_hints`]. The session marks a step done when the player
//! gives its command in its room and the step's `facts` hold after it, or
//! when the step's `facts` come to hold. A step with `event` has no hints;
//! it is done when the session accepts a report of its event with the
//! step's `args` and its `facts` hold after it. [`Session::hints`] shows the steps
//! of the current room that are not done and whose `needs` are done.
//! [`Session::hint`] and the typed word `hint` show the next level: the
//! hints of the step, then its command.

use super::{Input, Session, Step};
use crate::{Command, State, Turn, World};
use loremaster_model::{Id, Value, Walkthrough};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The typed word that shows the next hint, unless the story has a verb
/// with that word.
pub const HINT: &str = "hint";

/// The steps of the hint ladder, from the walkthroughs of a project.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HintBook {
    steps: Vec<HintStep>,
}

#[derive(Debug, Clone, PartialEq)]
struct HintStep {
    /// The id of the step, or `<walkthrough>#<n>` for a step with hints and
    /// no id.
    key: String,
    command: String,
    /// The room of the step: its `at`, or the room the walkthrough is in
    /// before the step.
    room: Id,
    /// The keys of the steps that must be done first.
    needs: Vec<String>,
    /// The host event that the step reports, for a step without a command.
    event: Option<Id>,
    /// The arguments of `event`: a report with others does not do the step.
    args: BTreeMap<String, Value>,
    hints: Vec<String>,
    /// The facts that hold after the step, from its checks.
    facts: BTreeMap<String, Value>,
}

impl HintBook {
    /// The steps with an id or hints of the walkthroughs that are not
    /// pending. Each walkthrough plays once from the start, as
    /// [`run_walkthrough`](crate::run_walkthrough) plays it, for the room of
    /// each step without `at`; that is one play per call, so a host builds
    /// the book once per build of the story. A step id that two walkthroughs
    /// use is one step: the first one counts, and `loremaster check` warns
    /// when the two steps differ.
    pub fn new(world: Arc<World>, walks: &[(String, Walkthrough)]) -> Self {
        let mut steps: Vec<HintStep> = Vec::new();
        for (key, walk) in walks {
            if walk.pending.is_some() || !walk.is_planned() {
                continue;
            }
            let rooms = rooms_before_steps(world.clone(), walk);
            let chart = loremaster_model::step_chart(key, walk);
            for (i, step) in walk.steps.iter().enumerate() {
                let Some(plan) = step.plan() else { continue };
                if plan.id.is_none() && plan.hints.is_empty() {
                    continue;
                }
                let key = match &plan.id {
                    Some(id) => id.to_string(),
                    None => format!("{key}#{}", i + 1),
                };
                if steps.iter().any(|s| s.key == key) {
                    continue;
                }
                let needs = chart
                    .as_ref()
                    .and_then(|c| c.steps.iter().find(|s| s.index == i))
                    .map(|s| s.needs.iter().map(Id::to_string).collect())
                    // A step without an id is not in the chart: its own `needs` gate it.
                    .unwrap_or_else(|| plan.needs.iter().map(Id::to_string).collect());
                steps.push(HintStep {
                    key,
                    command: step.command().to_string(),
                    room: plan.at.clone().unwrap_or_else(|| rooms[i].clone()),
                    needs,
                    // The host reports an event; the player cannot type it.
                    // The step stays in the book, so `needs` can name it.
                    hints: if plan.event.is_some() {
                        Vec::new()
                    } else {
                        plan.hints.clone()
                    },
                    event: plan.event.clone(),
                    args: plan.args.clone(),
                    facts: step.expect().map(|e| e.facts.clone()).unwrap_or_default(),
                });
            }
        }
        HintBook { steps }
    }

    /// No step has hints.
    pub fn is_empty(&self) -> bool {
        self.steps.iter().all(|s| s.hints.is_empty())
    }

    /// The keys of the steps that have hints, in walkthrough order.
    pub fn steps(&self) -> impl Iterator<Item = &str> {
        self.steps
            .iter()
            .filter(|s| !s.hints.is_empty())
            .map(|s| s.key.as_str())
    }
}

/// The room of the player before each step of a walkthrough.
fn rooms_before_steps(world: Arc<World>, walk: &Walkthrough) -> Vec<Id> {
    let mut session = Session::new(world);
    session.start();
    let mut rooms = Vec::with_capacity(walk.steps.len());
    for step in &walk.steps {
        rooms.push(session.state().room.clone());
        match step.plan().and_then(|p| Some((p.event.as_ref()?, &p.args))) {
            Some((event, args)) => {
                let _ = session.host_event(event.as_str(), args.clone());
            }
            None => {
                session.submit(Input::Text(step.command().into()));
            }
        }
    }
    rooms
}

/// A step of the current room for the hint ladder, with the levels shown so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Hint {
    /// The key of the step: its id, or `<walkthrough>#<n>`.
    pub step: String,
    /// The hints shown so far, the first one first.
    pub shown: Vec<String>,
    /// The command, once the hints are all shown and the player asked once more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// How many levels the ladder of the step has: its hints and the command.
    pub levels: usize,
}

impl Hint {
    /// All levels are shown.
    pub fn complete(&self) -> bool {
        self.command.is_some()
    }
}

fn facts_hold(state: &State, facts: &BTreeMap<String, Value>) -> bool {
    facts.iter().all(|(k, v)| state.facts.get(k) == Some(v))
}

impl Session {
    /// The hint ladder of this game, from the walkthroughs (CK-6). Without
    /// it, the session has no hints.
    pub fn set_hints(&mut self, book: HintBook) {
        self.hints = Arc::new(book);
    }

    /// [`Session::set_hints`] with the hint ladder of these walkthroughs.
    pub fn set_hints_from(&mut self, walks: &[(String, Walkthrough)]) {
        let book = HintBook::new(self.game.world_arc(), walks);
        self.set_hints(book);
    }

    /// Build the hint book again in the world of a reload, and fit the
    /// player's progress to it: steps the book no longer has are forgotten,
    /// and a shorter ladder must not show its command unasked. A command stays
    /// shown only when the player saw this same command before. Adds the done
    /// steps it forgets to `dropped`, as `step:<key>`. An empty book is no
    /// ladder to compare against, so the progress then stays as it is.
    pub(super) fn rebuild_hints(
        &mut self,
        walks: &[(String, Walkthrough)],
        dropped: &mut Vec<String>,
    ) {
        let old = self.hints.clone();
        self.set_hints_from(walks);
        let book = self.hints.clone();
        // For example a session from a save, which has no walkthroughs yet.
        if book.steps.is_empty() {
            return;
        }
        let find = |b: &'_ HintBook, key: &str| b.steps.iter().position(|s| s.key == key);
        self.revealed.retain(|key, level| {
            let Some(i) = find(&book, key) else {
                return false;
            };
            let step = &book.steps[i];
            let saw_command = find(&old, key)
                .map(|o| &old.steps[o])
                .is_some_and(|o| *level > o.hints.len() && o.command == step.command);
            *level = (*level).min(step.hints.len() + usize::from(saw_command));
            true
        });
        self.game.state_mut().steps_done.retain(|key| {
            let keep = find(&book, key).is_some();
            if !keep {
                dropped.push(format!("step:{key}"));
            }
            keep
        });
    }

    /// The story has hints at all, also when none is open in this room now.
    pub fn has_hints(&self) -> bool {
        !self.hints.is_empty()
    }

    /// The steps of the current room that the player has not done and can do
    /// now (their `needs` are done), each with the levels shown so far (UI-1).
    pub fn hints(&self) -> Vec<Hint> {
        self.open_steps()
            .map(|s| {
                let level = self.revealed.get(&s.key).copied().unwrap_or(0);
                Hint {
                    step: s.key.clone(),
                    shown: s.hints.iter().take(level).cloned().collect(),
                    command: (level > s.hints.len()).then(|| s.command.clone()),
                    levels: s.hints.len() + 1,
                }
            })
            .collect()
    }

    /// Show the next level of the hint ladder: of `step`, or of the first
    /// step of the current room that has a level left. The step has a note
    /// with the hint, and changes nothing in the game.
    pub fn hint(&mut self, step: Option<&str>) -> Step {
        let turn = self.hint_turn(step);
        self.step(turn)
    }

    pub(crate) fn hint_turn(&mut self, step: Option<&str>) -> Turn {
        let open: Vec<&HintStep> = self.open_steps().collect();
        let pick = match step {
            Some(key) => open.iter().find(|s| s.key == key),
            None => open
                .iter()
                .find(|s| self.revealed.get(&s.key).copied().unwrap_or(0) <= s.hints.len())
                .or(open.first()),
        };
        let Some(s) = pick else {
            let text = if self.hints.is_empty() {
                "This story has no hints."
            } else {
                "There is no hint for this place now."
            };
            return Self::note_turn(text.into());
        };
        let levels = s.hints.len() + 1;
        let level = (self.revealed.get(&s.key).copied().unwrap_or(0) + 1).min(levels);
        let text = match s.hints.get(level - 1) {
            Some(h) => format!("Hint {level} of {levels}: {h}"),
            None => format!("Hint {level} of {levels}: type \"{}\".", s.command),
        };
        let key = s.key.clone();
        self.revealed.insert(key, level);
        Self::note_turn(text)
    }

    /// The typed word `hint`: not while a rule waits or a talk is open, and
    /// not when the story has a verb with that word.
    pub(super) fn is_hint_word(&self, word: &str) -> bool {
        word == HINT
            && self.game.awaiting().is_none()
            && self.game.talk().is_none()
            && !self.parser_knows(HINT)
    }

    fn open_steps(&self) -> impl Iterator<Item = &HintStep> {
        let state = self.game.state();
        self.hints.steps.iter().filter(move |s| {
            !s.hints.is_empty()
                && s.room == state.room
                && !state.steps_done.contains(&s.key)
                && s.needs.iter().all(|n| state.steps_done.contains(n))
        })
    }

    /// Before an input: the steps whose command it gives in their room, and
    /// the steps whose facts do not hold yet. [`Session::mark_steps`] takes
    /// them after the input.
    pub(super) fn step_candidates(&self, input: &Input) -> (Vec<usize>, Vec<usize>) {
        if self.hints.steps.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let state = self.game.state();
        let open = || {
            self.hints
                .steps
                .iter()
                .enumerate()
                .filter(|(_, s)| !state.steps_done.contains(&s.key))
        };
        let pending: Vec<usize> = open()
            // An event step is done only by its event (`event_steps`).
            .filter(|(_, s)| s.event.is_none())
            .filter(|(_, s)| !s.facts.is_empty() && !facts_hold(state, &s.facts))
            .map(|(i, _)| i)
            .collect();
        let given: Option<Command> =
            if state.ended || self.game.awaiting().is_some() || self.game.talk().is_some() {
                None
            } else {
                match input {
                    Input::Text(text) => self.game.parse_text(text),
                    Input::Command(c) => Some(c.clone()),
                    _ => None,
                }
            };
        let matched = match given {
            Some(given) => open()
                .filter(|(_, s)| s.room == state.room)
                .filter(|(_, s)| self.game.parse_text(&s.command).as_ref() == Some(&given))
                .map(|(i, _)| i)
                .collect(),
            None => Vec::new(),
        };
        (matched, pending)
    }

    /// The steps not done yet that report the host event `name` with `args`.
    pub(super) fn event_steps(&self, name: &str, args: &BTreeMap<String, Value>) -> Vec<usize> {
        let state = self.game.state();
        (self.hints.steps.iter().enumerate())
            .filter(|(_, s)| s.event.as_ref().is_some_and(|e| e.as_str() == name))
            .filter(|(_, s)| s.args == *args)
            .filter(|(_, s)| !state.steps_done.contains(&s.key))
            .map(|(i, _)| i)
            .collect()
    }

    /// After an input: a step is done when the player gave its command and
    /// its facts hold now, or when its facts came to hold.
    pub(super) fn mark_steps(&mut self, (matched, pending): (Vec<usize>, Vec<usize>)) {
        if matched.is_empty() && pending.is_empty() {
            return;
        }
        let book = self.hints.clone();
        let state = self.game.state();
        let done: BTreeSet<String> = matched
            .iter()
            .chain(&pending)
            .map(|&i| &book.steps[i])
            .filter(|s| facts_hold(state, &s.facts))
            .map(|s| s.key.clone())
            .collect();
        if !done.is_empty() {
            self.game.state_mut().steps_done.extend(done);
        }
    }
}
