//! Walkthrough tests.

use crate::convert::{IntoValue, id};
use crate::pack::here;
use loremaster_model::{Expect, Pending, StepPlan, WalkStep, Walkthrough};
use loremaster_project::{Origin, Pack};

/// A walkthrough of a pack, from [`BuildPack::test`](crate::BuildPack::test).
///
/// [`step`](Self::step) adds a command. [`sees`](Self::sees),
/// [`not_sees`](Self::not_sees), [`fact`](Self::fact),
/// [`in_room`](Self::in_room), [`ended`](Self::ended) and
/// [`ending`](Self::ending) check the step added last, as
/// `{ do: …, see: …, facts: … }` in YAML. [`id`](Self::id),
/// [`at`](Self::at), [`needs`](Self::needs), [`before`](Self::before) and
/// [`hint`](Self::hint) place the step added last in the solution, for the
/// hint ladder and the dependency chart (CK-4). The `expect_` methods check
/// the game after the last step (`expect:` in YAML).
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("tests");
/// p.test("Find the key")
///     .step("n")
///     .step("open drawer")
///     .sees("Under a fold of oilcloth lies an iron key.")
///     .expect_fact("drawer_searched", true)
///     .expect_room("cottage");
/// assert_eq!(p.walkthroughs[0].1.steps.len(), 2);
/// ```
pub struct WalkthroughBuilder<'a> {
    pack: &'a mut Pack,
    i: usize,
}

impl<'a> WalkthroughBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, name: String) -> Self {
        // A YAML test has its path as key; a Rust test has its file, line
        // and column. A second test from the same place (a loop) gets `#2`,
        // and so on, so that each key names one test.
        let place = match here(pack) {
            Origin::Rust {
                file, line, column, ..
            } => format!("{file}:{line}:{column}"),
            other => other.file().to_string(),
        };
        let mut key = place.clone();
        let mut n = 1;
        while pack.walkthroughs.iter().any(|(k, _)| *k == key) {
            n += 1;
            key = format!("{place}#{n}");
        }
        pack.walkthroughs.push((
            key,
            Walkthrough {
                name: Some(name),
                steps: Vec::new(),
                expect: Expect::default(),
                pending: None,
            },
        ));
        let i = pack.walkthroughs.len() - 1;
        WalkthroughBuilder { pack, i }
    }

    fn walkthrough(&mut self) -> &mut Walkthrough {
        &mut self.pack.walkthroughs[self.i].1
    }

    fn expect(&mut self) -> &mut Expect {
        &mut self.walkthrough().expect
    }

    /// The step added last, as a step with checks and a plan.
    #[track_caller]
    fn checked(&mut self) -> (&mut Expect, &mut StepPlan) {
        let Some(step) = self.walkthrough().steps.last_mut() else {
            panic!("the walkthrough has no step yet: call `step` first");
        };
        if let WalkStep::Command(command) = step {
            *step = WalkStep::Checked {
                command: std::mem::take(command),
                expect: Expect::default(),
                plan: Box::default(),
            };
        }
        match step {
            WalkStep::Checked { expect, plan, .. } => (expect, &mut **plan),
            WalkStep::Command(_) => unreachable!("the step was made a checked step"),
        }
    }

    #[track_caller]
    fn step_expect(&mut self) -> &mut Expect {
        self.checked().0
    }

    #[track_caller]
    fn step_plan(&mut self) -> &mut StepPlan {
        self.checked().1
    }

    /// The last step has this id, for `needs`, `before` and the chart.
    #[track_caller]
    pub fn id(mut self, step: impl AsRef<str>) -> Self {
        self.step_plan().id = Some(id(step));
        self
    }

    /// The player does the last step in this room.
    #[track_caller]
    pub fn at(mut self, room: impl AsRef<str>) -> Self {
        self.step_plan().at = Some(id(room));
        self
    }

    /// The last step needs these steps first.
    #[track_caller]
    pub fn needs<S: AsRef<str>>(mut self, steps: impl IntoIterator<Item = S>) -> Self {
        let steps: Vec<_> = steps.into_iter().map(id).collect();
        self.step_plan().needs.extend(steps);
        self
    }

    /// These later steps need the last step.
    #[track_caller]
    pub fn before<S: AsRef<str>>(mut self, steps: impl IntoIterator<Item = S>) -> Self {
        let steps: Vec<_> = steps.into_iter().map(id).collect();
        self.step_plan().before.extend(steps);
        self
    }

    /// One more hint for the last step, from a nudge to almost the command.
    #[track_caller]
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.step_plan().hints.push(text.into());
        self
    }

    /// A command, as the player types it.
    pub fn step(mut self, command: impl Into<String>) -> Self {
        self.walkthrough()
            .steps
            .push(WalkStep::Command(command.into()));
        self
    }

    /// The response of the last step must show this text.
    #[track_caller]
    pub fn sees(mut self, text: impl Into<String>) -> Self {
        self.step_expect().see.push(text.into());
        self
    }

    /// The response of the last step must not show this text.
    #[track_caller]
    pub fn not_sees(mut self, text: impl Into<String>) -> Self {
        self.step_expect().not_see.push(text.into());
        self
    }

    /// After the last step, a variable or property has this value.
    #[track_caller]
    pub fn fact(mut self, name: impl Into<String>, value: impl IntoValue) -> Self {
        self.step_expect()
            .facts
            .insert(name.into(), value.into_value());
        self
    }

    /// After the last step, the player is in this room.
    #[track_caller]
    pub fn in_room(mut self, room: impl AsRef<str>) -> Self {
        self.step_expect().room = Some(id(room));
        self
    }

    /// After the last step, the game has ended (or not).
    #[track_caller]
    pub fn ended(mut self, ended: bool) -> Self {
        self.step_expect().ended = Some(ended);
        self
    }

    /// After the last step, the game has ended with this ending.
    #[track_caller]
    pub fn ending(mut self, ending: impl AsRef<str>) -> Self {
        self.step_expect().ending = Some(id(ending));
        self
    }

    /// At the end, the transcript must show this text.
    pub fn expect_see(mut self, text: impl Into<String>) -> Self {
        self.expect().see.push(text.into());
        self
    }

    /// At the end, the transcript must not show this text.
    pub fn expect_not_see(mut self, text: impl Into<String>) -> Self {
        self.expect().not_see.push(text.into());
        self
    }

    /// At the end, a variable or property has this value.
    pub fn expect_fact(mut self, name: impl Into<String>, value: impl IntoValue) -> Self {
        self.expect().facts.insert(name.into(), value.into_value());
        self
    }

    /// At the end, the player is in this room.
    pub fn expect_room(mut self, room: impl AsRef<str>) -> Self {
        self.expect().room = Some(id(room));
        self
    }

    /// At the end, the game has ended (or not).
    pub fn expect_ended(mut self, ended: bool) -> Self {
        self.expect().ended = Some(ended);
        self
    }

    /// At the end, the game has ended with this ending.
    pub fn expect_ending(mut self, ending: impl AsRef<str>) -> Self {
        self.expect().ending = Some(id(ending));
        self
    }

    /// The walkthrough cannot pass yet: the roadmap phase and what it needs.
    pub fn pending(mut self, phase: u8, needs: impl Into<String>) -> Self {
        self.walkthrough().pending = Some(Pending {
            phase,
            needs: needs.into(),
        });
        self
    }
}
