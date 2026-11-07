//! Timers: counters that go down each turn.

use crate::Effects;
use crate::pack::{register, replaceable, with_notes};
use loremaster_model::{Effect, Id, Timer};
use loremaster_project::Pack;
use std::collections::BTreeMap;

/// A timer of a pack, from [`BuildPack::timer`](crate::BuildPack::timer).
///
/// The [`Effects`] go to the value that [`at`](Self::at) chose last, and to
/// `0` (the timer runs out) before the first `at`.
pub struct TimerBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
    at: u32,
}

impl<'a> TimerBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, turns: u32) -> Self {
        register(pack, format!("timer:{id}"));
        pack.timers.push(Timer {
            id,
            turns,
            running: false,
            runs_while: None,
            at: BTreeMap::new(),
            repeat: false,
            notes: Vec::new(),
        });
        let i = pack.timers.len() - 1;
        TimerBuilder { pack, i, at: 0 }
    }

    fn timer(&mut self) -> &mut Timer {
        &mut self.pack.timers[self.i]
    }

    /// `running: true`: the timer runs from the start of the game.
    pub fn running(mut self) -> Self {
        self.timer().running = true;
        self
    }

    /// `runs_while:`: it counts down only while this holds.
    pub fn runs_while(mut self, condition: impl Into<String>) -> Self {
        self.timer().runs_while = Some(condition.into());
        self
    }

    /// `repeat: true`: start again when it runs out.
    pub fn repeat(mut self) -> Self {
        self.timer().repeat = true;
        self
    }

    /// The following effects run when the timer reaches `value`.
    pub fn at(mut self, value: u32) -> Self {
        self.at = value;
        self
    }
}

impl Effects for TimerBuilder<'_> {
    fn effects_mut(&mut self) -> &mut Vec<Effect> {
        let at = self.at;
        self.timer().at.entry(at).or_default()
    }
}

replaceable!(TimerBuilder, timers, "timer");
with_notes!(TimerBuilder, timers);
