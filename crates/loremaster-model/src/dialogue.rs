//! Dialogues: dramaturge's dialogue AST with loremaster's payloads (D3).
//!
//! A conversation with a thing, in `dialogues/<id>.yaml`:
//!
//! ```yaml
//! with: blackbeard
//! start: [again, meet]
//! nodes:
//!   meet:
//!     say: ["He pulls out a chair.", { says: "Sit, if you like." }]
//!     options:
//!       - { say: [{ says: "Who are you?" }], go: names }
//!       - { say: "Leave.", go: end }
//! ```
//!
//! The shape is the one of `dramaturge-syntax` (with its `serde` feature);
//! loremaster sets the payloads: conditions are strings in loremaster's
//! condition language, each `say` is a [`Line`], `then` holds world
//! [`Effect`]s and `notes` hold [`Note`]s. The talk starts at the first node
//! in `start` whose `when` holds. A node is one line of the other character;
//! its options are the player's answers. The speech of a node is spoken by
//! the thing `with`, the speech of an option by the player.

use crate::{Effect, Line, Note, Value};
use dramaturge_syntax as syntax;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

pub use syntax::Target as DialogueTarget;

/// A conversation with a thing. The loader sets its `id` to the file stem.
pub type Dialogue = syntax::Dialogue<String, Line, Effect, Note>;
/// One line of the other character, and the answers the player can give.
pub type DialogueNode = syntax::Node<String, Line, Effect, Note>;
/// An answer the player can choose.
pub type DialogueOption = syntax::Choice<String, Line, Effect, Note>;
/// What a node or an option changes when it applies.
pub type Outcomes = syntax::Outcomes<Effect>;

/// The id of a dialogue as a string.
pub trait DialogueId {
    /// The id, or `""` for a dialogue without one (the loader always sets it).
    fn id_str(&self) -> &str;
}

impl DialogueId for Dialogue {
    fn id_str(&self) -> &str {
        self.id.as_deref().unwrap_or_default()
    }
}

impl From<&syntax::Value> for Value {
    fn from(v: &syntax::Value) -> Self {
        match v {
            syntax::Value::Bool(b) => Value::Bool(*b),
            syntax::Value::Int(i) => Value::Int(*i),
            syntax::Value::Text(t) => Value::Text(t.clone()),
            // A value kind that loremaster does not know yet.
            other => Value::Text(format!("{other:?}")),
        }
    }
}

impl From<Value> for syntax::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Bool(b) => syntax::Value::Bool(b),
            Value::Int(i) => syntax::Value::Int(i),
            Value::Text(t) => syntax::Value::Text(t),
        }
    }
}

/// The dialogues of a [`Story`](crate::Story) as the story serializes them.
///
/// The story's JSON is the input of the model hash and of the save stamp.
/// Before D3, an option wrote its `go` after its outcomes, and always
/// (`go: end` too). This form keeps that, so that existing stories keep
/// their model hash and their saves. Everything else is the shape of
/// `dramaturge-syntax`. Reading takes that shape as it is.
pub(crate) fn story_form<S: Serializer>(dialogues: &[Dialogue], s: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct DialogueOut<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: &'a Option<String>,
        with: &'a str,
        start: &'a [String],
        nodes: Nodes<'a>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        notes: &'a [Note],
    }
    struct Nodes<'a>(&'a [DialogueNode]);
    impl Serialize for Nodes<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut map = s.serialize_map(Some(self.0.len()))?;
            for n in self.0 {
                map.serialize_entry(&n.key, &NodeOut::of(n))?;
            }
            map.end()
        }
    }
    s.collect_seq(dialogues.iter().map(|d| DialogueOut {
        id: &d.id,
        with: &d.with,
        start: &d.start,
        nodes: Nodes(&d.nodes),
        notes: &d.notes,
    }))
}

/// The outcomes of a node or an option, next to its other fields.
#[derive(Serialize)]
struct OutcomesOut<'a> {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    set: &'a BTreeMap<String, syntax::Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    add: &'a BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    relation: &'a BTreeMap<String, i32>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    remember: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    then: &'a [Effect],
}

