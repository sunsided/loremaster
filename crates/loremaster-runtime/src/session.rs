//! The Session API (ADR 0003): one `Session` per running game. A host sends
//! input down and gets effects back, as data (SE-1 to SE-5).
//!
//! Every front end is a host of this API: the web player, a game engine, the
//! terminal, the walkthrough runner and the AI tools. The effects carry no
//! HTML, CSS or pixel positions; the host decides how to show them.

use crate::output::{Block, BlockKind, Choice, LogEntry, TalkInfo};
use crate::state::{AwaitRequest, Awaiting};
use crate::{Command, Game, Restored, Save, SaveError, State, Turn, World};
use loremaster_model::Value;
use loremaster_model::{EndingKind, Id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

/// How many inputs `undo` can go back, until the host sets another depth.
pub const DEFAULT_UNDO_DEPTH: usize = 20;

/// The answers of the death prompt (SE-9).
pub const UNDO: &str = "undo";
pub const RESTART: &str = "restart";
pub const RESTORE: &str = "restore";

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
#[non_exhaustive]
pub struct EndingInfo {
    pub id: Id,
    pub kind: EndingKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    pub title: String,
}

/// What the session waits for after a step (SE-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Status {
    /// Ready for the next input.
    Ready,
    /// The story has ended. `ending` is `None` when the ending is unknown:
    /// a save restored into a changed story that no longer has it.
    Ended { ending: Option<EndingInfo> },
    /// A rule waits for the host (SE-5). Answer with `resume(id, value)`.
    Await { awaiting: Awaiting },
}

/// The result of one call: the effects in order, then the status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Loaded {
    /// The save was made with another version of the story.
    pub story_changed: bool,
    /// The save version before migration.
    pub from_version: u32,
    /// What the changed story no longer has, for example `fact:lamp.lit`.
    pub dropped: Vec<String>,
}

/// The state of a running game, to keep in memory and put back with
/// [`Session::restore`]. For a save file, use [`Session::save`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    state: State,
}

impl Snapshot {
    pub fn state(&self) -> &State {
        &self.state
    }
}

/// A running game, as the hosts see it.
pub struct Session {
    game: Game,
    /// The seed of a new game, for `restart`.
    seed: u64,
    /// The states before the last inputs, the newest last (SE-8).
    history: VecDeque<State>,
    undo_depth: usize,
    /// `start` ran, or the game came from a save.
    started: bool,
}

/// The kinds of log entries that are changes of state, which hosts get as
/// notices. The others (`rule`, `frame`, `event`, `author`) are for the
/// authoring views.
const NOTICE_KINDS: &[&str] = &["set", "move", "timer", "talk", "memory"];

impl Session {
    /// A new game of `world`, with the story's own seed.
    pub fn new(world: Arc<World>) -> Self {
        let seed = world.story.seed;
        Self::with_seed(world, seed)
    }

    /// A new game with another seed for chance, shuffled and sticky texts.
    /// The same world, seed and input give the same effects (SE-10).
    pub fn with_seed(world: Arc<World>, seed: u64) -> Self {
        let mut game = Game::new(world);
        game.state_mut().rng = seed;
        Self::of(game, seed)
    }

    fn of(game: Game, seed: u64) -> Self {
        Self {
            game,
            seed,
            history: VecDeque::new(),
            undo_depth: DEFAULT_UNDO_DEPTH,
            started: false,
        }
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
        let seed = game.world().story.seed;
        let mut session = Self::of(game, seed);
        session.started = true;
        Ok((session, loaded))
    }

    /// Restore a typed save game, as [`Session::save`] gives it.
    pub fn from_save(world: Arc<World>, save: &Save) -> Result<(Self, Loaded), SaveError> {
        let json = serde_json::to_value(save).map_err(|e| SaveError::Invalid(e.to_string()))?;
        Self::load(world, json)
    }

    /// The opening: the intro and the first room. A second call, or a call
    /// after a restore, starts nothing again and only reports the status.
    pub fn start(&mut self) -> Step {
        if self.started {
            return self.step(Turn::default());
        }
        self.started = true;
        let turn = self.game.start();
        self.step(turn)
    }

    /// Handle one input (SE-2). The text `undo` undoes the last input, unless
    /// the story has a verb with that word. While the death prompt waits, the
    /// text `undo` or `restart` answers it.
    pub fn submit(&mut self, input: Input) -> Step {
        if let Input::Text(text) = &input {
            let word = text.trim().to_lowercase();
            if self.death_prompt().is_some() && (word == UNDO || word == RESTART) {
                return self.answer_death(&word).unwrap_or_else(|e| self.message(e));
            }
            if word == UNDO && !self.story_has_word(UNDO) {
                return self.undo();
            }
        }
        let before = self.game.state().clone();
        let turn = match input {
            Input::Text(text) => self.game.input(&text),
            Input::Command(cmd) => self.game.act(cmd),
            Input::Choice(index) => self.game.act(Command::Choose { index }),
            Input::Tick => self.game.tick(),
        };
        self.remember(before);
        self.step(turn)
    }

