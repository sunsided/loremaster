//! The Workbench API of loremaster (SE-13, ADR 0003): the authoring views
//! over a running game.
//!
//! A [`Workbench`] owns a [`Session`]. Play goes through the session, as in
//! every other host. The views read the full world and state: the Atlas and
//! the map, the rules and the puzzle machines, the names that conditions and
//! markup can use, checks of text that the author is typing, a preview of a
//! room in other values, the assembly of a room, the tooltips, the room
//! matrix and the state explorer. The views change the game only through
//! [`Workbench::author`]; they never change the world. A changed world comes
//! from files or packs, and [`Workbench::reload`] carries the running game
//! over to it (SE-14).
//!
//! ```
//! use loremaster_project::load;
//! use loremaster_runtime::World;
//! use loremaster_workbench::Workbench;
//! use std::sync::Arc;
//!
//! let files = loremaster_project::scaffold("The Shore");
//! let project = load(files).unwrap();
//! let world = Arc::new(World::compile(project.story).unwrap());
//! let mut bench = Workbench::new(world);
//! bench.session_mut().start();
//! assert!(!bench.atlas().is_empty());
//! assert_eq!(bench.try_condition("turns = 0").holds, Some(true));
//! ```

mod atlas;
mod checks;
mod inspect;
mod rules;

use loremaster_model::{Value, Walkthrough, step_chart};
use loremaster_runtime::authoring::{self, Inspector};
use loremaster_runtime::{Input, Session, State, Turn, World};
use std::sync::Arc;

pub use atlas::{AtlasRoom, MapEnd, MapLink, MapRoom, MapView};
pub use checks::{MarkupCheck, Symbol, TextError};
pub use inspect::{Inspect, PhaseView, VarView};
pub use loremaster_model::{ChartStep, StepChart};
pub use loremaster_runtime::authoring::{
    AuthorCommand, Budget, DeadEnd, Dimension, EndingCheck, EndingFound, Exploration, FragmentView,
    FrameFragment, FramePart, FrameUse, MAX_CELLS, MatrixCell, NoReturn, PartView, RoomCoverage,
    RoomMatrix, RoomParts, StaleCloses, TalkOptionView, TalkView, TextVariant, ThreadReach,
};
pub use loremaster_runtime::{
    DialogueOutline, ExitInfo, OutlineNode, OutlineOption, Scene, ThingInfo, VerbInfo,
};
pub use rules::{MachineStepView, MachineView, RuleView};

/// A running game with the authoring views.
pub struct Workbench {
    session: Session,
    /// The walkthroughs of the project, for the dependency charts.
    walkthroughs: Vec<(String, Walkthrough)>,
}

impl Workbench {
    /// A new game of `world`. Call [`Workbench::start`] or
    /// `session_mut().start()` for the opening.
    pub fn new(world: Arc<World>) -> Self {
        Self::from_session(Session::new(world))
    }

    /// The views over a session that a host already has, for example one
    /// restored from a save.
    pub fn from_session(session: Session) -> Self {
        Self {
            session,
            walkthroughs: Vec::new(),
        }
    }

    /// The walkthroughs of the project: they give the hint ladder of the
    /// session (CK-6) and the dependency charts (CK-7).
    pub fn set_walkthroughs(&mut self, walks: Vec<(String, Walkthrough)>) {
        self.session.set_hints_from(&walks);
        self.walkthroughs = walks;
    }

    /// The session, for play and the read-only queries of the Session API.
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn into_session(self) -> Session {
        self.session
    }

    pub fn world(&self) -> &World {
        self.session.world()
    }

    pub fn state(&self) -> &State {
        self.session.state()
    }

