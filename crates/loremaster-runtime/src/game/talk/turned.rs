//! The game as dramaturge's state during one turn of a talk.
//!
//! dramaturge applies the outcomes of a node or an option to a
//! [`NarrativeState`]. [`Turned`] passes each read and change to the game
//! (see `narrative.rs`) and logs each change in the turn. [`WorldEffects`]
//! runs the world effects of `then`, which dramaturge hands over as a host
//! outcome.

use super::Then;
use crate::game::Game;
use crate::{LogEntry, Turn};
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, EntityId, EventKind, Fact, FactId, FactScope,
    FactValue, Memory, MemoryId, MemoryQuery, RelationshipAxisId,
};
use dramaturge_runtime::{CustomHandler, HostHandler, NarrativeState};
use loremaster_model::{Id, Value};
use std::borrow::Cow;

/// Answers derived values (`time.phase`, `here`, ...) in dialogue conditions.
#[derive(Clone, Copy)]
pub(super) struct DerivedValues;

impl CustomHandler<Game> for DerivedValues {
    fn condition(&self, key: &str, args: &[FactValue], game: &Game) -> bool {
        game.eval().custom(key, args)
    }
}

impl CustomHandler<Turned<'_>> for DerivedValues {
    fn condition(&self, key: &str, args: &[FactValue], state: &Turned<'_>) -> bool {
        state.game.eval().custom(key, args)
    }
}

/// The game during one turn of a talk: dramaturge applies outcomes to it.
/// Each read goes to the game. Each change goes to the game too, and the turn
/// log gets a line with the new value, in the order of the changes. A change
/// that stores nothing (a 0 delta, a fact outside the world scope) gets no line.
pub(super) struct Turned<'a> {
    pub game: &'a mut Game,
    pub turn: &'a mut Turn,
}

impl Turned<'_> {
    fn log(&mut self, kind: &str, text: String) {
        self.turn.log.push(LogEntry {
            kind: kind.into(),
            text,
        });
    }
}

impl NarrativeState for Turned<'_> {
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>> {
        NarrativeState::fact(&*self.game, scope, id)
    }

    fn set_fact(&mut self, scope: FactScope, fact: Fact) {
        let stored = scope == FactScope::World;
        let line = match &fact.value {
            FactValue::Bool(b) => Some(Value::Bool(*b)),
            FactValue::Int(i) => Some(Value::Int(*i)),
            FactValue::Text(t) => Some(Value::Text(t.clone())),
            _ => None,
        }
        .map(|value| format!("{} = {value}", fact.id.0));
        NarrativeState::set_fact(&mut *self.game, scope, fact);
        if stored && let Some(line) = line {
            self.log("set", line);
        }
    }

    fn remove_fact(&mut self, scope: &FactScope, id: &FactId) -> bool {
        let removed = NarrativeState::remove_fact(&mut *self.game, scope, id);
        if removed {
            self.log("set", format!("{} removed", id.0));
        }
        removed
    }

    fn alignment(&self, axis: &AlignmentAxisId, pole: &AlignmentPoleId) -> Option<i32> {
        NarrativeState::alignment(&*self.game, axis, pole)
    }

    fn add_alignment(&mut self, axis: AlignmentAxisId, pole: AlignmentPoleId, delta: i32) {
        NarrativeState::add_alignment(&mut *self.game, axis, pole, delta);
    }

    fn relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32> {
        NarrativeState::relationship(&*self.game, source, target, axis)
    }

    fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    ) {
        NarrativeState::add_relationship(&mut *self.game, source, target, axis.clone(), delta);
        // The game keeps only relations from the player; others store nothing.
        if delta == 0 || source != self.game.world.scope.player() {
            return;
        }
        let Some(thing) = self
            .game
            .world
            .symbols
            .entity_key(target)
            .map(str::to_string)
        else {
            return;
        };
        let value = NarrativeState::relationship(&*self.game, source, target, &axis).unwrap_or(0);
        self.log(
            "relation",
            format!("{thing}.{} {delta:+} = {value}", axis.0),
        );
    }

    fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        let symbols = &self.game.world.symbols;
        let line = symbols
            .entity_key(owner)
            .zip(symbols.memory_key(memory.id))
            .map(|(thing, key)| format!("{thing} remembers {key}"));
        NarrativeState::add_memory(&mut *self.game, owner, memory);
        if let Some(line) = line {
            self.log("memory", line);
        }
    }

    fn remove_memory(&mut self, owner: EntityId, id: MemoryId) -> bool {
        let symbols = &self.game.world.symbols;
        let line = symbols
            .entity_key(owner)
            .zip(symbols.memory_key(id))
            .map(|(thing, key)| format!("{thing} forgets {key}"));
        let removed = NarrativeState::remove_memory(&mut *self.game, owner, id);
        if removed && let Some(line) = line {
            self.log("memory", line);
        }
        removed
    }

    fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        NarrativeState::matches_memory_query(&*self.game, owner, query)
    }

    fn has_tag_available(&self, tag: &DialogTagKind) -> bool {
        NarrativeState::has_tag_available(&*self.game, tag)
    }

    fn emit_event(&mut self, kind: EventKind) {
        NarrativeState::emit_event(&mut *self.game, kind);
    }
}

/// Applies the world effects of `then` (a host outcome), which dramaturge
/// can not. They write into the turn of the [`Turned`] state.
pub(super) struct WorldEffects {
    pub owner: Id,
}

impl CustomHandler<Turned<'_>> for WorldEffects {}

impl HostHandler<Then, Turned<'_>> for WorldEffects {
    fn host_outcome(&mut self, effects: &Then, state: &mut Turned<'_>) {
        let Turned { game, turn } = state;
        // A `stop` (or `Flow::Stop` of a called behavior) ends the
        // node's effects, as it ends a rule.
        let outer = std::mem::replace(&mut game.stopped, false);
        for effect in effects.iter() {
            game.apply(turn, &self.owner, effect, 1);
            if game.stopped || game.state.ended {
                break;
            }
        }
        game.stopped = outer;
    }
}
