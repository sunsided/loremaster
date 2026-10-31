use dramaturge_model::{EntityId, SymbolTable};
use loremaster_model::{Story, Value, VarType};
use std::collections::{BTreeMap, BTreeSet};

/// The type of a readable value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Text,
    Enum(Vec<String>),
    /// A room id, or `carried` / `nowhere` for thing locations.
    Room,
}

impl Ty {
    pub fn of_value(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Ty::Bool,
            Value::Int(_) => Ty::Int,
            Value::Text(_) => Ty::Text,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Ty::Bool => "a flag".into(),
            Ty::Int => "a number".into(),
            Ty::Text => "text".into(),
            Ty::Enum(values) => format!("one of {}", values.join(", ")),
            Ty::Room => "a room".into(),
        }
    }
}

/// Every name a condition can refer to, with its type.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub vars: BTreeMap<String, Ty>,
    pub rooms: BTreeSet<String>,
    /// Thing id to its properties' types.
    pub things: BTreeMap<String, BTreeMap<String, Ty>>,
    pub phases: Vec<String>,
    /// Relation axes, such as `trust`: `thing.trust` reads the player's relation to a thing.
    pub relations: BTreeSet<String>,
    /// Memory keys that some dialogue gives a thing, for `remembers(thing, key)`.
    pub memories: BTreeSet<String>,
    /// Timer ids, for `timer.id`.
    pub timers: BTreeSet<String>,
    /// dramaturge ids for the player, the things and the memories. Built in
    /// story order, so the same story always gets the same numbers.
    pub symbols: SymbolTable,
}

/// The entity key of the player. Things use their id as the key; an id can
/// not start with `@`, so the two never meet.
pub const PLAYER: &str = "@player";

impl Scope {
    pub fn from_story(story: &Story) -> Self {
        let vars = story
            .vars
            .iter()
            .map(|v| {
                let ty = match &v.ty {
                    VarType::Bool => Ty::Bool,
                    VarType::Int => Ty::Int,
                    VarType::Enum(values) => Ty::Enum(values.clone()),
                };
                (v.id.to_string(), ty)
            })
            .collect();
        let rooms = story.rooms.iter().map(|r| r.id.to_string()).collect();
        let things = story
            .things
            .iter()
            .map(|t| {
                let props = t
                    .props
                    .iter()
                    .map(|(k, v)| (k.clone(), Ty::of_value(v)))
                    .collect();
                (t.id.to_string(), props)
            })
            .collect();
        let phases = story
            .clock
            .phase_names()
            .into_iter()
            .map(String::from)
            .collect();
        let relations = story.relations.iter().map(|r| r.to_string()).collect();
        let memories: BTreeSet<String> = story
            .dialogues
            .iter()
            .flat_map(|d| {
                d.nodes.iter().flat_map(|n| {
                    n.outcomes
                        .remember
                        .iter()
                        .chain(n.options.iter().flat_map(|o| o.outcomes.remember.iter()))
                })
            })
            .cloned()
            .collect();
        let mut symbols = SymbolTable::new();
        symbols.intern_entity(PLAYER);
        for t in &story.things {
            symbols.intern_entity(t.id.as_str());
        }
        for m in &memories {
            symbols.intern_memory(m);
        }
        let timers = story.timers.iter().map(|t| t.id.to_string()).collect();
        Self {
            timers,
            vars,
            rooms,
            things,
            phases,
            relations,
            memories,
            symbols,
        }
    }

    /// The dramaturge entity of the player.
    pub fn player(&self) -> EntityId {
        self.symbols.entity(PLAYER).unwrap_or(EntityId(0))
    }

    /// The dramaturge entity of a thing.
    pub fn entity(&self, thing: &str) -> Option<EntityId> {
        self.symbols.entity(thing)
    }

    /// Every name in scope, for "did you mean" hints and editor autocomplete.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.vars
            .keys()
            .chain(self.rooms.iter())
            .chain(self.things.keys())
            .map(String::as_str)
    }

    pub fn suggest(&self, name: &str) -> Option<&str> {
        self.names()
            .map(|n| (n, edit_distance(n, name)))
            .filter(|(_, d)| *d <= 2)
            .min_by_key(|(_, d)| *d)
            .map(|(n, _)| n)
    }
}

pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}
