//! Walkthrough tests.

use crate::convert::{IntoValue, id};
use crate::pack::here;
use loremaster_model::{Expect, Pending, WalkStep, Walkthrough};
use loremaster_project::{Origin, Pack};

/// A walkthrough of a pack, from [`BuildPack::test`](crate::BuildPack::test).
///
/// [`step`](Self::step) adds a command; [`sees`](Self::sees) and
/// [`not_sees`](Self::not_sees) check the response of the step added last.
/// The `expect_` methods check the game after the last step.
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
        // A YAML test has its path as key; a Rust test has its file and line.
        let key = match here(pack) {
            Origin::Rust { file, line, .. } => format!("{file}:{line}"),
            other => other.file().to_string(),
        };
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

    #[track_caller]
    fn step_expect(&mut self) -> &mut Expect {
        let Some(step) = self.walkthrough().steps.last_mut() else {
            panic!("the walkthrough has no step yet: call `step` first");
        };
        if let WalkStep::Command(command) = step {
            *step = WalkStep::Checked {
                command: std::mem::take(command),
                expect: Expect::default(),
            };
        }
        match step {
            WalkStep::Checked { expect, .. } => expect,
            WalkStep::Command(_) => unreachable!("the step was made a checked step"),
        }
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
