use loremaster_model::{Id, Layer};
use serde::{Deserialize, Serialize};

/// What one command produced.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub blocks: Vec<Block>,
    pub ended: bool,
    /// Rules that fired and state they changed, for the debugger.
    pub log: Vec<LogEntry>,
    /// The talk in progress after this turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk: Option<TalkInfo>,
    /// The answers the player can choose now, during a talk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
    /// The dice of this turn, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolls: Vec<RollDetail>,
    /// The actors that walked in this turn, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moves: Vec<MoveDetail>,
    /// The sounds of this turn, in order. They are one-shot and not saved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sounds: Vec<MediaCue>,
    /// `on: next` steps taken in this turn, for the step limit.
    #[serde(skip)]
    pub(crate) machine_steps: u32,
    /// The note about the step limit is in this turn.
    #[serde(skip)]
    pub(crate) step_limit_noted: bool,
}

/// An asset as the host needs it: the id, the file and the text for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MediaCue {
    pub id: String,
    /// The path of the file, relative to the project folder.
    pub file: String,
    /// The alt text of an image, or the caption of a sound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

/// The active media slots now. A host reads them after each step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MediaView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illustration: Option<MediaCue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait: Option<MediaCue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music: Option<MediaCue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambience: Option<MediaCue>,
    /// The hotspots of the illustration whose thing or exit the player can
    /// see now (ME-4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<SceneRegion>,
}

/// A hotspot on the illustration. A click on it is a command on its thing,
/// or `go` through its exit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SceneRegion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thing: Option<String>,
    /// The direction of the exit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<String>,
    /// The name to show and to read out: the thing's name now, or the room
    /// behind the exit.
    pub name: String,
    /// Left, top, width and height, in percent of the image.
    pub rect: [u8; 4],
}

/// An actor's walk of an agenda step: the room it left and the room it is
/// in now. Without a route, the actor goes to the step's place at once, so
/// the rooms need not be next to each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MoveDetail {
    /// The thing id of the actor.
    pub actor: Id,
    /// Its current name, for example "Uncle Otto".
    pub name: String,
    /// The room it left. None when it was in no room (nowhere or carried).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Id>,
    /// The room it is in now.
    pub to: Id,
}

/// One roll of the dice, for a host that shows dice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RollDetail {
    /// The dice as the story writes them, for example `2d6`.
    pub dice: String,
    pub total: i64,
    /// The result of each die. Empty for a total that a walkthrough forced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<i64>,
    /// Who rolled or what the roll tests, for example `You` or `LUCK`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
    /// The value that the roll tests against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<i64>,
    /// The result of the test, for example `lucky`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// Who the player is talking to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkInfo {
    pub dialogue: String,
    /// The thing id.
    pub with: String,
    /// Its current name, for example "Blackbeard".
    pub name: String,
    /// The key of the current node.
    pub node: String,
}

/// An answer the player can choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// The number to pass to `choose`, from 0. The player sees it plus 1.
    pub index: usize,
    pub text: String,
}

impl Turn {
    pub(crate) fn push(&mut self, kind: BlockKind, runs: Vec<Run>) {
        if runs.iter().any(|r| !r.text.trim().is_empty()) {
            self.blocks.push(Block {
                kind,
                runs,
                cause: Cause::Command,
            });
        }
    }

    pub(crate) fn say(&mut self, kind: BlockKind, text: impl Into<String>) {
        self.push(kind, vec![Run::plain(text)]);
    }

    /// The blocks from `first` on came from the world, not the player:
    /// `first` is `blocks.len()` before the world phase.
    pub(crate) fn caused_by_world(&mut self, first: usize) {
        debug_assert!(first <= self.blocks.len());
        for block in self.blocks.iter_mut().skip(first) {
            block.cause = Cause::World;
        }
    }

    /// The turn as plain text, paragraphs separated by blank lines.
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::plain_text)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// A paragraph of output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    pub runs: Vec<Run>,
    /// Why the block appeared: the player's input, or the world.
    #[serde(default)]
    pub cause: Cause,
}

/// Why a block appeared. [`Source::owner`] says who wrote the text; the
/// cause says what made it show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// The input the host sent for the player: a command and its echo,
    /// response and rules, the room after a move, an answer to a talk or an
    /// await, and the session's own replies (a tick or a command refused
    /// while the story waits or has ended, an author command, `undo`,
    /// `restore`, a hint, the journal).
    #[default]
    Command,
    /// The world: the opening of the story, the turn passing after an
    /// action or a tick (`on: turn` rules, timers, time rules, agendas,
    /// orders of other actors), and the rules of a host event
    /// (`Session::host_event`).
    World,
}

