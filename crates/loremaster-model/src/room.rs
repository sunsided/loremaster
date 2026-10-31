use crate::{Fragment, Frames, Id, Note, Rule};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A room: a node in the world graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub id: Id,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<Id>,
    /// Free tags. `dark` means the room needs a lit light source to be seen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Vec<Fragment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<Exit>,
    /// Map position, owned by the editor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 2]>,
    /// The level of the room, for towers and dungeons. Without it, the level comes from
    /// the `up` and `down` exits, counted from the start room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    /// Author text around the responses to verbs in this room.
    #[serde(default, skip_serializing_if = "Frames::is_empty")]
    pub frames: Frames,
    /// Rules that belong to this room. The loader moves them to the story's
    /// rules and sets their `within`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

impl Room {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// A direction of travel. Compass and vertical directions have short forms;
/// anything else is a named exit such as "through the mirror".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Up,
    Down,
    In,
    Out,
    Named(String),
}

impl Direction {
    const COMPASS: [(Direction, &'static str, &'static str); 12] = [
        (Direction::North, "north", "n"),
        (Direction::NorthEast, "northeast", "ne"),
        (Direction::East, "east", "e"),
        (Direction::SouthEast, "southeast", "se"),
        (Direction::South, "south", "s"),
        (Direction::SouthWest, "southwest", "sw"),
        (Direction::West, "west", "w"),
        (Direction::NorthWest, "northwest", "nw"),
        (Direction::Up, "up", "u"),
        (Direction::Down, "down", "d"),
        (Direction::In, "in", "in"),
        (Direction::Out, "out", "out"),
    ];

    /// Parse a word the player typed; only recognises the built-in directions.
    pub fn from_word(word: &str) -> Option<Self> {
        Self::COMPASS
            .iter()
            .find(|(_, long, short)| *long == word || *short == word)
            .map(|(d, _, _)| d.clone())
    }

    pub fn name(&self) -> &str {
        match self {
            Direction::Named(n) => n,
            other => Self::COMPASS
                .iter()
                .find(|(d, _, _)| d == other)
                .map(|(_, long, _)| *long)
                .unwrap_or_default(),
        }
    }

    pub fn opposite(&self) -> Option<Self> {
        use Direction::*;
        Some(match self {
            North => South,
            NorthEast => SouthWest,
            East => West,
            SouthEast => NorthWest,
            South => North,
            SouthWest => NorthEast,
            West => East,
            NorthWest => SouthEast,
            Up => Down,
            Down => Up,
            In => Out,
            Out => In,
            Named(_) => return None,
        })
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<String> for Direction {
    fn from(value: String) -> Self {
        Self::from_word(&value.to_lowercase()).unwrap_or(Direction::Named(value))
    }
}

impl From<Direction> for String {
    fn from(value: Direction) -> Self {
        value.name().to_string()
    }
}

/// An exit: a first-class edge from one room to another.
///
/// An exit without `to` is a message exit: it leads nowhere, and a move in its
/// direction only prints `blocked`, for example "The rocks drop sheer into the surf."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exit {
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Id>,
    /// The exit is only listed and usable while this holds (hidden until discovered).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<String>,
    /// A door or similar thing; the exit is passable only while it is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<Id>,
    /// Extra condition for passing. Fails with `blocked` text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
    /// Printed when the player goes through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traversal: Option<String>,
}

impl Exit {
    /// True for an exit that leads nowhere and only prints its `blocked` text.
    pub fn is_message(&self) -> bool {
        self.to.is_none()
    }
}