    fn ins(&self) -> Inspector<'_> {
        Inspector::of(&self.session)
    }

    // ---- play, as the editor's transcript shows it ----------------------

    /// The opening as a whole turn, with the rule log of the authoring views.
    /// A second call starts nothing again.
    pub fn start(&mut self) -> Turn {
        authoring::start(&mut self.session)
    }

    /// One input as a whole turn, with the rule log of the authoring views.
    /// Hosts that need only the effects use `session_mut().submit(input)`.
    pub fn play(&mut self, input: Input) -> Turn {
        authoring::play(&mut self.session, input)
    }

    /// The answer to a paused rule or to the death prompt, as a whole turn
    /// with the rule log. Read [`Session::resume`].
    pub fn resume(&mut self, await_id: &str, value: Value) -> Result<Turn, String> {
        authoring::resume(&mut self.session, await_id, value)
    }

    /// Go back to the state before the last input, as a whole turn: a note
    /// and the room as it is now. Read [`Session::undo`].
    pub fn undo(&mut self) -> Turn {
        authoring::undo(&mut self.session)
    }

    /// The next level of the hint ladder, as a turn with a note. It changes
    /// nothing in the game. Read [`Session::hint`].
    pub fn hint(&mut self, step: Option<&str>) -> Turn {
        authoring::hint(&mut self.session, step)
    }

    /// Start the story again, as the opening turn with the rule log. `undo`
    /// can go back to the state before it. Read [`Session::restart`].
    pub fn restart(&mut self) -> Turn {
        authoring::restart(&mut self.session)
    }

    /// Run an author command: go to a room, set a value, set the time, give a
    /// thing, or try one combination of the room matrix. It changes the state
    /// directly, and its turn shows as an author action.
    pub fn author(&mut self, command: AuthorCommand) -> Turn {
        authoring::author(&mut self.session, command)
    }

    /// Carry the running game over to a changed world, for example after an
    /// edit of the story (SE-14). The room, the values and the things carry
    /// over. A value still at its start value takes the new start value, so an
    /// edit of a start value shows at once. What the new world does not have
    /// is dropped; a room that is gone becomes the start room. The undo
    /// history of the session does not carry over.
    pub fn reload(&mut self, world: Arc<World>) {
        self.session = authoring::carry_over(&self.session, world);
        // The hint book plays the walkthroughs in its world: build it again.
        self.session.set_hints_from(&self.walkthroughs);
    }

    /// A new workbench with the game of this one in a changed world. This one
    /// does not change. Read [`Workbench::reload`].
    pub fn carried_to(&self, world: Arc<World>) -> Self {
        self.carried_to_with(world, self.walkthroughs.clone())
    }

    /// [`Workbench::carried_to`] with the walkthroughs of the changed story,
    /// so the hint book is built once, in the new world.
    pub fn carried_to_with(&self, world: Arc<World>, walks: Vec<(String, Walkthrough)>) -> Self {
        let mut next = Self::from_session(authoring::carry_over(&self.session, world));
        next.set_walkthroughs(walks);
        next
    }

    // ---- views ---------------------------------------------------------

    /// All rooms with what is in them now, for the Atlas.
    pub fn atlas(&self) -> Vec<AtlasRoom> {
        atlas::atlas(self.ins())
    }

    /// The dependency chart of each walkthrough with step ids, for the
    /// Atlas (CK-7): the steps, what each one needs, and the puzzle width.
    pub fn charts(&self) -> Vec<StepChart> {
        self.walkthroughs
            .iter()
            .filter_map(|(key, walk)| step_chart(key, walk))
            .collect()
    }

    /// All rooms and exits with the player's position and visits.
    pub fn map(&self) -> MapView {
        atlas::map(self.ins())
    }

    /// All rules with their state now.
    pub fn rules(&self) -> Vec<RuleView> {
        rules::rules(self.ins())
    }

    /// Every dialogue as an outline of nodes, options and their targets.
    pub fn dialogues(&self) -> Vec<DialogueOutline> {
        self.world().dialogue_outlines()
    }

    /// The puzzle machines with their states and steps.
    pub fn machines(&self) -> Vec<MachineView> {
        rules::machines(self.ins())
    }

    /// Every name that conditions and markup can use, with its value now.
    pub fn symbols(&self) -> Vec<Symbol> {
        checks::symbols(self.ins())
    }

    /// Check fragment text that the author is typing, and render it in the
    /// current state.
    pub fn try_markup(&self, src: &str) -> MarkupCheck {
        checks::try_markup(self.ins(), src)
    }

    /// Check fragment text and render it as it would read in other values,
    /// without changing the game. The names and values are the ones of
    /// [`RoomMatrix`]. `None` for an unknown room.
    pub fn try_markup_in(
        &self,
        room: &str,
        values: &[(String, String)],
        src: &str,
    ) -> Option<MarkupCheck> {
        let copy = self.ins().situation(room, values)?;
        Some(checks::try_markup(Inspector::of(&copy), src))
    }

    /// Check a condition that the author is typing, and tell whether it holds now.
    pub fn try_condition(&self, src: &str) -> MarkupCheck {
        checks::try_condition(self.ins(), src)
    }

    /// The room description as it would show now, without changing the game.
    pub fn describe(&self) -> Turn {
        self.ins().describe()
    }

    /// The room description in another combination of values, without
    /// changing the game. `None` for an unknown room.
    pub fn preview_situation(&self, room: &str, values: &[(String, String)]) -> Option<Turn> {
        let copy = self.ins().situation(room, values)?;
        Some(Inspector::of(&copy).describe())
    }

    /// How the description of the current room is assembled now: every
    /// fragment, whether it shows, and why not.
    pub fn assembly(&self) -> Vec<FragmentView> {
        self.ins().assembly()
    }

    /// The things and exits of the generated layers of the current room.
    pub fn room_parts(&self) -> RoomParts {
        self.ins().room_parts()
    }

    /// The action frame used for the last action.
    pub fn last_frame(&self) -> Option<&FrameUse> {
        Inspector::of(&self.session).last_frame()
    }

    /// The talk in progress with its hidden options, for the inspector.
    pub fn talk_view(&self) -> Option<TalkView> {
        self.ins().talk_view()
    }

    /// Facts about a thing, for an author tooltip.
    pub fn inspect_thing(&self, id: &str) -> Option<Inspect> {
        inspect::inspect_thing(self.ins(), id)
    }

    /// Facts about an exit of the current room, by its direction.
    pub fn inspect_exit(&self, direction: &str) -> Option<Inspect> {
        inspect::inspect_exit(self.ins(), direction)
    }

    /// All story variables, thing properties and timers with their values now.
    pub fn vars(&self) -> Vec<VarView> {
        inspect::vars(self.ins())
    }

    /// The phases of the day, in the order of their start times.
    pub fn phases(&self) -> Vec<PhaseView> {
        inspect::phases(self.ins())
    }

    /// The text of a room in every combination of the values it depends on.
    /// `None` for an unknown room.
    pub fn room_matrix(&self, room: &str) -> Option<RoomMatrix> {
        self.ins().room_matrix(room)
    }

    /// One fragment text in every combination of the values it depends on, in
    /// the current room. `None` when the text or the condition has a problem.
    pub fn text_matrix(&self, src: &str, when: Option<&str>) -> Option<RoomMatrix> {
        self.ins().text_matrix(src, when)
    }

    /// Walk the reachable states from the start of the story, and report the
    /// rooms, texts, endings and threads it reached, and the ending checks
    /// (`ending_checks`: dead ends and points of no return;
    /// [`EndingCheck::diagnostics`] gives them as diagnostics). It does not
    /// change the running game.
    pub fn explore(&self, budget: Budget) -> Exploration {
        self.ins().explore(budget)
    }
}

/// Walk the reachable states from the start of the story in `world`.
pub fn explore(world: Arc<World>, budget: Budget) -> Exploration {
    authoring::explore(world, budget)
}

/// The current name of a thing.
pub(crate) fn name_of(ins: Inspector<'_>, id: &loremaster_model::Id) -> String {
    ins.thing_name(id.as_str(), loremaster_lang::NameForm::Bare)
}

/// The room the player is in.
pub(crate) fn room_here(ins: Inspector<'_>) -> Option<&loremaster_model::Room> {
    ins.world().story.room(ins.state().room.as_str())
}
