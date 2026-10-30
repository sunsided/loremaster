//! The room matrix: the text of one room in every combination of the values
//! that the text depends on, reachable or not. It finds the corners where the
//! text reads badly: no text, two replacing fragments that fight, odd spacing.

use crate::world::CFragment;
use crate::{BlockKind, Game, Turn};
use dramaturge_model::{Condition, FactValue};
use loremaster_lang::{Derived, Operand, Segment};
use loremaster_model::{Id, Location, TimeOfDay, Value, VarType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One value that the room text depends on, and the values worth trying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    /// For example `lamp.lit`, `time.phase`, `first_visit`, `blackbeard.talked`.
    pub name: String,
    pub values: Vec<String>,
}

/// The room text for one combination of values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixCell {
    /// One value for each dimension, in the order of the dimensions.
    pub values: Vec<String>,
    pub text: String,
    /// Problems found in this text.
    pub warnings: Vec<String>,
}

/// The text of a room in all combinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomMatrix {
    pub room: String,
    pub dimensions: Vec<Dimension>,
    pub cells: Vec<MatrixCell>,
    /// The number of all combinations. When it is more than the cells, the matrix shows only the first ones.
    pub combinations: usize,
}

/// The most combinations that the matrix renders.
pub const MAX_CELLS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Fact(String),
    Phase,
    Hour,
    FirstVisit,
    Visits(Id),
    Carried(Id),
    Present(Id),
    Talked(Id),
    Interactions(Id),
}

impl Key {
    fn name(&self) -> String {
        match self {
            Key::Fact(n) => n.clone(),
            Key::Phase => "time.phase".into(),
            Key::Hour => "time.hour".into(),
            Key::FirstVisit => "first_visit".into(),
            Key::Visits(r) => format!("{r}.visits"),
            Key::Carried(t) => format!("{t}.carried"),
            Key::Present(t) => format!("{t}.present"),
            Key::Talked(t) => format!("{t}.talked"),
            Key::Interactions(t) => format!("{t}.interactions"),
        }
    }
}

/// Numbers that a condition compares with, so the matrix tries both sides of each limit.
#[derive(Default)]
struct Found {
    keys: BTreeMap<Key, BTreeSet<i64>>,
    texts: BTreeMap<Key, BTreeSet<String>>,
}

impl Found {
    fn number(&mut self, key: Key, n: i64) {
        let e = self.keys.entry(key).or_default();
        e.extend([n - 1, n, n + 1].into_iter().filter(|v| *v >= 0));
    }

    fn flag(&mut self, key: Key) {
        self.keys.entry(key).or_default();
    }

    fn text(&mut self, key: Key, t: &str) {
        self.texts.entry(key).or_default().insert(t.to_string());
    }

    fn condition(&mut self, c: &Condition) {
        match c {
            Condition::All(items) | Condition::Any(items) => {
                items.iter().for_each(|c| self.condition(c))
            }
            Condition::Not(inner) => self.condition(inner),
            Condition::FactEquals { fact_id, value, .. } => {
                let key = Key::Fact(fact_id.0.to_string());
                match value {
                    FactValue::Int(n) => self.number(key, *n),
                    FactValue::Text(t) => self.text(key, t),
                    _ => self.flag(key),
                }
            }
            Condition::FactAtLeast { fact_id, value, .. }
            | Condition::FactAtMost { fact_id, value, .. } => {
                self.number(Key::Fact(fact_id.0.to_string()), *value)
            }
            Condition::FactExists { fact_id, .. } => self.flag(Key::Fact(fact_id.0.to_string())),
            Condition::Custom { key, args } => {
                if let Some((d, _, value)) = Derived::from_condition(key, args) {
                    self.derived(&d, &value);
                }
            }
            _ => {}
        }
    }

    fn derived(&mut self, d: &Derived, value: &FactValue) {
        let n = match value {
            FactValue::Int(n) => *n,
            _ => 0,
        };
        match d {
            Derived::Phase => self.flag(Key::Phase),
            Derived::Hour => self.number(Key::Hour, n),
            Derived::FirstVisit => self.flag(Key::FirstVisit),
            Derived::Visited(r) => self.number(Key::Visits(r.clone()), 1),
            Derived::Visits(r) => self.number(Key::Visits(r.clone()), n),
            Derived::Carried(t) => self.flag(Key::Carried(t.clone())),
            Derived::Present(t) => self.flag(Key::Present(t.clone())),
            Derived::Talked(t) => self.number(Key::Talked(t.clone()), n),
            Derived::Interactions(t) => self.number(Key::Interactions(t.clone()), n),
            _ => {}
        }
    }

