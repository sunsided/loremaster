//! The engine side of a session, for the authoring primitives in
//! [`crate::authoring`]. Hosts do not see it.

use super::{Input, Session};
use crate::{Command, Game, Turn, World};
use std::sync::Arc;

impl Session {
    /// A session around a copy of a game, for example one combination of the
    /// room matrix. `start` does not run again.
    pub(crate) fn detached(game: Game) -> Self {
        let seed = game.world().story.seed;
        let mut session = Self::of(game, seed);
        session.started = true;
        session
    }

    /// This game in a changed world (SE-14). The seed, the undo depth and
    /// whether the host answers rolls stay. The undo history does not: its
    /// states belong to the old world.
    pub(crate) fn carried_over(&self, world: Arc<World>) -> Self {
        let mut game = self.game.carry_over(world);
        game.host_rolls = self.game.host_rolls;
        let mut next = Self::of(game, self.seed);
        next.undo_depth = self.undo_depth;
        next.started = self.started;
        next
    }

    pub(crate) fn engine(&self) -> &Game {
        &self.game
    }

    pub(crate) fn engine_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    /// The opening as a whole turn, with the rule log. A second call starts
    /// nothing again.
    pub(crate) fn start_turn(&mut self) -> Turn {
        if self.started {
            return Turn::default();
        }
        self.started = true;
        self.game.start()
    }

    /// One input as a whole turn, with the rule log. `undo` can go back to
    /// the state before it.
    pub(crate) fn turn(&mut self, input: Input) -> Turn {
        let before = self.before();
        let turn = match input {
            Input::Text(text) => self.game.input(&text),
            Input::Command(cmd) => self.game.act(cmd),
            Input::Choice(index) => self.game.act(Command::Choose { index }),
            Input::Tick => self.game.tick(),
        };
        self.remember(before);
        turn
    }
}
