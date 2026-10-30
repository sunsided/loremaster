use crate::{Id, Note, TimeOfDay, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// When / if / then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: Id,
    pub on: EventPattern,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub then: Vec<Effect>,
    /// Fire at most once per game.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub once: bool,
    /// For verb events: the rule's text replaces the default response.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub instead: bool,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// What a rule listens for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPattern {
    /// The game begins.
    Start,
    /// Every turn, after the command is handled.
    Turn,
    Enter(Id),
    Leave(Id),
    /// The clock reaches a time of day.
    Time(TimeOfDay),
    /// A verb was used, optionally on given things: `[verb, thing, other]`.
    Verb(Vec<Id>),
}

/// What a rule does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Set variables or properties: `{ bridge_collapsed: true, lamp.lit: false }`.
    Set(BTreeMap<String, Value>),
    /// Add to integer variables.
    Add(BTreeMap<String, i64>),
    /// Print text (inline markup).
    Say(String),
    /// Move things: `{ lamp: cellar }`, `carried` or `nowhere` are special targets.
    Move(BTreeMap<Id, Id>),
    /// Move the player.
    Goto(Id),
    /// End the game with a closing text.
    End(String),
}
