use crate::{AwaitSpec, CommandPattern, Id, Line, Note, StartTimers, TimeOfDay, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// When / if / then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// The machine that this rule is a step of. The loader sets it. A machine
    /// takes at most one step for one event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<Id>,
    /// The endings that this rule closes on purpose: after it fires, they can
    /// no longer be reached (a point of no return). The ending checks accept
    /// the rank drop when it matches this list. Needs a `reason`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<Id>,
    /// Why the rule closes the endings in `closes`, for the authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// What a rule listens for.
///
/// In YAML, `on:` is `start`, `turn`, or a map with one event kind. Its serde
/// form is written by hand (in `command.rs`), so that `verb` and `check`
/// take the slots of the Command record next to the verb.
#[derive(Debug, Clone, PartialEq)]
pub enum EventPattern {
    /// The game begins.
    Start,
    /// Every turn, after the command is handled.
    Turn,
    Enter(Id),
    Leave(Id),
    /// The clock reaches a time of day.
    Time(TimeOfDay),
    /// A verb was used. The list form `[verb, thing, other]` names the verb
    /// and its objects; the record form `{ verb: examine, direct: stove,
    /// actor: keeper }` names any slot of the command (D6). These rules run
    /// after the default action, or instead of it with `instead`.
    Verb(CommandPattern),
    /// Before a verb runs, with the same pattern as `verb`. A rule with `stop`
    /// refuses the action: the default action and the `verb` rules do not run.
    /// `[go]` and `[go, north]` check a move.
    Check(CommandPattern),
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
    /// Show a line: a text in inline markup, or a list of narration and
    /// speech segments. `line:` is the same key.
    #[serde(alias = "line")]
    Say(Line),
    /// Move things: `{ lamp: cellar }`, `carried` or `nowhere` are special targets.
    Move(BTreeMap<Id, Id>),
    /// Move the player.
    Goto(Id),
    /// End the game with an ending from `endings/`, by its id.
    End(Id),
    /// Stop the action: no more rules in this phase, and no later phases.
    /// In a `check` rule, this refuses the action.
    Stop,
    /// Send a custom event. Rules `on: { event: name }` fire at once.
    Emit(Id),
    /// Run a behavior here, at this place in the effects. It gets the event
    /// `Event::Call` with the id of the owner: the rule's id, or
    /// `dialogue:<id>` / `timer:<id>` in a dialogue or a timer.
    Call(Id),
    /// Stop the rule, ask the host, and go on with the answer (SE-5, SE-6).
    Await(Box<AwaitSpec>),
    /// Start timers, or start them again from the beginning.
    Start(StartTimers),
    /// Stop a timer. Its rules do not fire.
    Cancel(Id),
}

fn is_zero(n: &i32) -> bool {
    *n == 0
}
