//! Save games: an envelope around the [`State`], with a format version and the
//! story it belongs to (ST-4), migration of older versions (ST-5), and a
//! lenient restore into a changed story (ST-6).
//!
//! Compatibility before 1.0 is best effort (decision D5, ADR 0001): a save
//! that cannot be migrated is refused with a message, never loaded half-way.

use crate::{Game, State, World};
use loremaster_model::{Id, Location};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;

/// The `format` of a loremaster save.
pub const SAVE_FORMAT: &str = "loremaster-save";

/// The newest save version. Version 0 is the bare [`State`] of loremaster
/// 0.1, before the envelope. Version 1 is the envelope (P1-04). Version 2
/// adds the state of the actors and their agendas (`state.actors`, D6).
/// Version 3 keys the text progress as dramaturge's text core does
/// (`room:shore/2`, P4-01).
pub const SAVE_VERSION: u32 = 3;

/// A save game: the state, and the story it was made with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Save {
    pub format: String,
    pub version: u32,
    pub story: StoryStamp,
    pub state: State,
}

/// The story of a save: its id and a hash of its content. A different hash
/// means that the story changed since the save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryStamp {
    pub id: String,
    pub hash: String,
    /// The timers of the story at the time of the save. A timer that runs
    /// out leaves the state, so only this list tells a new timer from one
    /// that has run out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timers: Vec<Id>,
}

impl StoryStamp {
    pub fn of(world: &World) -> Self {
        // A story is plain data with string keys (or integer keys, which JSON
        // writes as strings), so it always serializes. A fallback hash would let
        // a changed story pass as unchanged.
        let content = serde_json::to_vec(&world.story).expect("a story always serializes to JSON");
        let digest = Sha256::digest(&content);
        let hash = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
        Self {
            id: story_id(&world.story.title),
            hash,
            timers: world.story.timers.iter().map(|t| t.id.clone()).collect(),
        }
    }
}

