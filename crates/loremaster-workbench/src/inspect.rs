//! Tooltips for things and exits, and the values for the command palette.

use crate::name_of;
use loremaster_model::{EventPattern, Id, Location, Value};
use loremaster_runtime::authoring::Inspector;
use serde::{Deserialize, Serialize};

/// What the author sees about a thing or an exit in a tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inspect {
    /// `thing` or `exit`.
    pub kind: String,
    /// The thing id, or the exit direction.
    pub id: String,
    pub title: String,
    /// Label and value pairs, in display order.
    pub facts: Vec<(String, String)>,
    /// Ids of rules that name this thing or the room of this exit.
    pub rules: Vec<String>,
}

/// A variable or thing property with its value now, for the command palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarView {
    pub name: String,
    pub value: Value,
    /// For an enum variable: all values. Empty for other types.
    pub options: Vec<String>,
}

/// The phases of the day with their start times, for the command palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseView {
    pub name: String,
    pub from: String,
}

/// Facts about a thing, for an author tooltip. Does not change any state.
pub(crate) fn inspect_thing(ins: Inspector<'_>, id: &str) -> Option<Inspect> {
    let t = ins.thing(id)?;
    let mut facts: Vec<(String, String)> = vec![("id".into(), t.id.to_string())];
    if !t.names.is_empty() {
        let current = ins.name_stage(id);
        let stages: Vec<String> = t
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let when = n
                    .when
                    .as_deref()
                    .map(|w| format!(" when {w}"))
                    .unwrap_or_default();
                let now = if Some(i) == current { " (now)" } else { "" };
                format!("{}{when}{now}", n.text)
            })
            .collect();
        // No stage holds: the plain name is the current one.
        let base = match current {
            None => format!("{} (now) → ", t.name),
            Some(_) => String::new(),
        };
        facts.push(("names".into(), format!("{base}{}", stages.join(" → "))));
        facts.push(("player calls it".into(), ins.nouns(id).join(", ")));
    } else if !t.aliases.is_empty() {
        facts.push(("also called".into(), t.aliases.join(", ")));
    }
    let location = match ins.state().location_of(id) {
        Some(Location::Room(r)) => ins
            .room(r.as_str())
            .map_or(r.to_string(), |room| format!("{} ({r})", room.name)),
        Some(Location::Carried) => "carried by the player".into(),
        Some(Location::In(h)) => format!("in {} ({h})", name_of(ins, h)),
        Some(Location::On(h)) => format!("on {} ({h})", name_of(ins, h)),
        _ => "nowhere".into(),
    };
    facts.push(("location".into(), location));
    if !t.also_in.is_empty() {
        let rooms: Vec<String> = t.also_in.iter().map(Id::to_string).collect();
        facts.push(("also in".into(), rooms.join(", ")));
    }
    let mut kind = vec![if t.portable { "portable" } else { "fixed" }];
    if !t.listed {
        kind.push("not listed");
    }
    facts.push(("kind".into(), kind.join(", ")));
    if let Some(key) = &t.key {
        facts.push(("key".into(), key.to_string()));
    }
    for prop in t.props.keys() {
        let name = format!("{id}.{prop}");
        if let Some(v) = ins.state().facts.get(&name) {
            facts.push((prop.clone(), v.to_string()));
        }
    }
    let doors: Vec<String> = ins
        .world()
        .story
        .rooms
        .iter()
        .flat_map(|r| {
            r.exits
                .iter()
                .filter(|e| e.door.as_ref().is_some_and(|d| d.as_str() == id))
                .map(move |e| format!("{} {}", r.id, e.direction.name()))
        })
        .collect();
    if !doors.is_empty() {
        facts.push(("door of".into(), doors.join(", ")));
    }
    let mentions = |text: &str| text.contains(id);
    let rules = ins
        .world()
        .story
        .rules
        .iter()
        .filter(|r| {
            matches!(&r.on, EventPattern::Verb(p) | EventPattern::Check(p) if p.objects().chain(&p.actor).any(|x| x.as_str() == id))
                || r.when.as_deref().is_some_and(mentions)
                || r.then.iter().any(|e| mentions(&format!("{e:?}")))
        })
        .map(|r| r.id.to_string())
        .collect();
    Some(Inspect {
        kind: "thing".into(),
        id: id.to_string(),
        title: name_of(ins, &t.id),
        facts,
        rules,
    })
}

