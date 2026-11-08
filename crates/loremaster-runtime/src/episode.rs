//! Episodes (TX-12): what one story of a series hands to the next. The story
//! lists in `export:` what its episode carries, and the next story lists in
//! `import:` what it takes. Facts, relations and memories, as in the save.

use crate::save::story_id;
use crate::{Game, SaveError};
use loremaster_lang::Ty;
use loremaster_model::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The `format` of a loremaster episode.
pub const EPISODE_FORMAT: &str = "loremaster-episode";

/// The newest episode version.
pub const EPISODE_VERSION: u32 = 1;

/// The values that a story exports, with the story they come from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub format: String,
    pub version: u32,
    /// The id of the story that wrote the episode, as in a save.
    pub story: String,
    /// Story variables and thing properties, by name (`gold`, `lamp.lit`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub facts: BTreeMap<String, Value>,
    /// Relations, keyed `thing.axis`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relations: BTreeMap<String, i32>,
    /// The memories that things keep, as `thing.key`.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub memories: BTreeSet<String>,
}

/// What a new game took from an episode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Imported {
    /// The story that wrote the episode.
    pub from: String,
    /// What the game took, for example `fact:gold`, `relation:keeper.trust`
    /// or `memory:keeper.asked`.
    pub applied: Vec<String>,
    /// What the episode has and the story does not list in `import`.
    pub skipped: Vec<String>,
    /// What the story imports, but with a value that does not fit its type,
    /// for example a number for a true/false variable.
    pub misfits: Vec<String>,
}

impl Game {
    /// The episode of the game now: the values that `export` lists. A
    /// relation that no dialogue changed, and a memory that the thing does not
    /// keep, are not in it.
    pub(crate) fn episode(&self) -> Episode {
        let story = &self.world().story;
        let state = self.state();
        let export = &story.export;
        Episode {
            format: EPISODE_FORMAT.into(),
            version: EPISODE_VERSION,
            story: story_id(&story.title),
            facts: export
                .facts
                .iter()
                .filter_map(|k| Some((k.clone(), state.facts.get(k)?.clone())))
                .collect(),
            relations: export
                .relations
                .iter()
                .filter_map(|k| Some((k.clone(), *state.relations.get(k)?)))
                .collect(),
            memories: export
                .memories
                .iter()
                .filter(|m| {
                    m.split_once('.').is_some_and(|(thing, key)| {
                        state
                            .memories
                            .get(thing)
                            .is_some_and(|keys| keys.contains(key))
                    })
                })
                .cloned()
                .collect(),
        }
    }

    /// Take from an episode, as JSON, what the story lists in `import`.
    /// Refuses data of another format or a newer version.
    pub(crate) fn import_episode(
        &mut self,
        episode: serde_json::Value,
    ) -> Result<Imported, SaveError> {
        let format = episode.get("format").and_then(|f| f.as_str()).unwrap_or("");
        if format != EPISODE_FORMAT {
            return Err(SaveError::UnknownFormat(format.into()));
        }
        let version = episode.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version > u64::from(EPISODE_VERSION) {
            return Err(SaveError::NewerVersion {
                save: u32::try_from(version).unwrap_or(u32::MAX),
                supported: EPISODE_VERSION,
            });
        }
        let episode: Episode =
            serde_json::from_value(episode).map_err(|e| SaveError::Invalid(e.to_string()))?;
        let world = self.world_arc();
        let import = &world.story.import;
        let mut out = Imported {
            from: episode.story,
            ..Imported::default()
        };
        let mut taken = Vec::new();
        for (key, value) in episode.facts {
            let name = format!("fact:{key}");
            if !import.facts.contains(&key) {
                out.skipped.push(name);
            } else if !fits(
                world.scope.vars.get(&key).or_else(|| {
                    let (thing, prop) = key.split_once('.')?;
                    world.scope.things.get(thing)?.get(prop)
                }),
                &value,
            ) {
                out.misfits.push(name);
            } else {
                self.state_mut().facts.insert(key.clone(), value);
                taken.push(key);
                out.applied.push(name);
            }
        }
        // Clamp after all facts are in: a limit can name an imported variable.
        for key in taken {
            let value = self.clamped(&key, self.state().facts[&key].clone());
            self.state_mut().facts.insert(key, value);
        }
        for (key, value) in episode.relations {
            if import.relations.contains(&key) {
                self.state_mut().relations.insert(key.clone(), value);
                out.applied.push(format!("relation:{key}"));
            } else {
                out.skipped.push(format!("relation:{key}"));
            }
        }
        for memory in episode.memories {
            match memory.split_once('.') {
                Some((thing, key)) if import.memories.contains(&memory) => {
                    self.state_mut()
                        .memories
                        .entry(thing.into())
                        .or_default()
                        .insert(key.into());
                    out.applied.push(format!("memory:{memory}"));
                }
                _ => out.skipped.push(format!("memory:{memory}")),
            }
        }
        Ok(out)
    }
}

/// Does the value fit the type of the fact?
fn fits(ty: Option<&Ty>, value: &Value) -> bool {
    match (ty, value) {
        (Some(Ty::Bool), Value::Bool(_))
        | (Some(Ty::Int), Value::Int(_))
        | (Some(Ty::Text), Value::Text(_)) => true,
        (Some(Ty::Enum(vals)), Value::Text(t)) => vals.contains(t),
        _ => false,
    }
}
