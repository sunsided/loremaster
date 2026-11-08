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
    /// The ending that ended the game, from `endings/`. Old saves have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<Id>,
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
    /// a dialogue changed; the rest are 0. dramaturge reads and writes them
    /// here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relations: BTreeMap<String, i32>,
    /// The memories that things keep, from dialogues. dramaturge reads and
    /// writes them here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub memories: BTreeMap<Id, BTreeSet<String>>,
    /// Running timers and the turns they have left.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub timers: BTreeMap<Id, u32>,
    /// A rule that waits for the host (SE-6): where it goes on, and what it asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub awaiting: Option<Awaiting>,
    /// The walkthrough steps of the hint ladder that the player has done
    /// (CK-6), by their key. Empty without hints.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub steps_done: BTreeSet<String>,
    /// What each actor has done of its agenda (D6), by actor id. An actor
    /// whose agenda has not moved yet has no entry. The actor's place is its
    /// location in `locations`, as for any thing. New in save version 2.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub actors: BTreeMap<Id, ActorState>,
}

/// The state of an actor (D6): the steps of its agenda that are over.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorState {
    /// Each step that is over, by its key (the step's `id`, or its place in
    /// the agenda from 1): it ran, or it was skipped. The first step of the
    /// agenda that is not here is the current one.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub steps: BTreeMap<String, StepOutcome>,
}

/// How a step of an agenda ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    /// The step ran: its command and its effects.
    Ran,
    /// The step did not happen: its `when` did not hold when it was due, or a
    /// `check` rule refused its command.
    Skipped,
}

/// A paused rule, as data (XP-3): the rule, the next effect, the request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Awaiting {
    /// The id to pass to `resume`.
    pub id: String,
    pub rule: Id,
    /// The effect of the rule that runs after the answer.
    pub next: usize,
    pub request: AwaitRequest,
    /// The variable that takes the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<String>,
    /// Local values of the rule. Empty until rules get locals.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub locals: BTreeMap<String, Value>,
}

/// What a paused rule asks the host (SE-5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AwaitRequest {
    /// Choose one option.
    Choice { options: Vec<AwaitOptionInfo> },
    /// Do a custom action and report one of its results.
    Action {
        name: Id,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        args: BTreeMap<String, Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        results: Vec<Id>,
    },
    /// Roll dice and report the total.
    Roll { dice: String, min: i64, max: i64 },
    /// Walk the player to a room; answer `arrived` or `refused`.
    WalkTo { room: Id },
    /// The player died (SE-9): answer `undo`, `restart` or `restore`. The
    /// session does `undo` and `restart` itself; for `restore`, the host
    /// loads a save with `Session::restore` or `Session::load`.
    DeathPrompt { ending: Id, options: Vec<Id> },
}

/// An option of a choice: its id and the text to show.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AwaitOptionInfo {
    pub id: Id,
    pub text: String,
}

impl AwaitRequest {
    pub(crate) fn of(spec: &loremaster_model::AwaitSpec) -> Option<Self> {
        if !spec.choice.is_empty() {
            let options = spec
                .choice
                .iter()
                .map(|o| AwaitOptionInfo {
                    id: o.id().clone(),
                    text: o.text().to_string(),
                })
                .collect();
            return Some(AwaitRequest::Choice { options });
        }
        if let Some(name) = &spec.action {
            return Some(AwaitRequest::Action {
                name: name.clone(),
                args: spec.args.clone(),
                results: spec.results.clone(),
            });
        }
        if let Some(text) = &spec.roll {
            let dice = loremaster_model::Dice::parse(text)?;
            return Some(AwaitRequest::Roll {
                dice: text.clone(),
                min: dice.min(),
                max: dice.max(),
            });
        }
        spec.walk_to
            .as_ref()
            .map(|room| AwaitRequest::WalkTo { room: room.clone() })
    }

    /// The answers that a list can offer, with their texts. A roll and an
    /// action without results have none: the host answers with a value.
    pub fn answers(&self) -> Vec<(Value, String)> {
        match self {
            AwaitRequest::Choice { options } => options
                .iter()
                .map(|o| (Value::Text(o.id.to_string()), o.text.clone()))
                .collect(),
            AwaitRequest::Action { results, .. } => results
                .iter()
                .map(|r| (Value::Text(r.to_string()), r.to_string()))
                .collect(),
            AwaitRequest::WalkTo { .. } => [crate::game::ARRIVED, crate::game::REFUSED]
                .iter()
                .map(|a| (Value::Text((*a).into()), (*a).to_string()))
                .collect(),
            AwaitRequest::DeathPrompt { options, .. } => options
                .iter()
                .map(|o| (Value::Text(o.to_string()), o.to_string()))
                .collect(),
            AwaitRequest::Roll { .. } => Vec::new(),
        }
    }

    /// Does the answer fit the request?
    pub fn accepts(&self, answer: &Value) -> Result<(), String> {
        match (self, answer) {
            (AwaitRequest::Roll { min, max, .. }, Value::Int(n)) if (min..=max).contains(&n) => {
                Ok(())
            }
            (AwaitRequest::Roll { min, max, dice }, _) => {
                Err(format!("a roll of {dice} is a number from {min} to {max}"))
            }
            // Without a list of results, the host answers with any value.
            (AwaitRequest::Action { results, .. }, _) if results.is_empty() => Ok(()),
            (_, a) if self.answers().iter().any(|(v, _)| v == a) => Ok(()),
            _ => {
                let list: Vec<String> = self.answers().iter().map(|(v, _)| v.to_string()).collect();
                Err(format!(
                    "the answer `{answer}` is not one of: {}",
                    list.join(", ")
                ))
            }
        }
    }

    /// The request in words, for logs and text hosts.
    pub fn describe(&self) -> String {
        match self {
            AwaitRequest::Choice { options } => format!(
                "choose {}",
                options
                    .iter()
                    .map(|o| o.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            AwaitRequest::Action { name, results, .. } if results.is_empty() => {
                format!("the action {name}")
            }
            AwaitRequest::Action { name, results, .. } => format!(
                "the action {name} ({})",
                results
                    .iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            AwaitRequest::Roll { dice, .. } => format!("roll {dice}"),
            AwaitRequest::WalkTo { room } => format!("walk to {room}"),
            AwaitRequest::DeathPrompt { options, .. } => format!(
                "you died: {}",
                options
                    .iter()
                    .map(|o| o.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
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
    /// How a step of an actor's agenda ended, if it is over.
    pub fn step_outcome(&self, actor: &str, step: &str) -> Option<StepOutcome> {
        self.actors.get(actor)?.steps.get(step).copied()
    }

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

    /// The thing, or a holder around it, is concealed (WO-2): its
    /// `concealed` property is true. Search or a rule reveals it.
    pub fn is_concealed(&self, thing: &str) -> bool {
        let mut at = thing;
        // A loop of containers is an authoring error; give up after a few steps.
        for _ in 0..16 {
            if self.flag(&format!("{at}.concealed")) {
                return true;
            }
            match self.locations.get(at).and_then(Location::holder) {
                Some(holder) => at = holder.as_str(),
                None => return false,
            }
        }
        false
    }

    /// How many things the player holds in the hands, for the carry limit.
    /// A concealed thing does not count: the player can not name it to drop it.
    pub fn held_count(&self) -> u32 {
        self.locations
            .iter()
            .filter(|(id, l)| matches!(l, Location::Carried) && !self.is_concealed(id.as_str()))
            .count() as u32
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
