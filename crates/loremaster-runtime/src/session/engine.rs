//! The engine side of a session, for the authoring primitives in
//! [`crate::authoring`]. Hosts do not see it.

use super::{Input, Session};
use crate::{Command, Game, Turn, World};
use loremaster_model::Value;
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

    /// This game in a changed world (SE-14), and what was dropped. The seed,
    /// the undo depth and whether the host answers rolls and walks stay. The
    /// undo history does not: its states belong to the old world. The core of
    /// [`Session::reload`]. This path does not refuse another interface: the
    /// session is then no longer bound to the host it was made for.
    pub(crate) fn carried_over(&self, world: Arc<World>) -> (Self, Vec<String>) {
        let (mut game, dropped) = self.game.carry_over(world);
        game.host_rolls = self.game.host_rolls;
        game.host_walks = self.game.host_walks;
        let mut next = Self::of(game, self.seed);
        next.undo_depth = self.undo_depth;
        next.started = self.started;
        next.hints = self.hints.clone();
        next.revealed = self.revealed.clone();
        next.built_for = (self.built_for.clone()).filter(|b| next.check_interface(b).is_ok());
        (next, dropped)
    }

    pub(crate) fn engine(&self) -> &Game {
        &self.game
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

    /// An author command as a whole turn. `undo` can go back to the state
    /// before it, as after an input.
    pub(crate) fn author_turn(&mut self, command: crate::authoring::AuthorCommand) -> Turn {
        let before = self.before();
        // An author command can set the facts of a step: only facts count, no command.
        let candidates = self.step_candidates(&Input::Tick);
        let turn = self.game.author(command);
        self.mark_steps(candidates);
        self.remember(before);
        turn
    }

    /// One input as a whole turn, with the rule log. Typed text that is a
    /// session word (`undo`, or an answer of the death prompt) runs in the
    /// session, as in [`Session::submit`]. `undo` can go back to the state
    /// before the input.
    pub(crate) fn turn(&mut self, input: Input) -> Turn {
        if let Some(turn) = self.session_word(&input) {
            return turn;
        }
        let before = self.before();
        let candidates = self.step_candidates(&input);
        let turn = match input {
            Input::Text(text) => self.game.input(&text),
            Input::Command(cmd) => self.game.act(cmd),
            Input::Choice(index) => self.game.act(Command::Choose { index }),
            Input::Tick => self.game.tick(),
        };
        self.mark_steps(candidates);
        self.remember(before);
        turn
    }

    /// The answer to a paused rule or the death prompt as a whole turn, with
    /// the rule log. `undo` can go back to the state before it.
    pub(crate) fn resume_turn(&mut self, await_id: &str, value: Value) -> Result<Turn, String> {
        if let Some(prompt) = self.death_prompt()
            && prompt.id == await_id
        {
            prompt.request.accepts(&value)?;
            return match value {
                Value::Text(answer) => self.death_turn(&answer),
                _ => Err("the death prompt takes a text answer".into()),
            };
        }
        let before = self.before();
        // The rest of the rule can set the facts of a step.
        let candidates = self.step_candidates(&Input::Tick);
        let turn = self.game.resume(await_id, value)?;
        self.mark_steps(candidates);
        self.remember(before);
        Ok(turn)
    }

    /// Back to the state before the last input, as a turn: a note and the
    /// room as it is now.
    pub(crate) fn undo_turn(&mut self) -> Turn {
        match self.history.pop_back() {
            Some((state, revealed)) => {
                self.game.set_state(state);
                if let Some(revealed) = revealed {
                    self.revealed = revealed;
                }
                self.here_turn("Undone.")
            }
            None => Self::note_turn("There is nothing to undo.".into()),
        }
    }

    /// The story from its start again, as the opening turn. `undo` can go
    /// back to the state before it.
    pub(crate) fn restart_turn(&mut self) -> Turn {
        let before = self.before();
        let host_rolls = self.game.host_rolls;
        let mut game = Game::new(self.game.world_arc());
        game.state_mut().rng = self.seed;
        game.host_rolls = host_rolls;
        game.host_walks = self.game.host_walks;
        game.forced = std::mem::take(&mut self.game.forced);
        self.game = game;
        self.started = true;
        // A new game: the hint ladders start from their first level again,
        // until an undo goes back before the restart.
        let revealed = std::mem::take(&mut self.revealed);
        let turn = self.game.start();
        self.remember_with(before, Some(revealed));
        turn
    }
}