    /// Answer the request of a rule that waits, and run the rest of the rule
    /// (SE-6). This is not a new turn: the clock and the timers do not move.
    ///
    /// The death prompt is answered here too: `undo` and `restart` run in the
    /// session; for `restore`, the host restores a save itself.
    pub fn resume(&mut self, await_id: &str, value: Value) -> Result<Step, String> {
        if let Some(prompt) = self.death_prompt()
            && prompt.id == await_id
        {
            prompt.request.accepts(&value)?;
            return match value {
                Value::Text(answer) => self.answer_death(&answer),
                _ => Err("the death prompt takes a text answer".into()),
            };
        }
        let before = self.game.state().clone();
        let turn = self.game.resume(await_id, value)?;
        self.remember(before);
        Ok(self.step(turn))
    }

    /// Go back to the state before the last input (SE-8).
    pub fn undo(&mut self) -> Step {
        match self.history.pop_back() {
            Some(state) => {
                self.game.set_state(state);
                self.shown_here("Undone.")
            }
            None => self.message("There is nothing to undo.".into()),
        }
    }

    /// How many inputs `undo` can go back. 0 turns undo off.
    pub fn set_undo_depth(&mut self, depth: usize) {
        self.undo_depth = depth;
        while self.history.len() > depth {
            self.history.pop_front();
        }
    }

    /// How many inputs `undo` can go back now.
    pub fn undo_steps(&self) -> usize {
        self.history.len()
    }

    /// The state now, to put back later with [`Session::restore`].
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.game.state().clone(),
        }
    }

    /// Put a snapshot back. `undo` can go back to the state before it.
    pub fn restore(&mut self, snapshot: &Snapshot) -> Step {
        let before = self.game.state().clone();
        self.game.set_state(snapshot.state.clone());
        self.remember(before);
        self.shown_here("Restored.")
    }

    /// Start the story again with the seed of this session. `undo` can go
    /// back to the state before it.
    pub fn restart(&mut self) -> Step {
        let before = self.game.state().clone();
        let host_rolls = self.game.host_rolls;
        let mut game = Game::new(self.game.world_arc());
        game.state_mut().rng = self.seed;
        game.host_rolls = host_rolls;
        self.game = game;
        let turn = self.game.start();
        self.remember(before);
        self.step(turn)
    }

    /// Let the host answer rolls (SE-7). Without it, the session answers a
    /// roll from its own RNG in the same step.
    pub fn set_host_rolls(&mut self, host_rolls: bool) {
        self.game.host_rolls = host_rolls;
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

    /// Keep the state before an input for `undo`, if the input changed it.
    fn remember(&mut self, before: State) {
        if self.undo_depth == 0 || &before == self.game.state() {
            return;
        }
        self.history.push_back(before);
        while self.history.len() > self.undo_depth {
            self.history.pop_front();
        }
    }

    fn story_has_word(&self, word: &str) -> bool {
        self.game
            .world()
            .story
            .verbs
            .iter()
            .any(|v| v.words.iter().any(|w| w.eq_ignore_ascii_case(word)))
    }

    fn answer_death(&mut self, answer: &str) -> Result<Step, String> {
        match answer {
            UNDO => Ok(self.undo()),
            RESTART => Ok(self.restart()),
            RESTORE => Err("restore a save with `Session::restore` or `Session::load`".into()),
            _ => Err(format!("`{answer}` is not an answer of the death prompt")),
        }
    }

    /// The death prompt, if the player died in a story that asks (SE-9).
    fn death_prompt(&self) -> Option<Awaiting> {
        let state = self.game.state();
        let story = &self.game.world().story;
        if !state.ended || !story.death_prompt {
            return None;
        }
        let ending = story.ending(state.ending.as_ref()?.as_str())?;
        if ending.kind != EndingKind::Death {
            return None;
        }
        let mut options = Vec::new();
        if !self.history.is_empty() {
            options.push(Id::new(UNDO));
        }
        options.push(Id::new(RESTART));
        options.push(Id::new(RESTORE));
        Some(Awaiting {
            id: format!("death@{}", state.turns),
            rule: ending.id.clone(),
            next: 0,
            request: AwaitRequest::DeathPrompt {
                ending: ending.id.clone(),
                options,
            },
            into: None,
            locals: BTreeMap::new(),
        })
    }

    /// A note and the room as it is now, without a turn.
    fn shown_here(&mut self, note: &str) -> Step {
        let mut turn = self.game.describe_here();
        turn.blocks.insert(
            0,
            Block {
                kind: BlockKind::Note,
                runs: vec![crate::output::Run::plain(note)],
            },
        );
        self.step(turn)
    }

    /// A note for the player, without a change.
    fn message(&self, text: String) -> Step {
        let turn = Turn {
            blocks: vec![Block {
                kind: BlockKind::Note,
                runs: vec![crate::output::Run::plain(text)],
            }],
            ..Turn::default()
        };
        self.step(turn)
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
                .filter(|e| NOTICE_KINDS.contains(&e.kind.as_str()))
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
        let status = if let Some(prompt) = self.death_prompt() {
            if turn.ended
                && let Some(ending) = self.ending()
            {
                effects.push(Effect::Ending { ending });
            }
            Status::Await { awaiting: prompt }
        } else if let Some(awaiting) = self.game.awaiting()
            && !self.game.state().ended
        {
            Status::Await {
                awaiting: awaiting.clone(),
            }
        } else if self.game.state().ended {
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