    fn segments(&mut self, segs: &[Segment]) {
        for seg in segs {
            match seg {
                Segment::Text(_) => {}
                Segment::Insert { operand, .. } => match operand {
                    Operand::Fact(name) => self.flag(Key::Fact(name.clone())),
                    Operand::Derived(d) => self.derived(d, &FactValue::Int(0)),
                    Operand::ThingName(..) => {}
                },
                Segment::Choice {
                    condition,
                    then,
                    otherwise,
                    ..
                } => {
                    self.condition(condition);
                    self.segments(then);
                    self.segments(otherwise);
                }
            }
        }
    }

    fn fragments(&mut self, fragments: &[CFragment]) {
        for f in fragments {
            if let Some(c) = &f.when {
                self.condition(c);
            }
            f.variants.iter().for_each(|v| self.segments(v));
        }
    }
}

fn values_for(game: &Game, key: &Key, found: &Found) -> Vec<String> {
    let story = &game.world().story;
    let numbers = |extra: &[i64]| -> Vec<String> {
        let mut set: BTreeSet<i64> = found.keys.get(key).cloned().unwrap_or_default();
        set.extend(extra);
        set.into_iter().map(|n| n.to_string()).collect()
    };
    match key {
        Key::Fact(name) => {
            if let Some(var) = story.var(name) {
                return match &var.ty {
                    VarType::Bool => vec!["false".into(), "true".into()],
                    VarType::Enum(values) => values.clone(),
                    VarType::Int => numbers(&[var.initial.as_int().unwrap_or(0)]),
                };
            }
            match game.state().facts.get(name) {
                Some(Value::Bool(_)) | None => vec!["false".into(), "true".into()],
                Some(Value::Int(n)) => numbers(&[*n]),
                Some(Value::Text(t)) => {
                    let mut v: BTreeSet<String> = found.texts.get(key).cloned().unwrap_or_default();
                    v.insert(t.clone());
                    v.into_iter().collect()
                }
            }
        }
        Key::Phase => story
            .clock
            .phase_names()
            .into_iter()
            .map(String::from)
            .collect(),
        Key::Hour | Key::Visits(_) | Key::Talked(_) | Key::Interactions(_) => numbers(&[0]),
        Key::FirstVisit | Key::Carried(_) | Key::Present(_) => vec!["true".into(), "false".into()],
    }
}

fn apply(game: &mut Game, room: &Id, key: &Key, value: &str) {
    let day = game.state().minutes - game.state().minutes % TimeOfDay::MINUTES_PER_DAY;
    match key {
        Key::Fact(name) => {
            let v = match game.state().facts.get(name) {
                Some(Value::Int(_)) => Value::Int(value.parse().unwrap_or(0)),
                Some(Value::Text(_)) => Value::Text(value.to_string()),
                _ if value == "true" || value == "false" => Value::Bool(value == "true"),
                _ => Value::Text(value.to_string()),
            };
            game.set_value(name, v);
        }
        Key::Phase => {
            if let Some(p) = game
                .world()
                .story
                .clock
                .phases
                .iter()
                .find(|p| p.name == value)
            {
                let at = u32::from(p.from.0);
                game.state_mut().minutes = day + at;
            }
        }
        Key::Hour => game.state_mut().minutes = day + value.parse::<u32>().unwrap_or(0) * 60,
        Key::FirstVisit => {
            let n = if value == "true" { 1 } else { 2 };
            game.state_mut().visits.insert(room.clone(), n);
        }
        Key::Visits(r) => {
            let n: u32 = value.parse().unwrap_or(0);
            game.state_mut().visits.insert(r.clone(), n);
        }
        Key::Carried(t) => {
            if value == "true" {
                game.state_mut()
                    .locations
                    .insert(t.clone(), Location::Carried);
            } else if matches!(
                game.state().location_of(t.as_str()),
                Some(Location::Carried)
            ) {
                game.state_mut()
                    .locations
                    .insert(t.clone(), Location::Room(room.clone()));
            }
        }
        Key::Present(t) => {
            let loc = if value == "true" {
                Location::Room(room.clone())
            } else {
                Location::Nowhere
            };
            game.state_mut().locations.insert(t.clone(), loc);
        }
        Key::Talked(t) => {
            game.state_mut()
                .talked
                .insert(t.clone(), value.parse().unwrap_or(0));
        }
        Key::Interactions(t) => {
            game.state_mut()
                .interactions
                .insert(t.clone(), value.parse().unwrap_or(0));
        }
    }
}

