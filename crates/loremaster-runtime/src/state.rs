use loremaster_model::{Id, Location, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// How far a fragment has got through its variants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentProgress {
    pub shown: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky: Option<u32>,
}

/// Everything that changes during play. This is the save game: small, plain
/// data, keyed by ids so it survives most edits to the story.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub room: Id,
    /// Story variables and thing properties (`lamp.lit`).
    pub facts: BTreeMap<String, Value>,
    pub locations: BTreeMap<Id, Location>,
    pub visits: BTreeMap<Id, u32>,
    /// Minutes since midnight of day one.
    pub minutes: u32,
    pub turns: u32,
    /// Keyed by fragment key, e.g. `room:shore#2`.
    pub progress: BTreeMap<String, FragmentProgress>,
    pub fired: BTreeSet<Id>,
    pub rng: u64,
    pub ended: bool,
    /// The verb of the action being handled now, for `action.*` in conditions.
    #[serde(default)]
    pub action: Option<Id>,
    /// The last command, as verb and objects, to count repeats.
    #[serde(default)]
    pub last_command: Option<String>,
    /// The room the player was in before the last move, for `back`.
    #[serde(default)]
    pub previous: Option<Id>,
    /// How many times in a row the player gave the last command.
    #[serde(default)]
    pub repeat: u32,
    /// Uses of each verb, as `verb` (whole game) and `verb@room`.
    #[serde(default)]
    pub action_counts: BTreeMap<String, u32>,
    /// How many commands named each thing.
    #[serde(default)]
    pub interactions: BTreeMap<Id, u32>,
    /// How many times the player talked to each thing.
    #[serde(default)]
    pub talked: BTreeMap<Id, u32>,
    /// The conversation in progress, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub talk: Option<Talk>,
    /// The player's relation to things, keyed `thing.axis`. Only values that
    /// a dialogue changed; the rest are 0. dramaturge holds the same values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relations: BTreeMap<String, i32>,
    /// The memories that things keep, from dialogues. dramaturge holds them too.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub memories: BTreeMap<Id, BTreeSet<String>>,
    /// Running timers and the turns they have left.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub timers: BTreeMap<Id, u32>,
}

/// Where a conversation is: the dialogue and its current node, by key, so
/// that a save survives edits of the dialogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talk {
    pub dialogue: Id,
    pub node: String,
    /// The nodes before the current one, for `go: back`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub back: Vec<String>,
}

impl State {
    pub fn location_of(&self, thing: &str) -> Option<&Location> {
        self.locations.get(thing)
    }

    /// Where the thing is at the end of its containers and supporters: a room,
    /// carried or nowhere.
    pub fn outer_location(&self, thing: &str) -> Option<&Location> {
        let mut at = self.locations.get(thing)?;
        // A loop of containers is an authoring error; give up after a few steps.
        for _ in 0..16 {
            match at.holder() {
                Some(holder) => at = self.locations.get(holder.as_str())?,
                None => return Some(at),
            }
        }
        None
    }

    /// The player has the thing, in the hands or in something carried.
    pub fn is_carried(&self, thing: &str) -> bool {
        matches!(self.outer_location(thing), Some(Location::Carried))
    }

    /// The player holds the thing itself, not in a container.
    pub fn is_held(&self, thing: &str) -> bool {
        matches!(self.locations.get(thing), Some(Location::Carried))
    }

    /// The thing is in the room, also inside containers and on supporters there.
    pub fn is_in_room(&self, thing: &str, room: &str) -> bool {
        matches!(self.outer_location(thing), Some(Location::Room(r)) if r.as_str() == room)
    }

    /// The first closed container around the thing, if one hides it. A
    /// container is closed when its `open` property is false; `transparent`
    /// lets the player see in, but not reach in.
    pub fn closed_around(&self, thing: &str, seeing: bool) -> Option<&Id> {
        let mut at = self.locations.get(thing)?;
        for _ in 0..16 {
            let Location::In(holder) = at else {
                at = self.locations.get(at.holder()?.as_str())?;
                continue;
            };
            let closed = matches!(
                self.facts.get(&format!("{holder}.open")),
                Some(Value::Bool(false))
            );
            let see_through = self.flag(&format!("{holder}.transparent"));
            if closed && !(seeing && see_through) {
                return Some(holder);
            }
            at = self.locations.get(holder.as_str())?;
        }
        None
    }

    /// The things directly in or on a holder, in id order.
    pub fn contents(&self, holder: &str) -> Vec<&Id> {
        self.locations
            .iter()
            .filter(|(_, at)| at.holder().is_some_and(|h| h.as_str() == holder))
            .map(|(id, _)| id)
            .collect()
    }

    pub fn visits_to(&self, room: &str) -> u32 {
        self.visits.get(room).copied().unwrap_or(0)
    }

    pub fn flag(&self, name: &str) -> bool {
        matches!(self.facts.get(name), Some(Value::Bool(true)))
    }
}
