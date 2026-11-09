//! Primitives for the authoring views (SE-13, ADR 0003 point 8).
//!
//! The views are in the crate `loremaster-workbench`: atlas, rules,
//! machines, symbols, markup and condition checks, the map, the inspectors
//! and the variables. They read the world and the state through
//! [`Session::world`] and [`Session::state`]. Where a view needs the engine,
//! it calls a primitive here: a condition, a text in markup, a name stage,
//! the scope, the fragments of a room, or a copy of the game in another
//! situation. Views that are mostly engine work stay here whole: the
//! assembly of a room, the room matrix, the explorer and the author commands.
//!
//! This module is not part of the Session API and does not follow semver on
//! its own. Use `loremaster-workbench`.

use crate::game::Game;
use crate::{Input, Session, SourcePiece, State, Turn, World};
use dramaturge_model::Condition;
use loremaster_lang::{NameForm, Scope, Segment};
use loremaster_model::Id;
use std::sync::Arc;

pub use crate::explore::{
    Budget, DeadEnd, EndingCheck, EndingFound, Exploration, NoReturn, RoomCoverage, StaleCloses,
    TextVariant,
};
pub use crate::game::{
    AuthorCommand, FragmentView, FrameFragment, FramePart, FrameUse, PartView, RoomParts,
    TalkOptionView, TalkView,
};
pub use crate::matrix::{Dimension, MAX_CELLS, MatrixCell, RoomMatrix};
pub use crate::threads::ThreadReach;

/// Read access to the engine behind a session, for the authoring views.
/// It changes nothing.
#[derive(Clone, Copy)]
pub struct Inspector<'a> {
    game: &'a Game,
}

impl<'a> Inspector<'a> {
    pub fn of(session: &'a Session) -> Self {
        Self {
            game: session.engine(),
        }
    }

    pub fn world(&self) -> &'a World {
        self.game.world()
    }

    pub fn state(&self) -> &'a State {
        self.game.state()
    }

    /// The story's thing with this id, found by index.
    pub fn thing(&self, id: &str) -> Option<&'a loremaster_model::Thing> {
        self.game.world().thing(id)
    }

    /// The story's room with this id, found by index.
    pub fn room(&self, id: &str) -> Option<&'a loremaster_model::Room> {
        self.game.world().room(id)
    }

    /// The names that conditions and markup can use, to compile what the
    /// author types.
    pub fn scope(&self) -> &'a Scope {
        &self.game.world().scope
    }

    /// True when the condition holds now.
    pub fn holds(&self, condition: &Condition) -> bool {
        self.game.eval().holds(condition)
    }

    /// Markup rendered in the state now.
    pub fn render(&self, segments: &[Segment]) -> String {
        crate::text::render(&self.game.eval(), segments)
    }

    /// Markup in pieces, with the branches that do not hold now marked as
    /// not active.
    pub fn outline(&self, segments: &[Segment]) -> Vec<SourcePiece> {
        let mut pieces = Vec::new();
        crate::text::outline(&self.game.eval(), segments, true, &mut pieces);
        pieces
    }

    /// The current name of a thing in a form, for example "the oil lamp".
    pub fn thing_name(&self, id: &str, form: NameForm) -> String {
        self.game.eval().thing_name(id, form)
    }

    /// The index of the thing's name stage now; `None` without stages, or
    /// when no stage holds.
    pub fn name_stage(&self, id: &str) -> Option<usize> {
        self.game.eval().name_stage(id)
    }

    /// A thing as the scene shows it: its current name and aliases.
    pub fn thing_info(&self, id: &str) -> Option<crate::ThingInfo> {
        self.game.thing_info(id)
    }

    /// The nouns the player can use for a thing now. Empty for an unknown thing.
    pub fn nouns(&self, id: &str) -> Vec<String> {
        self.game
            .world()
            .story
            .thing(id)
            .map(|t| self.game.nouns_of(t))
            .unwrap_or_default()
    }

    /// The things the player can see and refer to now.
    pub fn in_scope(&self) -> Vec<Id> {
        self.game.in_scope()
    }

    /// True when the exit with this index of the room shows now. False for
    /// an unknown room or exit.
    pub fn exit_visible(&self, room: &str, exit: usize) -> bool {
        self.exit_holds(room, exit, |e| &e.visible)
    }

    /// True when the player can pass the exit now.
    pub fn exit_passable(&self, room: &str, exit: usize) -> bool {
        self.exit_holds(room, exit, |e| &e.passable)
    }

    fn exit_holds(
        &self,
        room: &str,
        exit: usize,
        condition: impl Fn(&crate::world::CompiledExit) -> &Option<Condition>,
    ) -> bool {
        let world = self.game.world();
        world
            .room_idx(room)
            .and_then(|ri| world.rooms[ri].exits.get(exit))
            .is_some_and(|e| self.game.eval().holds_opt(condition(e)))
    }

    /// True when the condition of the rule with this index (in the order of
    /// `story.rules`) holds now, or it has none.
    pub fn rule_holds(&self, index: usize) -> bool {
        self.game
            .world()
            .rules
            .get(index)
            .is_some_and(|r| self.game.eval().holds_opt(&r.when))
    }

    /// The rules that fired in the last turn.
    pub fn last_fired(&self) -> &'a [Id] {
        self.game.last_fired()
    }

    /// The action frame used for the last action.
    pub fn last_frame(&self) -> Option<&'a FrameUse> {
        self.game.last_frame()
    }

    /// How the description of the current room is assembled now.
    pub fn assembly(&self) -> Vec<FragmentView> {
        self.game.assembly()
    }

    /// The generated objects and exits layers of the current room.
    pub fn room_parts(&self) -> RoomParts {
        self.game.room_parts()
    }

    /// The talk in progress with its hidden options, or `None`.
    pub fn talk_view(&self) -> Option<TalkView> {
        self.game.talk_view()
    }

    /// The description of the current room as it would show now.
    pub fn describe(&self) -> Turn {
        self.game.describe_preview()
    }

    /// A copy of the game in one combination of the room matrix: in the room,
    /// with each named value set. `None` for an unknown room.
    pub fn situation(&self, room: &str, values: &[(String, String)]) -> Option<Session> {
        let mut copy = self.game.clone();
        crate::matrix::apply_cell(&mut copy, room, values).then(|| Session::detached(copy))
    }

    /// Walk the reachable states from the start of the story of this game.
    /// It does not change the game.
    pub fn explore(&self, budget: Budget) -> Exploration {
        explore(self.game.world_arc(), budget)
    }

    /// The text of a room in every combination of the values it depends on.
    pub fn room_matrix(&self, room: &str) -> Option<RoomMatrix> {
        crate::matrix::room_matrix(self.game, room)
    }

    /// One fragment text in every combination of the values it depends on.
    pub fn text_matrix(&self, src: &str, when: Option<&str>) -> Option<RoomMatrix> {
        crate::matrix::text_matrix(self.game, src, when)
    }
}

