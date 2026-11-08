//! The contract between a story and its host (IF-1): the custom actions that
//! the story asks the host to do, and the host events that the host reports.
//!
//! ```yaml
//! # story.yaml
//! interface:
//!   actions:
//!     - { id: lockpick, args: { difficulty: int }, results: [success, failure, broken_pick] }
//!   events:
//!     - { id: killed, args: { boss: text }, when: "player_alive" }
//! ```

use crate::{Id, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// The type of an argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    Bool,
    Int,
    Text,
    Enum(Vec<String>),
}

impl ArgType {
    fn fits(&self, value: &Value) -> bool {
        match (self, value) {
            (ArgType::Bool, Value::Bool(_))
            | (ArgType::Int, Value::Int(_))
            | (ArgType::Text, Value::Text(_)) => true,
            (ArgType::Enum(values), Value::Text(t)) => values.contains(t),
            _ => false,
        }
    }
}

impl fmt::Display for ArgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgType::Bool => f.write_str("bool"),
            ArgType::Int => f.write_str("int"),
            ArgType::Text => f.write_str("text"),
            ArgType::Enum(values) => write!(f, "one of {}", values.join(", ")),
        }
    }
}

/// A custom action: the host does it and reports one of its results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDecl {
    pub id: Id,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, ArgType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<Id>,
}

/// A host event: the host reports it, and rules `on: { event: id }` run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDecl {
    pub id: Id,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, ArgType>,
    /// The session refuses the event while this condition does not hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// The `interface:` section of a story. See the module docs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<EventDecl>,
}

impl Interface {
    pub fn action(&self, id: &str) -> Option<&ActionDecl> {
        self.actions.iter().find(|a| a.id.as_str() == id)
    }

    pub fn event(&self, id: &str) -> Option<&EventDecl> {
        self.events.iter().find(|e| e.id.as_str() == id)
    }

    /// Read the JSON form, for example the constant of the generated types.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The same declarations in one order, for the hash and the comparison.
    fn canonical(&self) -> Interface {
        let mut c = self.clone();
        c.actions.sort_by(|a, b| a.id.cmp(&b.id));
        c.events.sort_by(|a, b| a.id.cmp(&b.id));
        for a in &mut c.actions {
            a.results.sort();
            a.args.values_mut().for_each(sort_enum);
        }
        for e in &mut c.events {
            e.args.values_mut().for_each(sort_enum);
        }
        c
    }

    /// SHA-256 of the canonical JSON, in hex. The order of the declarations
    /// and of the results does not change it.
    pub fn hash(&self) -> String {
        let json = serde_json::to_vec(&self.canonical()).expect("an interface is plain data");
        Sha256::digest(&json)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// `Err` with what differs, when a host built for `built` meets this
    /// interface (IF-5). "Added" means in the story and not in the host.
    pub fn compare(&self, built: &Interface) -> Result<(), InterfaceMismatch> {
        let (story, host) = (self.canonical(), built.canonical());
        if story == host {
            return Ok(());
        }
        let mut m = InterfaceMismatch {
            story: self.hash(),
            built: built.hash(),
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
        };
        for a in &story.actions {
            match host.action(a.id.as_str()) {
                None => m.added.push(format!("action {}", a.id)),
                Some(b) if a != b => m.changed.push(format!("action {}", a.id)),
                Some(_) => {}
            }
        }
        for e in &story.events {
            match host.event(e.id.as_str()) {
                None => m.added.push(format!("event {}", e.id)),
                Some(b) if e != b => m.changed.push(format!("event {}", e.id)),
                Some(_) => {}
            }
        }
        for a in host
            .actions
            .iter()
            .filter(|a| story.action(a.id.as_str()).is_none())
        {
            m.removed.push(format!("action {}", a.id));
        }
        for e in host
            .events
            .iter()
            .filter(|e| story.event(e.id.as_str()).is_none())
        {
            m.removed.push(format!("event {}", e.id));
        }
        Err(m)
    }
}

fn sort_enum(ty: &mut ArgType) {
    if let ArgType::Enum(values) = ty {
        values.sort();
    }
}

/// `Err` unless `given` has exactly the declared arguments, with their types.
/// `of` names the action or event in the message.
pub fn check_args(
    of: &str,
    declared: &BTreeMap<String, ArgType>,
    given: &BTreeMap<String, Value>,
) -> Result<(), String> {
    for (name, value) in given {
        match declared.get(name) {
            None => return Err(format!("{of} has no argument `{name}`")),
            Some(ty) if !ty.fits(value) => {
                return Err(format!(
                    "argument `{name}` of {of} must be {ty}, not `{value}`"
                ));
            }
            Some(_) => {}
        }
    }
    match declared.keys().find(|name| !given.contains_key(*name)) {
        Some(name) => Err(format!("{of} needs the argument `{name}`")),
        None => Ok(()),
    }
}

/// The host was built for another interface than the story has (IF-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterfaceMismatch {
    /// The hash of the story's interface.
    pub story: String,
    /// The hash that the host was built for.
    pub built: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl fmt::Display for InterfaceMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the host was built for interface {}, but the story has {}",
            self.built, self.story
        )?;
        for (word, list) in [
            ("added", &self.added),
            ("removed", &self.removed),
            ("changed", &self.changed),
        ] {
            if !list.is_empty() {
                write!(f, "; {word}: {}", list.join(", "))?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for InterfaceMismatch {}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn action(id: &str, results: &[&str]) -> ActionDecl {
        ActionDecl {
            id: Id::new(id),
            args: BTreeMap::new(),
            results: results.iter().map(|r| Id::new(*r)).collect(),
        }
    }

    #[test]
    fn the_hash_ignores_order_but_not_content() {
        let a = Interface {
            actions: vec![action("x", &["a", "b"]), action("y", &[])],
            events: vec![],
        };
        let b = Interface {
            actions: vec![action("y", &[]), action("x", &["b", "a"])],
            events: vec![],
        };
        check!(a.hash() == b.hash());
        check!(a.compare(&b).is_ok());
        let c = Interface {
            actions: vec![action("x", &["a"]), action("z", &[])],
            events: vec![],
        };
        check!(a.hash() != c.hash());
    }

    #[test]
    fn a_mismatch_names_added_removed_and_changed() {
        let story = Interface {
            actions: vec![action("kept", &["a", "b"]), action("new", &[])],
            events: vec![],
        };
        let built = Interface {
            actions: vec![action("kept", &["a"]), action("old", &[])],
            events: vec![],
        };
        let m = story.compare(&built).unwrap_err();
        check!(m.added == ["action new"]);
        check!(m.removed == ["action old"]);
        check!(m.changed == ["action kept"]);
        let text = m.to_string();
        check!(text.contains("added: action new"), "{text}");
        check!(text.contains("removed: action old"), "{text}");
        check!(text.contains("changed: action kept"), "{text}");
    }

    #[test]
    fn arguments_are_checked() {
        let declared = BTreeMap::from([("n".to_string(), ArgType::Int)]);
        let ok = BTreeMap::from([("n".to_string(), Value::Int(1))]);
        check!(check_args("x", &declared, &ok).is_ok());
        let wrong = BTreeMap::from([("n".to_string(), Value::Text("a".into()))]);
        check!(check_args("x", &declared, &wrong).is_err());
        check!(check_args("x", &declared, &BTreeMap::new()).is_err());
        let extra = BTreeMap::from([
            ("n".to_string(), Value::Int(1)),
            ("m".to_string(), Value::Int(1)),
        ]);
        check!(check_args("x", &declared, &extra).is_err());
    }
}
