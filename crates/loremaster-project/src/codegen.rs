//! `loremaster codegen` (DS-14): an override as a `world!` snippet, so that
//! the programmer can paste it over the Rust part.
//!
//! shortcut: rooms, things, endings and timers print as `world!`, except for
//! the fields in `hand_field`; the other kinds, and those fields, print as a
//! `// port by hand` comment with their JSON. Printing rules needs a printer
//! for every effect; upgrade when writers override rules in practice.

use crate::Pack;
use loremaster_model::{Ending, Fragment, FragmentMode, Layer, Location, Note, Room, Thing, Timer};
use serde_json::Value;
use std::fmt::Write;

/// The `world!` snippet for the entity `key` of `pack`.
pub fn world_snippet(pack: &Pack, key: &str) -> Option<String> {
    let (kind, id) = key.split_once(':')?;
    // `world!` reads an id as a Rust identifier and has no quoted form.
    // shortcut: ASCII only, so a non-ASCII identifier is ported by hand too.
    let ident = id
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let body = match kind {
        _ if !ident => format!("    // port by hand: {key}: {}\n", pack.entity_value(key)?),
        "room" => room(pack.rooms.iter().find(|e| e.id.as_str() == id)?),
        "thing" => thing(pack.things.iter().find(|e| e.id.as_str() == id)?),
        "ending" => ending(pack.endings.iter().find(|e| e.id.as_str() == id)?),
        "timer" => timer(pack.timers.iter().find(|e| e.id.as_str() == id)?),
        _ => format!("    // port by hand: {key}: {}\n", pack.entity_value(key)?),
    };
    Some(format!("world! {{\n{body}}}\n"))
}

fn text(s: &str) -> String {
    format!("{s:?}")
}

/// A `[layer] [mode] [when c =>] "text" [id x] [replaces] [note "n"]` fragment.
/// `None` for what `world!` cannot say.
fn frag(f: &Fragment) -> Option<String> {
    if !f.one_of.is_empty() || !f.learn.is_empty() || f.notes.iter().any(|n| !plain_note(n)) {
        return None;
    }
    let mut out = String::new();
    if f.layer != Layer::Base {
        let _ = write!(out, "{} ", word(&f.layer));
    }
    if f.mode != FragmentMode::Always {
        let _ = write!(out, "{} ", word(&f.mode));
    }
    if let Some(when) = &f.when {
        let _ = write!(out, "when {when} => ");
    }
    if let [one] = f.variants.as_slice() {
        out += &text(one);
    } else {
        let list: Vec<String> = f.variants.iter().map(|v| text(v)).collect();
        let _ = write!(out, "[{}]", list.join(", "));
    }
    if let Some(id) = &f.id {
        let _ = write!(out, " id {id}");
    }
    if f.replaces {
        out += " replaces";
    }
    for n in &f.notes {
        let _ = write!(out, " note {}", text(&n.text));
    }
    Some(out)
}

fn plain_note(n: &Note) -> bool {
    n.id.is_none() && !n.todo && !n.done && n.thread.is_none() && n.role.is_none()
}

/// The snake_case word of a unit variant, as in YAML.
fn word<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn frags(out: &mut String, prefix: &str, list: &[Fragment]) {
    for f in list {
        match frag(f) {
            Some(s) => {
                let _ = writeln!(out, "        {prefix}{s};");
            }
            None => hand(
                out,
                "fragment",
                &serde_json::to_value(f).unwrap_or_default(),
            ),
        }
    }
}

fn hand(out: &mut String, what: &str, value: &Value) {
    let _ = writeln!(out, "        // port by hand: {what}: {value}");
}

fn notes(out: &mut String, list: &[Note]) {
    for n in list {
        if n.id.is_some() || n.done || n.thread.is_some() || n.role.is_some() {
            hand(out, "note", &serde_json::to_value(n).unwrap_or_default());
        } else {
            let kw = if n.todo { "todo" } else { "note" };
            let _ = writeln!(out, "        {kw} {};", text(&n.text));
        }
    }
}

/// Fields that `world!` has no syntax for here: they become comments.
fn hand_field<T: serde::Serialize>(out: &mut String, e: &T, fields: &[&str]) {
    let v = serde_json::to_value(e).unwrap_or_default();
    for f in fields {
        if let Some(value) = v.get(*f) {
            hand(out, f, value);
        }
    }
}