/// The id of a story: its title in lowercase words joined by `-`.
pub(crate) fn story_id(title: &str) -> String {
    title
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

/// Why a save cannot be restored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The data is not a loremaster save.
    UnknownFormat(String),
    /// The save comes from a newer loremaster.
    NewerVersion { save: u32, supported: u32 },
    /// The save is for a different story.
    OtherStory { save: String, story: String },
    /// The save has the right format and version, but its content is broken.
    Invalid(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::UnknownFormat(found) => write!(f, "this is not a loremaster save ({found})"),
            SaveError::NewerVersion { save, supported } => write!(
                f,
                "the save has version {save}, but this loremaster reads saves up to version {supported}"
            ),
            SaveError::OtherStory { save, story } => {
                write!(f, "the save is for the story `{save}`, not for `{story}`")
            }
            SaveError::Invalid(why) => write!(f, "the save is broken: {why}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// A restored game, and what the restore had to change.
pub(crate) struct Restored {
    pub game: Game,
    /// The save was made with a different version of the story.
    pub story_changed: bool,
    /// The save version before migration.
    pub from_version: u32,
    /// What the changed story no longer has, for example `fact:lamp.lit`,
    /// `thing:key` or `timer:fuse`. The restore dropped these values.
    pub dropped: Vec<String>,
}

impl Game {
    /// The game as a save, with the story it belongs to.
    pub fn save(&self) -> Save {
        Save {
            format: SAVE_FORMAT.to_string(),
            version: SAVE_VERSION,
            story: StoryStamp::of(self.world()),
            state: self.state().clone(),
        }
    }

    /// Restore a save, as JSON. Migrates older versions and a changed story;
    /// refuses a save of another format, a newer version, or another story.
    pub fn load_save(world: Arc<World>, save: Value) -> Result<Restored, SaveError> {
        let (save, from_version) = migrate(save)?;
        let here = StoryStamp::of(&world);
        if let Some(stamp) = &save.story
            && stamp.id != here.id
        {
            return Err(SaveError::OtherStory {
                save: stamp.id.clone(),
                story: here.id,
            });
        }
        // A save without a stamp (version 0) may be from any version of the story.
        let story_changed = save.story.as_ref().is_none_or(|s| s.hash != here.hash);
        let (state, dropped) = if story_changed {
            reconcile(&world, save.state, save.story.as_ref())
        } else {
            (save.state, Vec::new())
        };
        Ok(Restored {
            game: Game::restore(world, state),
            story_changed,
            from_version,
            dropped,
        })
    }
}

/// A save after migration; version 0 has no story stamp.
struct Migrated {
    story: Option<StoryStamp>,
    state: State,
}

/// Bring a save to [`SAVE_VERSION`], one version at a time.
fn migrate(save: Value) -> Result<(Migrated, u32), SaveError> {
    let invalid = |e: serde_json::Error| SaveError::Invalid(e.to_string());
    let Some(object) = save.as_object() else {
        return Err(SaveError::UnknownFormat("not an object".into()));
    };
    let version = match object.get("format") {
        // Version 0: the bare state of loremaster 0.1.
        None if object.contains_key("room") && object.contains_key("facts") => 0,
        None => return Err(SaveError::UnknownFormat("no `format`".into())),
        Some(Value::String(f)) if f == SAVE_FORMAT => object
            .get("version")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| SaveError::Invalid("no `version`".into()))?,
        Some(other) => return Err(SaveError::UnknownFormat(format!("format {other}"))),
    };
    if version > SAVE_VERSION {
        return Err(SaveError::NewerVersion {
            save: version,
            supported: SAVE_VERSION,
        });
    }
    // One step at a time: 0 is a bare state, the steps from 1 on change the
    // state inside the envelope.
    let (story, mut state) = match version {
        0 => (None, save),
        _ => {
            let Value::Object(mut envelope) = save else {
                unreachable!("checked above")
            };
            let story: StoryStamp = serde_json::from_value(
                envelope
                    .remove("story")
                    .ok_or_else(|| SaveError::Invalid("no `story`".into()))?,
            )
            .map_err(invalid)?;
            let state = envelope
                .remove("state")
                .ok_or_else(|| SaveError::Invalid("no `state`".into()))?;
            (Some(story), state)
        }
    };
    for from in version.max(1)..SAVE_VERSION {
        match from {
            1 => v1_to_v2(&mut state)?,
            2 => v2_to_v3(&mut state)?,
            _ => unreachable!("a step for each version below SAVE_VERSION"),
        }
    }
    let state = serde_json::from_value(state).map_err(invalid)?;
    Ok((Migrated { story, state }, version))
}

/// Version 1 to 2: the state gets `actors`, the progress of the agendas.
/// A save of version 1 comes from a loremaster without agendas, so no actor
/// has done anything: each agenda starts from its first step. A step whose
/// time passed before the save is due and runs in the next turn.
fn v1_to_v2(state: &mut Value) -> Result<(), SaveError> {
    let Some(state) = state.as_object_mut() else {
        return Err(SaveError::Invalid("the state is not an object".into()));
    };
    state
        .entry("actors")
        .or_insert_with(|| Value::Object(Default::default()));
    Ok(())
}

/// Forget active actors and orders that the story no longer has. When the
/// active actor is gone, or the save is from before the story had
/// `playable`, the first playable actor takes over where the player stands,
/// with what the player carries and what it holds itself.
pub(crate) fn keep_known_actors(world: &World, state: &mut State) {
    let playable = &world.story.playable;
    state.orders.retain(|a, _| playable.contains(a));
    if state.active.as_ref().is_some_and(|a| playable.contains(a)) {
        return;
    }
    state.active = playable.first().cloned();
    let Some(first) = state.active.clone() else {
        return;
    };
    for a in playable {
        state.facts.insert(
            format!("{a}.active"),
            loremaster_model::Value::Bool(*a == first),
        );
    }
    state.orders.remove(&first);
    for at in state.locations.values_mut() {
        if at.holder() == Some(&first) {
            *at = Location::Carried;
        }
    }
    state
        .locations
        .insert(first, Location::Room(state.room.clone()));
}

/// Keep the progress of the agendas that the story still has: an actor
/// without an agenda, and a step that its agenda no longer has, are dropped.
/// Returns what it dropped, as `agenda:<actor>#<step>` (or `agenda:<actor>`).
pub(crate) fn keep_known_steps(
    world: &World,
    actors: &mut std::collections::BTreeMap<Id, crate::ActorState>,
) -> Vec<String> {
    let mut dropped = Vec::new();
    actors.retain(|actor, progress| {
        let Some(agenda) = world.story.agenda_of(actor.as_str()) else {
            dropped.push(format!("agenda:{actor}"));
            return false;
        };
        let keys: std::collections::BTreeSet<String> =
            agenda.keyed_steps().map(|(k, _)| k).collect();
        progress.steps.retain(|key, _| {
            let keep = keys.contains(key);
            if !keep {
                dropped.push(format!("agenda:{actor}#{key}"));
            }
            keep
        });
        true
    });
    dropped
}

/// Version 2 to 3: text progress before the text core was keyed
/// `room:shore#2`; the core keys it `room:shore/2`, as dramaturge does. A
/// fragment with an `id` is keyed by its id now, so its progress starts
/// over. The field `sticky` reads as `pick` through a serde alias.
fn v2_to_v3(state: &mut Value) -> Result<(), SaveError> {
    let Some(progress) = state.get_mut("progress") else {
        return Ok(());
    };
    let Some(progress) = progress.as_object_mut() else {
        return Err(SaveError::Invalid(
            "the text progress is not an object".into(),
        ));
    };
    let old = std::mem::take(progress);
    for (key, p) in old {
        let key = match key.rsplit_once('#') {
            Some((owner, i)) if !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()) => {
                format!("{owner}/{i}")
            }
            _ => key,
        };
        progress.insert(key, p);
    }
    Ok(())
}

/// Keep what is still valid in a changed story (ST-6). New values start at
/// their start value; values that the story no longer has are dropped and
/// listed.
fn reconcile(
    world: &Arc<World>,
    mut state: State,
    stamp: Option<&StoryStamp>,
) -> (State, Vec<String>) {
    let story = &world.story;
    let start = Game::new(world.clone()).state().clone();
    let mut dropped = Vec::new();

    // A paused rule of a changed story may point at other effects now.
    if let Some(open) = state.awaiting.take() {
        dropped.push(format!("await:{}", open.id));
    }
    state.combat = None;

    let old_facts = std::mem::take(&mut state.facts);
    for (key, value) in &old_facts {
        if start.facts.contains_key(key) {
            state.facts.insert(key.clone(), value.clone());
        } else {
            dropped.push(format!("fact:{key}"));
        }
    }
    for (key, value) in &start.facts {
        state
            .facts
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    let has_room = |id: &Id| story.room(id.as_str()).is_some();
    let old_locations = std::mem::take(&mut state.locations);
    for (thing, location) in old_locations {
        let Some(begin) = start.locations.get(&thing) else {
            dropped.push(format!("thing:{thing}"));
            continue;
        };
        // `in` needs a container and `on` a supporter, as in the loader.
        let valid = match &location {
            Location::Room(r) => has_room(r),
            Location::In(h) => story.thing(h.as_str()).is_some_and(|t| t.container),
            Location::On(h) => story.thing(h.as_str()).is_some_and(|t| t.supporter),
            Location::Carried | Location::Nowhere => true,
        };
        state
            .locations
            .insert(thing, if valid { location } else { begin.clone() });
    }
    for (thing, location) in &start.locations {
        state
            .locations
            .entry(thing.clone())
            .or_insert_with(|| location.clone());
    }

    if !has_room(&state.room) {
        dropped.push(format!("room:{}", state.room));
        state.room = story.start.clone();
    }
    keep_known_actors(world, &mut state);
    if state.previous.as_ref().is_some_and(|r| !has_room(r)) {
        state.previous = None;
    }
    state.visits.retain(|r, _| {
        let keep = has_room(r);
        if !keep {
            dropped.push(format!("room:{r}"));
        }
        keep
    });

    state.timers.retain(|id, _| {
        let keep = story.timer(id.as_str()).is_some();
        if !keep {
            dropped.push(format!("timer:{id}"));
        }
        keep
    });
    // A timer that the story did not have at the save starts as at the start
    // of a game. A save without a stamp (version 0) cannot tell, so it starts
    // no timer.
    if let Some(stamp) = stamp {
        for (id, turns) in &start.timers {
            if !stamp.timers.contains(id) {
                state.timers.insert(id.clone(), *turns);
            }
        }
    }

    let rules: std::collections::BTreeSet<&str> =
        story.rules.iter().map(|r| r.id.as_str()).collect();
    state.fired.retain(|id| {
        let keep = rules.contains(id.as_str());
        if !keep {
            dropped.push(format!("rule:{id}"));
        }
        keep
    });

    let talk_ok = state.talk.as_ref().is_none_or(|t| {
        story
            .dialogue(t.dialogue.as_str())
            .is_some_and(|d| d.node(&t.node).is_some())
    });
    if !talk_ok && let Some(t) = state.talk.take() {
        dropped.push(format!("dialogue:{}", t.dialogue));
    }
    // The nodes before the current one, for `go: back`: keep only the nodes
    // that the dialogue still has.
    if let Some(t) = &mut state.talk
        && let Some(d) = story.dialogue(t.dialogue.as_str())
    {
        let dialogue = t.dialogue.clone();
        t.back.retain(|node| {
            let keep = d.node(node).is_some();
            if !keep {
                dropped.push(format!("dialogue:{dialogue}#{node}"));
            }
            keep
        });
    }
    if let Some(e) = &state.ending
        && story.ending(e.as_str()).is_none()
    {
        dropped.push(format!("ending:{e}"));
        state.ending = None;
    }

    let has_thing = |id: &str| story.thing(id).is_some();
    state.relations.retain(|key, _| {
        let keep = key.split_once('.').is_some_and(|(t, _)| has_thing(t));
        if !keep {
            dropped.push(format!("relation:{key}"));
        }
        keep
    });
    state.memories.retain(|thing, _| {
        let keep = has_thing(thing.as_str());
        if !keep {
            dropped.push(format!("memories:{thing}"));
        }
        keep
    });
    dropped.extend(keep_known_steps(world, &mut state.actors));

    dropped.sort();
    dropped.dedup();
    (state, dropped)
}
