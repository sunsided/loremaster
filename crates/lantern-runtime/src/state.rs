use lantern_model::{Id, Location, Value};
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
}

impl State {
    pub fn location_of(&self, thing: &str) -> Option<&Location> {
        self.locations.get(thing)
    }

    pub fn is_carried(&self, thing: &str) -> bool {
        matches!(self.locations.get(thing), Some(Location::Carried))
    }

    pub fn is_in_room(&self, thing: &str, room: &str) -> bool {
        matches!(self.locations.get(thing), Some(Location::Room(r)) if r.as_str() == room)
    }

    pub fn visits_to(&self, room: &str) -> u32 {
        self.visits.get(room).copied().unwrap_or(0)
    }

    pub fn flag(&self, name: &str) -> bool {
        matches!(self.facts.get(name), Some(Value::Bool(true)))
    }
}
