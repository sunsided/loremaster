//! The Session API (ADR 0003): one `Session` per running game. A host sends
//! input down and gets effects back, as data (SE-1 to SE-5).
//!
//! Every front end is a host of this API: the web player, a game engine, the
//! terminal, the walkthrough runner and the AI tools. The effects carry no
//! HTML, CSS or pixel positions; the host decides how to show them.

use crate::output::{Block, BlockKind, Choice, LogEntry, RollDetail, TalkInfo};
use crate::state::{AwaitRequest, Awaiting};
use crate::{Command, Game, Restored, Save, SaveError, State, Turn, World};
use loremaster_model::{EndingKind, Id, Interface, InterfaceMismatch, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

mod engine;
mod hints;
mod queries;

pub use hints::{HINT, Hint, HintBook};

/// How many inputs `undo` can go back, until the host sets another depth.
pub const DEFAULT_UNDO_DEPTH: usize = 20;

/// The answers of the death prompt (SE-9).
pub const UNDO: &str = "undo";
pub const RESTART: &str = "restart";
pub const RESTORE: &str = "restore";
/// The text command that shows the quest journal.
pub const JOURNAL: &str = "journal";

/// Input from the host (SE-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A roll of the dice: the dice, the total and any test. A `roll` notice
    /// has the same total as text.
    Roll { roll: RollDetail },
    /// Play this sound once. Show its caption when the sound is off.
    Sound { sound: crate::MediaCue },
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
    /// The id of the story, so a snapshot of another story is refused.
    story: String,
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
    /// The states before the last inputs, the newest last (SE-8). A
    /// restart also keeps the hint levels it cleared, for its undo.
    history: VecDeque<(State, Option<BTreeMap<String, usize>>)>,
    undo_depth: usize,
    /// `start` ran, or the game came from a save.
    started: bool,
    /// The hint ladder, from the walkthroughs (CK-6).
    hints: Arc<hints::HintBook>,
    /// How many levels of each step's hint ladder the player has seen.
    revealed: BTreeMap<String, usize>,
}

/// The kinds of log entries that are changes of state, which hosts get as
/// notices. The others (`rule`, `frame`, `event`, `author`) are for the
/// authoring views.
const NOTICE_KINDS: &[&str] = &["set", "move", "timer", "talk", "memory", "relation", "roll"];

impl Session {
    /// A new game of `world`, with the story's own seed.
    pub fn new(world: Arc<World>) -> Self {
        let seed = world.story.seed;
        Self::with_seed(world, seed)
    }