/// Facts about an exit of the current room, for an author tooltip.
pub(crate) fn inspect_exit(ins: Inspector<'_>, direction: &str) -> Option<Inspect> {
    let room = ins.room(ins.state().room.as_str())?;
    let (ei, exit) = room
        .exits
        .iter()
        .enumerate()
        .find(|(_, e)| e.direction.name() == direction)?;
    let to = match &exit.to {
        Some(to) => ins
            .room(to.as_str())
            .map_or(to.to_string(), |r| format!("{} ({})", r.name, r.id)),
        None => "nowhere: a message exit".to_string(),
    };
    let mut facts = vec![("leads to".to_string(), to)];
    if let Some(door) = &exit.door {
        let state = if ins.state().flag(&format!("{door}.locked")) {
            "locked"
        } else if ins.state().flag(&format!("{door}.open")) {
            "open"
        } else {
            "closed"
        };
        facts.push(("door".into(), format!("{door}, {state}")));
    }
    if let Some(src) = &exit.visible {
        let now = if ins.exit_visible(room.id.as_str(), ei) {
            "true now"
        } else {
            "false now"
        };
        facts.push(("visible when".into(), format!("{src} ({now})")));
    }
    if let Some(src) = &exit.passable {
        let now = if ins.exit_passable(room.id.as_str(), ei) {
            "true now"
        } else {
            "false now"
        };
        facts.push(("passable when".into(), format!("{src} ({now})")));
    }
    if let Some(to) = &exit.to {
        let back = ins
            .room(to.as_str())
            .is_some_and(|t| t.exits.iter().any(|e| e.to.as_ref() == Some(&room.id)));
        facts.push((
            "way back".into(),
            if back { "yes" } else { "no, one way" }.into(),
        ));
    }
    let target = exit.to.clone();
    let rules = ins
        .world()
        .story
        .rules
        .iter()
        .filter(|r| matches!(&r.on, EventPattern::Enter(x) | EventPattern::Leave(x) if Some(x) == target.as_ref()))
        .map(|r| r.id.to_string())
        .collect();
    Some(Inspect {
        kind: "exit".into(),
        id: direction.to_string(),
        title: format!("Exit {direction}"),
        facts,
        rules,
    })
}

/// All story variables and thing properties with their values now.
pub(crate) fn vars(ins: Inspector<'_>) -> Vec<VarView> {
    ins.state()
        .facts
        .iter()
        .map(|(name, value)| VarView {
            name: name.clone(),
            value: value.clone(),
            options: match ins.world().story.var(name).map(|v| &v.ty) {
                Some(loremaster_model::VarType::Enum(values)) => values.clone(),
                _ => Vec::new(),
            },
        })
        .chain(ins.world().story.timers.iter().map(|t| VarView {
            name: format!("timer.{}", t.id),
            value: Value::Int(i64::from(
                ins.state().timers.get(&t.id).copied().unwrap_or(0),
            )),
            options: Vec::new(),
        }))
        .collect()
}

/// The phases of the day, in order of their start time.
pub(crate) fn phases(ins: Inspector<'_>) -> Vec<PhaseView> {
    let mut phases: Vec<&loremaster_model::Phase> = ins.world().story.clock.phases.iter().collect();
    phases.sort_by_key(|p| p.from);
    phases
        .into_iter()
        .map(|p| PhaseView {
            name: p.name.clone(),
            from: p.from.to_string(),
        })
        .collect()
}
