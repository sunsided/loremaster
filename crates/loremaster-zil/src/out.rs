//! The imported world, and how it is written as a loremaster project folder.

use std::fmt::Write as _;

/// One fragment of a description or presence text.
#[derive(Debug, Clone, PartialEq)]
pub struct Frag {
    pub text: String,
    pub when: Option<String>,
}

impl Frag {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            when: None,
        }
    }

    pub fn when(when: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            when: Some(when.into()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExitOut {
    pub direction: String,
    /// The room the exit leads to; `None` for a message exit.
    pub to: Option<String>,
    pub visible: Option<String>,
    pub door: Option<String>,
    pub passable: Option<String>,
    pub blocked: Option<String>,
    pub traversal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoomOut {
    pub id: String,
    /// The ZIL name and file, for the comment at the top of the file.
    pub origin: String,
    pub name: String,
    pub tags: Vec<String>,
    pub position: (i32, i32),
    pub description: Vec<Frag>,
    pub exits: Vec<ExitOut>,
}

#[derive(Debug, Clone)]
pub struct ThingOut {
    pub id: String,
    pub origin: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub location: crate::convert::Place,
    /// More rooms for local-global scenery.
    pub also_in: Vec<String>,
    pub container: bool,
    pub supporter: bool,
    pub portable: bool,
    pub key: Option<String>,
    pub listed: bool,
    pub props: Vec<(String, bool)>,
    pub description: Vec<Frag>,
    pub presence: Vec<Frag>,
}

#[derive(Debug, Clone)]
pub enum Initial {
    Bool(bool),
    Int(i64),
}

#[derive(Debug, Clone)]
pub struct VarOut {
    pub id: String,
    pub initial: Initial,
}

#[derive(Debug, Clone)]
pub struct StoryOut {
    pub title: String,
    pub author: String,
    pub start: String,
    pub intro: Vec<Frag>,
    pub vars: Vec<VarOut>,
}

/// A file to write, with its path relative to the project folder.
pub struct OutFile {
    pub path: String,
    pub text: String,
}

/// Quote a string for YAML. A JSON string is a valid YAML double-quoted scalar.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn fragments(out: &mut String, key: &str, frags: &[Frag]) {
    if frags.is_empty() {
        return;
    }
    let _ = writeln!(out, "{key}:");
    for f in frags {
        match &f.when {
            None => {
                let _ = writeln!(out, "  - {}", quote(&f.text));
            }
            Some(when) => {
                let _ = writeln!(out, "  - when: {}", quote(when));
                let _ = writeln!(out, "    text: {}", quote(&f.text));
            }
        }
    }
}

fn list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| quote(s)).collect();
    format!("[{}]", quoted.join(", "))
}

impl StoryOut {
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "title: {}", quote(&self.title));
        let _ = writeln!(out, "author: {}", quote(&self.author));
        let _ = writeln!(out, "start: {}", self.start);
        fragments(&mut out, "intro", &self.intro);
        if !self.vars.is_empty() {
            let _ = writeln!(out, "\nvars:");
            for v in &self.vars {
                let (ty, initial) = match v.initial {
                    Initial::Bool(b) => ("bool", b.to_string()),
                    Initial::Int(i) => ("int", i.to_string()),
                };
                let _ = writeln!(
                    out,
                    "  - {{ id: {}, type: {ty}, initial: {initial} }}",
                    v.id
                );
            }
        }
        out
    }
}

impl RoomOut {
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Imported from ZIL: {}", self.origin);
        let _ = writeln!(out, "name: {}", quote(&self.name));
        if !self.tags.is_empty() {
            let _ = writeln!(out, "tags: [{}]", self.tags.join(", "));
        }
        let _ = writeln!(out, "position: [{}, {}]", self.position.0, self.position.1);
        fragments(&mut out, "description", &self.description);
        if !self.exits.is_empty() {
            let _ = writeln!(out, "exits:");
            for e in &self.exits {
                let mut fields = vec![format!("direction: {}", e.direction)];
                if let Some(to) = &e.to {
                    fields.push(format!("to: {to}"));
                }
                let optional = [
                    ("visible", &e.visible),
                    ("door", &e.door),
                    ("passable", &e.passable),
                    ("blocked", &e.blocked),
                    ("traversal", &e.traversal),
                ];
                for (key, value) in optional {
                    if let Some(v) = value {
                        let v = if key == "door" { v.clone() } else { quote(v) };
                        fields.push(format!("{key}: {v}"));
                    }
                }
                let _ = writeln!(out, "  - {{ {} }}", fields.join(", "));
            }
        }
        out
    }
}

impl ThingOut {
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Imported from ZIL: {}", self.origin);
        let _ = writeln!(out, "name: {}", quote(&self.name));
        if !self.aliases.is_empty() {
            let _ = writeln!(out, "aliases: {}", list(&self.aliases));
        }
        use crate::convert::Place;
        let _ = match &self.location {
            Place::Room(room) => writeln!(out, "location: {{ room: {room} }}"),
            Place::In(holder) => writeln!(out, "location: {{ in: {holder} }}"),
            Place::On(holder) => writeln!(out, "location: {{ on: {holder} }}"),
            Place::Nowhere => writeln!(out, "location: nowhere"),
        };
        if !self.also_in.is_empty() {
            let _ = writeln!(out, "also_in: {}", list(&self.also_in));
        }
        if self.container {
            let _ = writeln!(out, "container: true");
        }
        if self.supporter {
            let _ = writeln!(out, "supporter: true");
        }
        if self.portable {
            let _ = writeln!(out, "portable: true");
        }
        if let Some(key) = &self.key {
            let _ = writeln!(out, "key: {key}");
        }
        if !self.listed {
            let _ = writeln!(out, "listed: false");
        }
        if !self.props.is_empty() {
            let props: Vec<String> = self
                .props
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
            let _ = writeln!(out, "props: {{ {} }}", props.join(", "));
        }
        fragments(&mut out, "description", &self.description);
        fragments(&mut out, "presence", &self.presence);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn quotes_yaml_strings() {
        check!(quote("say \"hi\"\\") == r#""say \"hi\"\\""#);
    }

    #[test]
    fn writes_a_room() {
        let room = RoomOut {
            id: "kitchen".into(),
            origin: "KITCHEN (1dungeon.zil)".into(),
            name: "Kitchen".into(),
            tags: vec![],
            position: (1, 0),
            description: vec![
                Frag::plain("A kitchen."),
                Frag::when("window.open", "open."),
            ],
            exits: vec![ExitOut {
                direction: "east".into(),
                to: Some("behind_house".into()),
                door: Some("window".into()),
                ..Default::default()
            }],
        };
        check!(
            room.to_yaml()
                == "# Imported from ZIL: KITCHEN (1dungeon.zil)\n\
                    name: \"Kitchen\"\n\
                    position: [1, 0]\n\
                    description:\n  - \"A kitchen.\"\n  - when: \"window.open\"\n    text: \"open.\"\n\
                    exits:\n  - { direction: east, to: behind_house, door: window }\n"
        );
    }
}
