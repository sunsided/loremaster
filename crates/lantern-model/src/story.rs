use crate::{ClockConfig, Fragment, Frames, Id, Room, Rule, Thing, Value, VerbDef};
use serde::{Deserialize, Serialize};

/// The type of a story variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarType {
    Bool,
    Int,
    Enum(Vec<String>),
}

/// A declared story variable such as `bridge_collapsed` or `weather`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarDef {
    pub id: Id,
    #[serde(rename = "type")]
    pub ty: VarType,
    pub initial: Value,
}

/// A whole story: everything a project folder holds, and what the bundle carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub start: Id,
    /// Printed before the first room description.
    #[serde(default)]
    pub intro: Vec<Fragment>,
    #[serde(default)]
    pub clock: ClockConfig,
    #[serde(default)]
    pub vars: Vec<VarDef>,
    /// Custom verbs in addition to the standard library.
    #[serde(default)]
    pub verbs: Vec<VerbDef>,
    #[serde(default)]
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub things: Vec<Thing>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Author text around the responses to verbs, for the whole story.
    #[serde(default, skip_serializing_if = "Frames::is_empty")]
    pub frames: Frames,
    /// Seed for shuffled and sticky fragments; fixed so playthroughs replay.
    #[serde(default)]
    pub seed: u64,
}

impl Story {
    pub fn room(&self, id: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id.as_str() == id)
    }

    pub fn thing(&self, id: &str) -> Option<&Thing> {
        self.things.iter().find(|t| t.id.as_str() == id)
    }

    pub fn var(&self, id: &str) -> Option<&VarDef> {
        self.vars.iter().find(|v| v.id.as_str() == id)
    }
}