fn room(r: &Room) -> String {
    let mut b = String::new();
    if let Some([x, y]) = r.position {
        let _ = writeln!(b, "        at [{x:?}, {y:?}];");
    }
    if let Some(region) = &r.region {
        let _ = writeln!(b, "        region {region};");
    }
    if let Some(level) = r.level {
        let _ = writeln!(b, "        level {level};");
    }
    for t in &r.tags {
        let _ = writeln!(b, "        tag {};", text(t));
    }
    frags(&mut b, "", &r.description);
    for e in &r.exits {
        let mut line = format!("exit {}", text(e.direction.name()));
        if let Some(to) = &e.to {
            let _ = write!(line, " -> {to}");
        }
        if let Some(d) = &e.door {
            let _ = write!(line, " door {d}");
        }
        if let Some(c) = &e.visible {
            let _ = write!(line, " visible {c}");
        }
        if let Some(c) = &e.passable {
            let _ = write!(line, " passable {c}");
        }
        if let Some(t) = &e.blocked {
            let _ = write!(line, " blocked {}", text(t));
        }
        if let Some(t) = &e.traversal {
            let _ = write!(line, " traversal {}", text(t));
        }
        let _ = writeln!(b, "        {line};");
    }
    notes(&mut b, &r.notes);
    hand_field(&mut b, r, &["frames", "rules"]);
    format!("    room {} {} {{\n{b}    }}\n", r.id, text(&r.name))
}

fn thing(t: &Thing) -> String {
    let place = match &t.location {
        Location::Room(r) => format!("in {r}"),
        Location::In(c) => format!("inside {c}"),
        Location::On(c) => format!("upon {c}"),
        Location::Carried => "carried".into(),
        Location::Nowhere => "nowhere".into(),
    };
    let mut b = String::new();
    if !t.aliases.is_empty() {
        let list: Vec<String> = t.aliases.iter().map(|a| text(a)).collect();
        let _ = writeln!(b, "        aliases [{}];", list.join(", "));
    }
    for (on, word) in [
        (t.portable, "portable"),
        (t.container, "container"),
        (t.supporter, "supporter"),
        (!t.listed, "unlisted"),
        (t.proper, "proper"),
    ] {
        if on {
            let _ = writeln!(b, "        {word};");
        }
    }
    if let Some(c) = t.capacity {
        let _ = writeln!(b, "        capacity {c};");
    }
    if let Some(k) = &t.key {
        let _ = writeln!(b, "        key {k};");
    }
    for a in &t.also_in {
        let _ = writeln!(b, "        also_in {a};");
    }
    for (k, v) in &t.props {
        let _ = writeln!(
            b,
            "        prop {k} = {};",
            serde_json::to_value(v).unwrap_or_default()
        );
    }
    frags(&mut b, "examine ", &t.description);
    frags(&mut b, "presence ", &t.presence);
    notes(&mut b, &t.notes);
    hand_field(
        &mut b,
        t,
        &["head", "red_herring", "names", "frames", "rules"],
    );
    format!(
        "    thing {} {} {place} {{\n{b}    }}\n",
        t.id,
        text(&t.name)
    )
}

fn ending(e: &Ending) -> String {
    let mut b = format!("        kind {};\n", e.kind.as_str());
    if let Some(r) = e.rank {
        let _ = writeln!(b, "        rank {r};");
    }
    if !e.text.is_empty() {
        let _ = writeln!(b, "        {};", text(&e.text));
    }
    notes(&mut b, &e.notes);
    format!("    ending {} {} {{\n{b}    }}\n", e.id, text(&e.title))
}

fn timer(t: &Timer) -> String {
    let mut b = String::new();
    if t.running {
        b += "        running;\n";
    }
    if t.repeat {
        b += "        repeat;\n";
    }
    if let Some(c) = &t.runs_while {
        let _ = writeln!(b, "        while {c};");
    }
    notes(&mut b, &t.notes);
    hand_field(&mut b, t, &["at"]);
    format!("    timer {} turns {} {{\n{b}    }}\n", t.id, t.turns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use loremaster_model::{Id, NoteRole};

    fn pack_with(room: Room) -> Pack {
        Pack {
            rooms: vec![room],
            ..Pack::default()
        }
    }

    #[test]
    fn what_world_cannot_say_is_ported_by_hand() {
        let mut beat = Note::text("The gull knows.");
        beat.thread = Some(Id::new("lost_key"));
        beat.role = Some(NoteRole::Setup);
        let room = Room {
            notes: vec![beat],
            ..serde_json::from_value(serde_json::json!({ "id": "shore", "name": "Shore" })).unwrap()
        };
        let s = world_snippet(&pack_with(room), "room:shore").unwrap();
        assert!(s.contains("// port by hand: note:"), "{s}");
        assert!(s.contains("lost_key"), "{s}");

        let room: Room =
            serde_json::from_value(serde_json::json!({ "id": "front-door", "name": "Door" }))
                .unwrap();
        let s = world_snippet(&pack_with(room), "room:front-door").unwrap();
        // Alphanumeric, but no Rust identifier.
        let odd: Room =
            serde_json::from_value(serde_json::json!({ "id": "a\u{b2}", "name": "A" })).unwrap();
        let odd = world_snippet(&pack_with(odd), "room:a\u{b2}").unwrap();
        assert!(odd.contains("// port by hand"), "{odd}");
        assert!(
            s.starts_with("world! {\n    // port by hand: room:front-door:"),
            "{s}"
        );
    }
}