impl<'a> OutcomesOut<'a> {
    fn of(o: &'a Outcomes) -> Self {
        Self {
            set: &o.set,
            add: &o.add,
            relation: &o.relation,
            remember: &o.remember,
            then: &o.then,
        }
    }
}

#[derive(Serialize)]
struct NodeOut<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: &'a Option<String>,
    say: &'a Line,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    lines: &'a [syntax::Line<Line>],
    #[serde(skip_serializing_if = "Option::is_none")]
    next: &'a Option<DialogueTarget>,
    #[serde(flatten)]
    outcomes: OutcomesOut<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<OptionOut<'a>>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [Note],
}

impl<'a> NodeOut<'a> {
    fn of(n: &'a DialogueNode) -> Self {
        Self {
            speaker: &n.speaker,
            when: &n.when,
            say: &n.say,
            lines: &n.lines,
            next: &n.next,
            outcomes: OutcomesOut::of(&n.outcomes),
            options: n.options.iter().map(OptionOut::of).collect(),
            notes: &n.notes,
        }
    }
}

#[derive(Serialize)]
struct OptionOut<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    say: &'a Option<Line>,
    #[serde(skip_serializing_if = "Option::is_none")]
    when: &'a Option<String>,
    #[serde(flatten)]
    outcomes: OutcomesOut<'a>,
    go: &'a DialogueTarget,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    tags: &'a [syntax::Tag],
    #[serde(skip_serializing_if = "syntax::Intentions::is_empty")]
    intentions: &'a syntax::Intentions,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: &'a Option<syntax::Hint>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    notes: &'a [Note],
}

impl<'a> OptionOut<'a> {
    fn of(o: &'a DialogueOption) -> Self {
        Self {
            id: &o.id,
            say: &o.say,
            when: &o.when,
            outcomes: OutcomesOut::of(&o.outcomes),
            go: &o.go,
            tags: &o.tags,
            intentions: &o.intentions,
            hint: &o.hint,
            notes: &o.notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};

    fn blackbeard() -> serde_json::Value {
        serde_json::json!({
            "id": "bb",
            "with": "blackbeard",
            "start": ["meet"],
            "nodes": {
                "meet": {
                    "say": "Sit.",
                    "relation": { "trust": 1 },
                    "options": [
                        { "say": "Who are you?", "go": "names", "remember": ["asked"] },
                        { "say": "Bye." }
                    ]
                },
                "names": { "say": "Blackbeard.", "set": { "known": true } }
            }
        })
    }

    #[test]
    fn nodes_keep_the_order_of_the_file() {
        let d: Dialogue = serde_json::from_value(blackbeard()).unwrap();
        let keys: Vec<&str> = d.nodes.iter().map(|n| n.key.as_str()).collect();
        check!(keys == ["meet", "names"]);
        assert!(let Some(meet) = d.node("meet"));
        check!(meet.outcomes.relation["trust"] == 1);
        check!(meet.options[0].outcomes.remember == ["asked"]);
        check!(d.node("names").unwrap().options.is_empty());
        check!(d.id_str() == "bb");

        let json = serde_json::to_value(&d).unwrap();
        check!(json["nodes"]["names"]["set"]["known"] == true);
        check!(json["nodes"]["meet"]["options"][0]["go"] == "names");
    }

    #[test]
    fn an_option_without_go_ends_the_talk() {
        let o: DialogueOption =
            serde_json::from_value(serde_json::json!({ "say": "Bye." })).unwrap();
        check!(o.go == DialogueTarget::End);
    }

    #[test]
    fn the_story_form_writes_go_after_the_outcomes() {
        let d: Dialogue = serde_json::from_value(blackbeard()).unwrap();
        let mut out = Vec::new();
        story_form(&[d], &mut serde_json::Serializer::new(&mut out)).unwrap();
        let text = String::from_utf8(out).unwrap();
        check!(text.contains(r#"{"say":"Who are you?","remember":["asked"],"go":"names"}"#));
        check!(text.contains(r#"{"say":"Bye.","go":"end"}"#));
    }
}
