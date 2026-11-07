//! Puzzle machines: named states and the steps between them.

use crate::Effects;
use crate::convert::IntoStates;
use crate::pack::{register, replaceable, with_notes};
use loremaster_model::{Effect, EventPattern, Id, Line, Machine, StateDef, Step};
use loremaster_project::Pack;

/// A machine of a pack, from [`BuildPack::machine`](crate::BuildPack::machine).
///
/// [`step`](Self::step) adds a step; [`when`](Self::when),
/// [`say`](Self::say), [`instead`](Self::instead) and the [`Effects`] then
/// change the step added last. `assemble` makes the machine's variable and
/// one rule for each step, as for a YAML machine.
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("hades");
/// p.machine("exorcism", "idle")
///     .state_described("idle", "The spirits block the gate.")
///     .state("rung")
///     .step("idle", on::verb(["ring", "bell"]), "rung")
///     .when("here == entrance_to_hades")
///     .say("The bell suddenly becomes red hot.")
///     .start_timer("exorcism_window")
///     .step(["rung"], on::timer("exorcism_window"), "idle");
/// assert_eq!(p.machines[0].steps.len(), 2);
/// ```
pub struct MachineBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> MachineBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, start: String) -> Self {
        register(pack, format!("machine:{id}"));
        pack.machines.push(Machine {
            id,
            start,
            states: Vec::new(),
            steps: Vec::new(),
            notes: Vec::new(),
        });
        let i = pack.machines.len() - 1;
        MachineBuilder { pack, i }
    }

    fn machine(&mut self) -> &mut Machine {
        &mut self.pack.machines[self.i]
    }

    #[track_caller]
    fn last_step(&mut self) -> &mut Step {
        match self.machine().steps.last_mut() {
            Some(step) => step,
            None => panic!("the machine has no step yet: call `step` first"),
        }
    }

    /// A state by name. Without states, the machine's states are its start
    /// state and every state that a step names.
    pub fn state(mut self, name: impl Into<String>) -> Self {
        self.machine().states.push(StateDef::Name(name.into()));
        self
    }

    /// A state with author text for the Atlas.
    pub fn state_described(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.machine().states.push(StateDef::Described {
            name: name.into(),
            description: Some(description.into()),
        });
        self
    }

    /// A step: in one of the `from` states, on the event, go to `to`.
    pub fn step(mut self, from: impl IntoStates, on: EventPattern, to: impl Into<String>) -> Self {
        self.machine().steps.push(Step {
            from: from.into_states(),
            on,
            when: None,
            to: to.into(),
            say: None,
            then: Vec::new(),
            instead: false,
        });
        self
    }

    /// `when:` of the last step.
    #[track_caller]
    pub fn when(mut self, condition: impl Into<String>) -> Self {
        self.last_step().when = Some(condition.into());
        self
    }

    /// `say:` of the last step: its text, before its effects. This is the
    /// step's own field; [`Effects::effect`] with [`Effect::Say`] adds a text
    /// among the effects.
    #[track_caller]
    pub fn say(mut self, text: impl Into<Line>) -> Self {
        self.last_step().say = Some(text.into());
        self
    }

    /// `instead: true` of the last step.
    #[track_caller]
    pub fn instead(mut self) -> Self {
        self.last_step().instead = true;
        self
    }
}

impl Effects for MachineBuilder<'_> {
    /// The effects of the last step.
    #[track_caller]
    fn effects_mut(&mut self) -> &mut Vec<Effect> {
        &mut self.last_step().then
    }
}

replaceable!(MachineBuilder, machines, "machine");
with_notes!(MachineBuilder, machines);