    /// A new game for a host built for `built_for` (IF-5). Refuses to make
    /// the session when the story has another interface.
    pub fn for_host(world: Arc<World>, built_for: &Interface) -> Result<Self, InterfaceMismatch> {
        let session = Self::new(world);
        session.check_interface(built_for)?;
        Ok(session)
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
            hints: Arc::default(),
            revealed: BTreeMap::new(),
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

    /// A new game of `world` that starts with an episode of the story
    /// before (TX-12), as JSON. It takes only what `import` in story.yaml
    /// lists; the report says what it took and what did not fit. Call
    /// [`Session::start`] next, as for a new game.
    pub fn from_episode(
        world: Arc<World>,
        episode: serde_json::Value,
    ) -> Result<(Self, crate::Imported), SaveError> {
        let mut session = Self::new(world);
        let imported = session.game.import_episode(episode)?;
        Ok((session, imported))
    }

    /// The opening: the intro and the first room. A second call, or a call
    /// after a restore, starts nothing again and only reports the status.
    pub fn start(&mut self) -> Step {
        let turn = self.start_turn();
        self.step(turn)
    }

    /// Handle one input (SE-2). The text `undo` undoes the last input, and
    /// the text `hint` shows the next hint (see [`Session::hint`]), unless
    /// the story has a verb with that word. While the death prompt waits, the
    /// text `undo` or `restart` answers it.
    pub fn submit(&mut self, input: Input) -> Step {
        let turn = self.turn(input);
        self.step(turn)
    }

    /// The turn of a session word in typed text, or `None` when the text is
    /// a command for the story. While the death prompt waits, `undo`,
    /// `restart` and `restore` answer it. Otherwise `undo` undoes, unless a
    /// rule waits or a talk is open (then the text is an answer) or the
    /// story has a verb with that word.
    fn session_word(&mut self, input: &Input) -> Option<Turn> {
        let Input::Text(text) = input else {
            return None;
        };
        let word = text.trim().to_lowercase();
        if self.death_prompt().is_some() && [UNDO, RESTART, RESTORE].contains(&word.as_str()) {
            return Some(self.death_turn(&word).unwrap_or_else(Self::note_turn));
        }
        if self.is_hint_word(&word) {
            return Some(self.hint_turn(None));
        }
        let free = self.game.awaiting().is_none() && self.game.talk().is_none();
        if word == JOURNAL && free && !self.parser_knows(JOURNAL) {
            return Some(self.journal_turn());
        }
        (word == UNDO && free && !self.parser_knows(UNDO)).then(|| self.undo_turn())
    }

    /// Answer the request of a rule that waits, and run the rest of the rule
    /// (SE-6). This is not a new turn: the clock and the timers do not move.
    ///
    /// The death prompt is answered here too: `undo` and `restart` run in the
    /// session; for `restore`, the host restores a save itself.
    pub fn resume(&mut self, await_id: &str, value: Value) -> Result<Step, String> {
        let turn = self.resume_turn(await_id, value)?;
        Ok(self.step(turn))
    }

    /// The host reports a declared event (IF-1, HG-1): check its arguments
    /// and precondition, and run the rules that listen for it. An `Err` says
    /// why it is refused, and nothing has changed.
    pub fn host_event(
        &mut self,
        name: &str,
        args: BTreeMap<String, Value>,
    ) -> Result<Step, String> {
        let before = self.before();
        let (mut matched, pending) = self.step_candidates(&Input::Tick);
        matched.extend(self.event_steps(name));
        let turn = self.game.host_event(name, &args)?;
        self.mark_steps((matched, pending));
        self.remember(before);
        Ok(self.step(turn))
    }

    /// `Err` with what differs when the host was built for another
    /// interface than the story has (IF-5).
    pub fn check_interface(&self, built_for: &Interface) -> Result<(), InterfaceMismatch> {
        let story = self.interface();
        story.compare(built_for)
    }

    /// The interface of the story; empty when it has none.
    pub fn interface(&self) -> Interface {
        self.game
            .world()
            .story
            .interface
            .clone()
            .unwrap_or_default()
    }

    /// Go back to the state before the last input (SE-8).
    pub fn undo(&mut self) -> Step {
        let turn = self.undo_turn();
        self.step(turn)
    }

    /// What the session waits for now: the same status as the last step
    /// returned.
    pub fn status(&self) -> Status {
        let state = self.game.state();
        if let Some(prompt) = self.death_prompt() {
            Status::Await { awaiting: prompt }
        } else if let Some(awaiting) = self.game.awaiting()
            && !state.ended
        {
            Status::Await {
                awaiting: awaiting.clone(),
            }
        } else if state.ended {
            Status::Ended {
                ending: self.ending(),
            }
        } else {
            Status::Ready
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
            story: crate::save::story_id(&self.game.world().story.title),
            state: self.game.state().clone(),
        }
    }

    /// Put a snapshot back. `undo` can go back to the state before it. A
    /// snapshot of another story is refused.
    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<Step, SaveError> {
        let story = crate::save::story_id(&self.game.world().story.title);
        if snapshot.story != story {
            return Err(SaveError::OtherStory {
                save: snapshot.story.clone(),
                story,
            });
        }
        let before = self.before();
        self.game.set_state(snapshot.state.clone());
        self.started = true;
        self.remember(before);
        let turn = self.here_turn("Restored.");
        Ok(self.step(turn))
    }

    /// Start the story again with the seed of this session. `undo` can go
    /// back to the state before it.
    pub fn restart(&mut self) -> Step {
        let turn = self.restart_turn();
        self.step(turn)
    }

    /// Let the host answer rolls (SE-7). Without it, the session answers a
    /// roll from its own RNG in the same step. The setting belongs to the
    /// session, not the save: set it again after `load` or `from_save`.
    pub fn set_host_rolls(&mut self, host_rolls: bool) {
        self.game.host_rolls = host_rolls;
    }

    /// The next rolls take these totals, in order, for walkthrough tests.
    /// A roll after the last one uses the RNG again.
    pub(crate) fn force_rolls(&mut self, totals: &[i64]) {
        self.game.forced.totals = totals.iter().copied().collect();
    }

    /// What went wrong with the forced rolls: totals out of the range of
    /// their dice, and totals that no roll took. Clears the forced rolls.
    pub(crate) fn forced_roll_errors(&mut self) -> Vec<String> {
        let forced = std::mem::take(&mut self.game.forced);
        let mut out = forced.misfits;
        if !forced.totals.is_empty() {
            let left: Vec<_> = forced.totals.iter().map(i64::to_string).collect();
            out.push(format!("no roll took the forced rolls {}", left.join(", ")));
        }
        out
    }

    /// Choose an answer: a short form of `submit(Input::Choice(index))`.
    pub fn choose(&mut self, index: usize) -> Step {
        self.submit(Input::Choice(index))
    }

    /// The episode for the next story (TX-12): the values that `export` in
    /// story.yaml lists. A host writes it at the end, or at any time.
    pub fn episode(&self) -> crate::Episode {
        self.game.episode()
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

    /// The state before an input, if undo is on.
    fn before(&self) -> Option<State> {
        (self.undo_depth > 0).then(|| self.game.state().clone())
    }

    /// Keep the state before an input for `undo`, if the input changed it.
    fn remember(&mut self, before: Option<State>) {
        self.remember_with(before, None);
    }

    /// [`Self::remember`], with the hint levels that `undo` brings back.
    fn remember_with(&mut self, before: Option<State>, revealed: Option<BTreeMap<String, usize>>) {
        let Some(before) = before else {
            return;
        };
        // Hints change no state: a restart from the start state still
        // has levels to bring back.
        if &before == self.game.state() && revealed.as_ref().is_none_or(BTreeMap::is_empty) {
            return;
        }
        self.history.push_back((before, revealed));
        while self.history.len() > self.undo_depth {
            self.history.pop_front();
        }
    }

    /// Does the parser know the word as a verb (the standard verbs and the
    /// story's)? Then it is a command, not a session word.
    fn parser_knows(&self, word: &str) -> bool {
        self.game
            .world()
            .verbs
            .iter()
            .any(|v| v.words.iter().any(|w| w.eq_ignore_ascii_case(word)))
    }

    fn death_turn(&mut self, answer: &str) -> Result<Turn, String> {
        match answer {
            UNDO => Ok(self.undo_turn()),
            RESTART => Ok(self.restart_turn()),
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
    fn here_turn(&mut self, note: &str) -> Turn {
        let mut turn = self.game.describe_here();
        turn.blocks.insert(
            0,
            Block {
                kind: BlockKind::Note,
                runs: vec![crate::output::Run::plain(note)],
            },
        );
        turn
    }

    /// The journal for the `journal` command: one block per quest.
    fn journal_turn(&self) -> Turn {
        let entries = self.game.journal();
        let lines = if entries.is_empty() {
            vec!["Your journal is empty.".to_string()]
        } else {
            (entries.iter())
                .map(|e| {
                    let mark = if e.done { " (done)" } else { "" };
                    format!("{}{mark}: {}", e.title, e.text)
                })
                .collect()
        };
        let blocks = (lines.into_iter())
            .map(|line| Block {
                kind: BlockKind::Response,
                runs: vec![crate::output::Run::plain(line)],
            })
            .collect();
        Turn {
            blocks,
            ..Turn::default()
        }
    }

    /// A note for the player, without a change.
    fn note_turn(text: String) -> Turn {
        Turn {
            blocks: vec![Block {
                kind: BlockKind::Note,
                runs: vec![crate::output::Run::plain(text)],
            }],
            ..Turn::default()
        }
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
        effects.extend(turn.rolls.into_iter().map(|roll| Effect::Roll { roll }));
        effects.extend(turn.sounds.into_iter().map(|sound| Effect::Sound { sound }));
        // While a rule waits, its answers come with `Status::Await`, not as
        // the choices of a talk.
        if let Some(talk) = turn.talk
            && !turn.choices.is_empty()
            && self.game.awaiting().is_none()
        {
            effects.push(Effect::Choices {
                talk,
                choices: turn.choices,
            });
        }
        // The turn that ended the story shows the ending, also when the death
        // prompt waits after it.
        if turn.ended
            && self.game.state().ended
            && let Some(ending) = self.ending()
        {
            effects.push(Effect::Ending { ending });
        }
        Step {
            effects,
            status: self.status(),
        }
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
