//! The game as dramaturge's narrative state.
//!
//! dramaturge evaluates conditions and applies outcomes through
//! [`NarrativeState`]. This implementation reads and writes the save state
//! ([`State`](crate::State)) directly. There is no second store.
//!
//! The mapping:
//!
//! - World facts are `State::facts`. Other fact scopes are not used.
//! - The relations of the player to a thing are `State::relations`, keyed
//!   `thing.axis`. A relation on an axis of the story is 0 when no dialogue
//!   changed it. Relations that do not start at the player are not kept.
//! - The memories of a thing are `State::memories`, by memory key. A memory
//!   has the player as subject and its key as tag, as the dialogue compiler
//!   makes it.
//! - loremaster does not use alignments, dialog tags or the event log.

use super::Game;
use crate::eval::to_fact_value;
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, EntityId, EventKind, Fact, FactId, FactScope,
    FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
};
use dramaturge_runtime::NarrativeState;
use loremaster_model::{Id, Value};
use std::borrow::Cow;

impl Game {
    /// The thing of a relation, when the relation starts at the player.
    fn relation_key(&self, source: EntityId, target: EntityId, axis: &str) -> Option<String> {
        if source != self.world.scope.player() {
            return None;
        }
        let thing = self.world.symbols.entity_key(target)?;
        Some(format!("{thing}.{axis}"))
    }
}

impl NarrativeState for Game {
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>> {
        if *scope != FactScope::World {
            return None;
        }
        let value = self.state.facts.get(id.0.as_ref())?;
        Some(Cow::Owned(to_fact_value(value)))
    }

    fn set_fact(&mut self, scope: FactScope, fact: Fact) {
        if scope != FactScope::World {
            return;
        }
        let value = match fact.value {
            FactValue::Bool(b) => Value::Bool(b),
            FactValue::Int(i) => Value::Int(i),
            FactValue::Text(t) => Value::Text(t),
            // loremaster values have no floats, entities or sets.
            _ => return,
        };
        self.state.facts.insert(fact.id.0.to_string(), value);
    }

    fn remove_fact(&mut self, scope: &FactScope, id: &FactId) -> bool {
        *scope == FactScope::World && self.state.facts.remove(id.0.as_ref()).is_some()
    }

    fn alignment(&self, _: &AlignmentAxisId, _: &AlignmentPoleId) -> Option<i32> {
        None
    }

    fn add_alignment(&mut self, _: AlignmentAxisId, _: AlignmentPoleId, _: i32) {}

    fn relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32> {
        let key = self.relation_key(source, target, &axis.0)?;
        if let Some(value) = self.state.relations.get(&key) {
            return Some(*value);
        }
        // Every thing starts at 0 on the axes of the story.
        let thing = self.world.symbols.entity_key(target)?;
        let declared = self
            .world
            .story
            .relations
            .iter()
            .any(|a| a.as_str() == &*axis.0);
        (declared && self.world.thing(thing).is_some()).then_some(0)
    }

    fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    ) {
        // A delta of 0 changes nothing, so it stores nothing.
        if delta == 0 {
            return;
        }
        if let Some(key) = self.relation_key(source, target, &axis.0) {
            *self.state.relations.entry(key).or_insert(0) += delta;
        }
    }

    fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        let symbols = &self.world.symbols;
        let (Some(thing), Some(key)) = (symbols.entity_key(owner), symbols.memory_key(memory.id))
        else {
            return;
        };
        self.state
            .memories
            .entry(Id::new(thing))
            .or_default()
            .insert(key.to_string());
    }

    fn remove_memory(&mut self, owner: EntityId, id: MemoryId) -> bool {
        let symbols = &self.world.symbols;
        let (Some(thing), Some(key)) = (symbols.entity_key(owner), symbols.memory_key(id)) else {
            return false;
        };
        let memories = &mut self.state.memories;
        let Some(keys) = memories.get_mut(thing) else {
            return false;
        };
        let removed = keys.remove(key);
        // An empty set would stay in the save file.
        if keys.is_empty() {
            memories.remove(thing);
        }
        removed
    }

    /// The save keeps memories by key only, so the memory rebuilt here has an
    /// id and the player as subject, nothing more. That answers the queries
    /// that loremaster lowers (`MemoryQuery::by_id`); a query on tags,
    /// confidence or facts would never match. Keep more in the save first if
    /// a condition needs them.
    fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        let symbols = &self.world.symbols;
        let Some(keys) = symbols
            .entity_key(owner)
            .and_then(|thing| self.state.memories.get(thing))
        else {
            return false;
        };
        let player = self.world.scope.player();
        keys.iter().any(|key| {
            symbols.memory(key).is_some_and(|id| {
                let memory = Memory::new(id).with_subject(player);
                query.matches(&memory)
            })
        })
    }

    fn has_tag_available(&self, _: &DialogTagKind) -> bool {
        false
    }

    fn emit_event(&mut self, _: EventKind) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;
    use assert2::check;
    use std::sync::Arc;

    fn cantina() -> Game {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
        let files = loremaster_project::read_dir(&root).unwrap();
        let project = loremaster_project::load(files).unwrap();
        Game::new(Arc::new(World::compile(project.story).unwrap()))
    }

    #[test]
    fn world_facts_are_the_save_state_facts() {
        let mut game = cantina();
        let id = FactId::new("asked");
        NarrativeState::set_fact(&mut game, FactScope::World, Fact::bool("asked", true));
        check!(game.state.facts.get("asked") == Some(&Value::Bool(true)));
        check!(game.fact(&FactScope::World, &id).as_deref() == Some(&FactValue::Bool(true)));
        check!(game.fact(&FactScope::Party, &id).is_none());
        check!(game.remove_fact(&FactScope::World, &id));
        check!(!game.state.facts.contains_key("asked"));
    }

    #[test]
    fn relations_on_story_axes_start_at_zero() {
        let mut game = cantina();
        let player = game.world.scope.player();
        let blackbeard = game.world.scope.entity("blackbeard").unwrap();
        let trust = RelationshipAxisId::new("trust");
        check!(game.relationship(player, blackbeard, &trust) == Some(0));
        check!(game.relationship(player, blackbeard, &RelationshipAxisId::new("fear")) == None);
        check!(game.relationship(blackbeard, player, &trust) == None);
        game.add_relationship(player, blackbeard, trust.clone(), 2);
        check!(game.relationship(player, blackbeard, &trust) == Some(2));
        check!(game.state.relations.get("blackbeard.trust") == Some(&2));
    }

    #[test]
    fn a_zero_relation_delta_stores_nothing() {
        let mut game = cantina();
        let player = game.world.scope.player();
        let blackbeard = game.world.scope.entity("blackbeard").unwrap();
        let trust = RelationshipAxisId::new("trust");
        game.add_relationship(player, blackbeard, trust.clone(), 0);
        check!(!game.state.relations.contains_key("blackbeard.trust"));
        check!(game.relationship(player, blackbeard, &trust) == Some(0));
        game.add_relationship(player, blackbeard, trust.clone(), 3);
        game.add_relationship(player, blackbeard, trust.clone(), 0);
        check!(game.state.relations.get("blackbeard.trust") == Some(&3));
    }

    #[test]
    fn memories_are_the_save_state_memories() {
        let mut game = cantina();
        let player = game.world.scope.player();
        let blackbeard = game.world.scope.entity("blackbeard").unwrap();
        let id = game.world.symbols.memory("asked_marigold").unwrap();
        let query = MemoryQuery::by_id(id);
        check!(!game.matches_memory_query(blackbeard, &query));
        game.add_memory(blackbeard, Memory::new(id).with_subject(player));
        check!(game.matches_memory_query(blackbeard, &query));
        check!(game.state.memories["blackbeard"].contains("asked_marigold"));
        check!(game.remove_memory(blackbeard, id));
        check!(!game.matches_memory_query(blackbeard, &query));
        check!(!game.state.memories.contains_key("blackbeard"));
        check!(!game.remove_memory(blackbeard, id));
    }
}
