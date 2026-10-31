use crate::{Id, Note, StartTimers, TimeOfDay, Value};
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
    /// The thing, room or region that the rule belongs to. A rule in a thing or
    /// room file belongs to that thing or room. A thing rule fires for verbs on
    /// that thing, and for other events while the thing is present. A room or
    /// region rule fires while the player is there. More specific rules run first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<Id>,
    /// Order in one scope: a higher priority runs first. Equal rules run in file order.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub priority: i32,
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
    /// These rules run after the default action, or instead of it with `instead`.
    Verb(Vec<Id>),
    /// Before a verb runs, with the same pattern as `verb`. A rule with `stop`
    /// refuses the action: the default action and the `verb` rules do not run.
    /// `[go]` and `[go, north]` check a move.
    Check(Vec<Id>),
    /// A timer runs out.
    Timer(Id),
    /// An `emit` effect sends this event.
    Event(Id),
    /// A dialogue reaches a node: `[dialogue]` for any node, `[dialogue, node]` for one.
    Talk(Vec<Id>),
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
    /// Stop the action: no more rules in this phase, and no later phases.
    /// In a `check` rule, this refuses the action.
    Stop,
    /// Send a custom event. Rules `on: { event: name }` fire at once.
    Emit(Id),
    /// Start timers, or start them again from the beginning.
    Start(StartTimers),
    /// Stop a timer. Its rules do not fire.
    Cancel(Id),
}

fn is_zero(n: &i32) -> bool {
    *n == 0
}