impl Block {
    /// The text with its markup: an emphasized run in stars, as the story
    /// writes it.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        let mut open = false;
        for run in &self.runs {
            // Runs that a link split stay in one pair of stars.
            if run.emphasis != open {
                out.push('*');
                open = run.emphasis;
            }
            out.push_str(&run.text);
        }
        if open {
            out.push('*');
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    /// The player's own command, echoed.
    Command,
    /// A room name heading.
    Title,
    /// Assembled room or thing text.
    Prose,
    /// The objects in a room.
    Objects,
    /// The exits of a room.
    Exits,
    /// A verb's response.
    Response,
    /// Text from a rule.
    Narration,
    /// The closing text.
    End,
    /// A short parser note about an action the game did by itself, for
    /// example "(first opening the cottage door)". Show it apart from the prose.
    Note,
    /// The player's line in a talk.
    Said,
    /// The line of the character the player talks to.
    Speech,
    /// An author action from the command palette, for example "go to Lantern Room".
    /// It is not part of the game.
    Author,
}

/// A stretch of text, with where it came from and what it points at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
    /// Who speaks the run, when it is speech (a `says` segment): a thing id
    /// or `player`. The text has the quotes already; a front end can style
    /// the run by its speaker. Speech in a rule without `by` has no speaker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// The story marks the run as emphasis (`*Kestrel*`). The text has no
    /// stars; a front end shows it in italics.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub emphasis: bool,
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: None,
            link: None,
            speaker: None,
            emphasis: false,
        }
    }
}

/// Split the runs of each block at `*emphasis*`: the text between a pair
/// of stars on one line, with no space just inside them, becomes a run with
/// `emphasis`, without the stars. Other stars stay text, and so does `**`.
pub(crate) fn split_emphasis(blocks: &mut [Block]) {
    for block in blocks {
        if matches!(block.kind, BlockKind::Command | BlockKind::Author) {
            continue;
        }
        let runs = std::mem::take(&mut block.runs);
        for run in runs {
            emphasis_of(run, &mut block.runs);
        }
    }
}

fn emphasis_of(run: Run, out: &mut Vec<Run>) {
    // A star next to another star (`**`) is no delimiter, so two spans
    // never touch and `plain_text` can put every star back.
    let bytes = run.text.as_bytes();
    let stars: Vec<usize> = (run.text.match_indices('*').map(|(i, _)| i))
        .filter(|&i| bytes.get(i + 1) != Some(&b'*') && (i == 0 || bytes[i - 1] != b'*'))
        .collect();
    // As in Markdown, `2 * 3 * 4` is no emphasis: the stars touch the word.
    let pair = stars
        .windows(2)
        .map(|w| (w[0], w[1]))
        .find(|&(open, close)| {
            let inner = &run.text[open + 1..close];
            !inner.is_empty() && inner.trim() == inner && !inner.contains('\n')
        });
    let Some((open, close)) = pair.filter(|_| !run.emphasis) else {
        out.push(run);
        return;
    };
    let piece = |text: &str, emphasis: bool| Run {
        text: text.to_string(),
        emphasis,
        ..run.clone()
    };
    if open > 0 {
        out.push(piece(&run.text[..open], false));
    }
    out.push(piece(&run.text[open + 1..close], true));
    let rest = &run.text[close + 1..];
    if !rest.is_empty() {
        emphasis_of(piece(rest, false), out);
    }
}

/// The fragment that produced a run: answers "why did this appear?".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// `room:shore`, `thing:lamp`, `rule:dusk_falls`, `story:intro`.
    pub owner: String,
    pub fragment: usize,
    pub layer: Layer,
    /// The condition that held, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Which variant showed, for cycling and random fragments.
    pub variant: usize,
}

/// Something the player can click in the SCUMM-style front end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub kind: LinkKind,
    /// A thing id, or an exit direction.
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Thing,
    Exit,
}

/// One line of the turn log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub kind: String,
    pub text: String,
}

/// A quest in the player's journal (HG-3, HG-5): a machine with `quest: true`
/// whose current state has journal text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// The machine id.
    pub quest: String,
    pub title: String,
    /// The current state.
    pub state: String,
    /// The journal text of the state, rendered.
    pub text: String,
    /// The quest is finished in this state.
    pub done: bool,
}