/// Force the totals of the next rolls of a session, as a walkthrough does.
pub fn force_rolls(session: &mut Session, totals: &[i64]) {
    session.force_rolls(totals);
}

/// The errors of the forced rolls, as a walkthrough reports them.
pub fn forced_roll_errors(session: &mut Session) -> Vec<String> {
    session.forced_roll_errors()
}

/// The opening as a whole turn, with the rule log, for an editor transcript.
pub fn start(session: &mut Session) -> Turn {
    session.start_turn()
}

/// One input as a whole turn, with the rule log, for an editor transcript.
/// Typed session words (`undo`, the death prompt) run as in [`Session::submit`].
pub fn play(session: &mut Session, input: Input) -> Turn {
    session.turn(input)
}

/// One step of an explorer path (`go north`, `say …`, `answer …`, `leave`)
/// as a whole turn, with the rule log. `None` when the game has no such step
/// now, for example after an edit of the story. Next to the turn come the
/// errors of the rolls that the walk forced for the step, as in
/// [`forced_roll_errors`]; empty when the replay took them all.
pub fn replay_step(session: &mut Session, step: &str) -> Option<(Turn, Vec<String>)> {
    session.explorer_turn(step)
}

/// The answer to a paused rule or the death prompt as a whole turn, with the
/// rule log, for an editor transcript. Read [`Session::resume`].
pub fn resume(
    session: &mut Session,
    await_id: &str,
    value: loremaster_model::Value,
) -> Result<Turn, String> {
    session.resume_turn(await_id, value)
}

/// [`Session::undo`] as a whole turn, for an editor transcript.
pub fn undo(session: &mut Session) -> Turn {
    session.undo_turn()
}

/// The next level of the hint ladder, as a turn with a note. Read
/// [`Session::hint`].
pub fn hint(session: &mut Session, step: Option<&str>) -> Turn {
    session.hint_turn(step)
}

/// [`Session::restart`] as a whole turn, with the rule log of the opening.
pub fn restart(session: &mut Session) -> Turn {
    session.restart_turn()
}

/// A session on a bare state, as it is: no migration and no check against
/// the story. `start` does not run again. For tests and tools that build a
/// state by hand; hosts restore saves with [`Session::load`].
pub fn session_of(world: Arc<World>, state: State) -> Session {
    Session::detached(Game::restore(world, state))
}

/// Run an author command: it changes the state directly. `undo` can go back
/// to the state before it.
pub fn author(session: &mut Session, command: AuthorCommand) -> Turn {
    session.author_turn(command)
}

/// The session in a changed world (SE-14). The state carries over; a value
/// still at its start value takes the new start value, and what the new world
/// does not have is dropped.
pub fn carry_over(session: &Session, world: Arc<World>) -> Session {
    session.carried_over(world).0
}

/// [`carry_over`] with the hint book built again from these walkthroughs and
/// the hint progress fitted to it, as in [`Session::reload`]. `session` does
/// not change.
pub fn carry_over_with(
    session: &Session,
    world: Arc<World>,
    walks: &[(String, loremaster_model::Walkthrough)],
) -> Session {
    session.carried_over_with(world, walks).0
}

/// A `combat` effect in words, for the rules view.
pub fn describe_combat(spec: &loremaster_model::CombatSpec) -> String {
    let enemies: Vec<&str> = spec.enemies.iter().map(|e| e.as_str()).collect();
    let mode = if spec.together {
        "all at once"
    } else {
        "one by one"
    };
    let escape = match &spec.escape {
        Some(room) => format!(", escape to {room}"),
        None => String::new(),
    };
    format!(
        "combat with {} ({mode}): win to {}, loss to {}{escape}",
        enemies.join(", "),
        spec.win,
        spec.loss
    )
}

/// What an `await` effect waits for, in words, for the rules view. `None`
/// for a spec that the checks reject.
pub fn describe_await(spec: &loremaster_model::AwaitSpec) -> Option<String> {
    crate::state::AwaitRequest::of(spec).map(|r| r.describe())
}

/// Walk the reachable states from the start of the story.
pub fn explore(world: Arc<World>, budget: Budget) -> Exploration {
    crate::explore::explore(&Game::new(world), budget)
}
