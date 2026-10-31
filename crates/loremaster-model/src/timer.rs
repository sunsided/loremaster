use crate::{Effect, Id, Note};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A counter that goes down by one each turn, for example a lamp battery or a
/// fuse. Its value is world state: `timer.lamp_battery` in conditions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timer {
    pub id: Id,
    /// Turns from the start value to zero.
    pub turns: u32,
    /// The timer runs from the start of the game. Without this, a `start`
    /// effect starts it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub running: bool,
    /// The timer counts down only while this condition holds, for example `lamp.lit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_while: Option<String>,
    /// Effects when the timer reaches a value. At `0` the timer runs out, and
    /// rules `on: { timer: id }` fire after these effects.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub at: BTreeMap<u32, Vec<Effect>>,
    /// Start again from `turns` when the timer runs out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub repeat: bool,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// The timers that a `start` effect starts: `start: fuse` with the timer's own
/// `turns`, or `start: { fuse: 6 }` with a number of turns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StartTimers {
    One(Id),
    Many(BTreeMap<Id, u32>),
}

impl StartTimers {
    /// Each timer with its turns, if given.
    pub fn entries(&self) -> Vec<(Id, Option<u32>)> {
        match self {
            Self::One(id) => vec![(id.clone(), None)],
            Self::Many(m) => m.iter().map(|(k, v)| (k.clone(), Some(*v))).collect(),
        }
    }
}
