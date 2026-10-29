use lantern_model::Layer;
use serde::{Deserialize, Serialize};

/// What one command produced.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    pub blocks: Vec<Block>,
    pub ended: bool,
    /// Rules that fired and state they changed, for the debugger.
    pub log: Vec<LogEntry>,
}

impl Turn {
    pub(crate) fn push(&mut self, kind: BlockKind, runs: Vec<Run>) {
        if runs.iter().any(|r| !r.text.trim().is_empty()) {
            self.blocks.push(Block { kind, runs });
        }
    }

    pub(crate) fn say(&mut self, kind: BlockKind, text: impl Into<String>) {
        self.push(kind, vec![Run::plain(text)]);
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
}

impl Block {
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
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
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: None,
            link: None,
        }
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
