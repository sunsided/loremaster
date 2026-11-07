//! The Session API (ADR 0003): one `Session` per running game. A host sends
//! input down and gets effects back, as data (SE-1 to SE-5).
//!
//! Every front end is a host of this API: the web player, a game engine, the
//! terminal, the walkthrough runner and the AI tools. The effects carry no
//! HTML, CSS or pixel positions; the host decides how to show them.

use crate::output::{Block, Choice, LogEntry, TalkInfo};
use crate::{Command, Game, Restored, Save, SaveError, State, Turn, World};
use loremaster_model::{EndingKind, Id};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input from the host (SE-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Input {
    /// A command as the player typed it, for example `open the drawer`.
    Text(String),
    /// A command record, for example from a verb bar or a game engine.
    Command(Command),
    /// The answer with this index in the current choices, from 0.
    Choice(usize),
    /// Time passes without an action of the player.
    Tick,
}

/// One thing the session shows or tells the host (SE-4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Effect {
    /// A paragraph: its kind, and its runs with sources and links.
    Text { block: Block },
    /// A change of state, for example `set lamp.lit = true`.
    Notice { entry: LogEntry },
    /// A talk is open: who the player talks to, and the answers to choose from.
    Choices {
        talk: TalkInfo,
        choices: Vec<Choice>,
    },
    /// The story has ended.
    Ending { ending: EndingInfo },
}

/// An ending from `endings/`, as the host needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndingInfo {
    pub id: Id,
    pub kind: EndingKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    pub title: String,
}

/// What the session waits for after a step (SE-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// Ready for the next input.
    Ready,
    /// The story has ended. `ending` is `None` for a story without endings.
    Ended { ending: Option<EndingInfo> },
}

/// The result of one call: the effects in order, then the status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub effects: Vec<Effect>,
    pub status: Status,
}

impl Step {
    /// The text of the step, paragraphs separated by blank lines.
    pub fn plain_text(&self) -> String {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Text { block } => Some(block.plain_text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// What a restore had to change (ADR 0001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loaded {
    /// The save was made with another version of the story.
    pub story_changed: bool,
    /// The save version before migration.
    pub from_version: u32,
    /// What the changed story no longer has, for example `fact:lamp.lit`.
    pub dropped: Vec<String>,
}

/// A running game, as the hosts see it.
pub struct Session {
    game: Game,
}

impl Session {
    /// A new game of `world`, with the story's own seed.
    pub fn new(world: Arc<World>) -> Self {
        Self {
            game: Game::new(world),
        }
    }

    /// A new game with another seed for chance, shuffled and sticky texts.
    /// The same world, seed and input give the same effects (SE-10).
    pub fn with_seed(world: Arc<World>, seed: u64) -> Self {
        let mut game = Game::new(world);
        game.state_mut().rng = seed;
        Self { game }
    }

    /// Restore a save game (ADR 0001). The report says what the restore had
    /// to change for a changed story.
    pub fn load(world: Arc<World>, save: serde_json::Value) -> Result<(Self, Loaded), SaveError> {
        let Restored {
            game,
            story_changed,
            from_version,
            dropped,
        } = Game::load_save(world, save)?;
        let loaded = Loaded {
            story_changed,
            from_version,
            dropped,
        };
        Ok((Self { game }, loaded))
    }

    /// The opening: the intro and the first room.
    pub fn start(&mut self) -> Step {
        let turn = self.game.start();
        self.step(turn)
    }

    /// Handle one input (SE-2).
    pub fn submit(&mut self, input: Input) -> Step {
        let turn = match input {
            Input::Text(text) => self.game.input(&text),
            Input::Command(cmd) => self.game.act(cmd),
            Input::Choice(index) => self.game.act(Command::Choose { index }),
            Input::Tick => self.game.act(Command::Verb {
                verb: Id::new("wait"),
                objects: Vec::new(),
            }),
        };
        self.step(turn)
    }

    /// Choose an answer: a short form of `submit(Input::Choice(index))`.
    pub fn choose(&mut self, index: usize) -> Step {
        self.submit(Input::Choice(index))
    }

    /// The game as a save game (ADR 0001).
    pub fn save(&self) -> Save {
        self.game.save()
    }

    /// The state, read-only.
    pub fn state(&self) -> &State {
        self.game.state()
    }

    pub fn world(&self) -> &World {
        self.game.world()
    }

    /// The engine behind the session. Hosts use the session; this is for the
    /// authoring views until they move to the Workbench API (P2-08).
    #[doc(hidden)]
    pub fn game(&self) -> &Game {
        &self.game
    }

    #[doc(hidden)]
    pub fn game_mut(&mut self) -> &mut Game {
        &mut self.game
    }

    fn step(&self, turn: Turn) -> Step {
        let mut effects: Vec<Effect> = turn
            .blocks
            .into_iter()
            .map(|block| Effect::Text { block })
            .collect();
        // The rule lines of the log are for the authoring views, not for hosts.
        effects.extend(
            turn.log
                .into_iter()
                .filter(|e| e.kind != "rule")
                .map(|entry| Effect::Notice { entry }),
        );
        if let Some(talk) = turn.talk
            && !turn.choices.is_empty()
        {
            effects.push(Effect::Choices {
                talk,
                choices: turn.choices,
            });
        }
        let status = if self.game.state().ended {
            let ending = self.ending();
            if let Some(ending) = &ending
                && turn.ended
            {
                effects.push(Effect::Ending {
                    ending: ending.clone(),
                });
            }
            Status::Ended { ending }
        } else {
            Status::Ready
        };
        Step { effects, status }
    }

    fn ending(&self) -> Option<EndingInfo> {
        let id = self.game.state().ending.as_ref()?;
        let ending = self.game.world().story.ending(id.as_str())?;
        Some(EndingInfo {
            id: ending.id.clone(),
            kind: ending.kind,
            rank: ending.rank,
            title: ending.title.clone(),
        })
    }
}