fn warnings(game: &Game, room: &Id, prose: &str) -> Vec<String> {
    let mut out = Vec::new();
    if prose.trim().is_empty() {
        out.push("no room text".into());
    }
    if prose.contains("  ") {
        out.push("two spaces in a row".into());
    }
    if [" .", " ,", " ;", " :", " !", " ?"]
        .iter()
        .any(|p| prose.contains(p))
    {
        out.push("a space before punctuation".into());
    }
    if prose.contains("{") || prose.contains("}") {
        out.push("a brace in the text".into());
    }
    // Two replacing fragments in one layer whose conditions are both true: only the last one shows.
    if let Some(ri) = game.world().room_idx(room.as_str()) {
        let ev = game.eval();
        let mut layers: BTreeMap<loremaster_model::Layer, usize> = BTreeMap::new();
        for f in &game.world().rooms[ri].description {
            if f.replaces && ev.holds_opt(&f.when) {
                *layers.entry(f.layer).or_default() += 1;
            }
        }
        for (layer, n) in layers {
            if n > 1 {
                out.push(format!("{n} replacing fragments in the {layer:?} layer").to_lowercase());
            }
        }
        // A thing with more than one presence text at the same time usually says the same thing twice.
        let first = game.state().visits_to(room.as_str()) <= 1;
        for (ti, t) in game.world().story.things.iter().enumerate() {
            if !game.state().is_in_room(t.id.as_str(), room.as_str()) {
                continue;
            }
            let n = crate::text::visible(&ev, &game.world().things[ti].presence, first).len();
            if n > 1 {
                out.push(format!("{n} presence texts of {} show", t.id));
            }
        }
    }
    out
}

