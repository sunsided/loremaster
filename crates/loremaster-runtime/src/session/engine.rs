//! The engine side of a session, for the authoring primitives in
//! [`crate::authoring`]. Hosts do not see it.

use super::{Input, Session};
use crate::{Command, Game, Turn};

impl Session {
    /// A session around a game, for example a game that carried over to a
    /// changed world. `started`: the opening already ran.
    pub(crate) fn from_engine(game: Game, started: bool) -> Self {
        Self { game, started }
    }

    pub(crate) fn engine(&self) -> &Game {
        &self.game
    }

    pub(crate) fn engine_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    pub(crate) fn started(&self) -> bool {
        self.started
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

    /// One input as a whole turn, with the rule log.
    pub(crate) fn turn(&mut self, input: Input) -> Turn {
        match input {
            Input::Text(text) => self.game.input(&text),
            Input::Command(cmd) => self.game.act(cmd),
            Input::Choice(index) => self.game.act(Command::Choose { index }),
            Input::Tick => self.game.tick(),
        }
    }
}
