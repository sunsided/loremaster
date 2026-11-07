use crate::{Id, Note};
use serde::{Deserialize, Serialize};

/// How a story ends: an entity in `endings/`. The `end` effect names it by id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ending {
    pub id: Id,
    pub kind: EndingKind,
    /// The order of the endings for the checks and the player: 1 is the
    /// best. Endings without a rank, such as deaths, are not ranked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    /// The name of the ending, shown above its text.
    pub title: String,
    /// The text, with inline markup.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// What an ending means for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndingKind {
    Win,
    Loss,
    Death,
    Neutral,
}

impl EndingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EndingKind::Win => "win",
            EndingKind::Loss => "loss",
            EndingKind::Death => "death",
            EndingKind::Neutral => "neutral",
        }
    }
}