/// The text of a room in all combinations of the values that the text depends on.
pub fn room_matrix(game: &Game, room: &str) -> Option<RoomMatrix> {
    let world = game.world();
    let ri = world.room_idx(room)?;
    let room_id = world.story.rooms[ri].id.clone();
    let mut found = Found::default();
    found.fragments(&world.rooms[ri].description);
    for (ti, t) in world.story.things.iter().enumerate() {
        let here = matches!(&t.location, Location::Room(r) if *r == room_id)
            || game.state().is_in_room(t.id.as_str(), room);
        if here {
            found.fragments(&world.things[ti].presence);
            world.things[ti]
                .names
                .iter()
                .flatten()
                .for_each(|c| found.condition(c));
        }
    }
    let keys: Vec<Key> = found
        .keys
        .keys()
        .chain(found.texts.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let dimensions: Vec<Dimension> = keys
        .iter()
        .map(|k| Dimension {
            name: k.name(),
            values: values_for(game, k, &found),
        })
        .collect();
    let combinations = dimensions
        .iter()
        .map(|d| d.values.len().max(1))
        .product::<usize>();

    let mut base = game.clone();
    enter(&mut base, &room_id);
    let mut cells = Vec::new();
    for index in 0..combinations.min(MAX_CELLS) {
        let mut rest = index;
        let mut values = Vec::new();
        let mut g = base.clone();
        for (k, d) in keys.iter().zip(&dimensions) {
            let v = &d.values[rest % d.values.len()];
            rest /= d.values.len();
            apply(&mut g, &room_id, k, v);
            values.push(v.clone());
        }
        let preview = g.describe_preview();
        let (text, prose) = room_text(&preview);
        let warnings = warnings(&g, &room_id, &prose);
        cells.push(MatrixCell {
            values,
            text,
            warnings,
        });
    }
    Some(RoomMatrix {
        room: room_id.to_string(),
        dimensions,
        cells,
        combinations,
    })
}

/// Put a game into one combination of a room matrix.
pub(crate) fn apply_cell(game: &mut Game, room: &str, assignment: &[(String, String)]) -> bool {
    let Some(ri) = game.world().room_idx(room) else {
        return false;
    };
    let room_id = game.world().story.rooms[ri].id.clone();
    enter(game, &room_id);
    for (name, value) in assignment {
        apply(game, &room_id, &key_named(name), value);
    }
    true
}

/// The full room text of a room description: the written text, then the things. Also the written text alone.
pub(crate) fn room_text(turn: &Turn) -> (String, String) {
    let of = |kind: BlockKind| {
        turn.blocks
            .iter()
            .filter(|b| b.kind == kind)
            .map(|b| b.plain_text())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prose = of(BlockKind::Prose);
    let objects = of(BlockKind::Objects);
    let text = if objects.is_empty() {
        prose.clone()
    } else {
        format!("{prose}\n{objects}")
    };
    (text, prose)
}

/// Put the player in the room without a turn. A room that the player never saw counts as seen one time.
fn enter(game: &mut Game, room: &Id) {
    let state = game.state_mut();
    state.room = room.clone();
    let visits = state.visits.entry(room.clone()).or_default();
    *visits = (*visits).max(1);
}

fn key_named(name: &str) -> Key {
    match name {
        "time.phase" => Key::Phase,
        "time.hour" => Key::Hour,
        "first_visit" => Key::FirstVisit,
        _ => match name.rsplit_once('.') {
            Some((x, "visits")) => Key::Visits(Id::new(x)),
            Some((x, "carried")) => Key::Carried(Id::new(x)),
            Some((x, "present")) => Key::Present(Id::new(x)),
            Some((x, "talked")) => Key::Talked(Id::new(x)),
            Some((x, "interactions")) => Key::Interactions(Id::new(x)),
            _ => Key::Fact(name.to_string()),
        },
    }
}

/// One fragment text in all combinations of the values that it depends on, in the current room.
/// For the "Other values" list of the writer. A combination in which the condition does not hold
/// has an empty text. None when the text or the condition has a problem.
pub fn text_matrix(game: &Game, src: &str, when: Option<&str>) -> Option<RoomMatrix> {
    let world = game.world();
    let segments = loremaster_lang::compile_markup(src, &world.scope).ok()?;
    let condition = match when.map(str::trim).filter(|w| !w.is_empty()) {
        Some(w) => Some(loremaster_lang::compile_condition(w, &world.scope).ok()?),
        None => None,
    };
    let mut found = Found::default();
    found.segments(&segments);
    if let Some(c) = &condition {
        found.condition(c);
    }
    let keys: Vec<Key> = found
        .keys
        .keys()
        .chain(found.texts.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let dimensions: Vec<Dimension> = keys
        .iter()
        .map(|k| Dimension {
            name: k.name(),
            values: values_for(game, k, &found),
        })
        .collect();
    let combinations = dimensions
        .iter()
        .map(|d| d.values.len().max(1))
        .product::<usize>();
    let room_id = game.state().room.clone();
    let mut cells = Vec::new();
    for index in 0..combinations.min(MAX_CELLS) {
        let mut rest = index;
        let mut values = Vec::new();
        let mut g = game.clone();
        for (k, d) in keys.iter().zip(&dimensions) {
            let v = &d.values[rest % d.values.len()];
            rest /= d.values.len();
            apply(&mut g, &room_id, k, v);
            values.push(v.clone());
        }
        let shows = condition.as_ref().is_none_or(|c| g.eval().holds(c));
        let text = if shows {
            g.try_markup(src).text
        } else {
            String::new()
        };
        let mut warnings = warnings(&g, &room_id, &text);
        warnings.retain(|w| !w.contains("presence") && !w.contains("replacing"));
        for w in &mut warnings {
            if w == "no room text" {
                *w = "no text".to_string();
            }
        }
        if !shows {
            warnings = vec!["the condition does not hold".to_string()];
        }
        cells.push(MatrixCell {
            values,
            text,
            warnings,
        });
    }
    Some(RoomMatrix {
        room: room_id.to_string(),
        dimensions,
        cells,
        combinations,
    })
}
