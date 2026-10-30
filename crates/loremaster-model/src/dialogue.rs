use crate::{Effect, Id, Note, Value};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

/// A conversation with a thing, in `dialogues/<id>.yaml`.
///
/// ```yaml
/// with: blackbeard
/// start: [again, meet]
/// nodes:
///   meet:
///     say: "\"Sit, if you like.\""
///     options:
///       - { say: "Who are you?", go: names }
///       - { say: "Goodbye.", go: end }
/// ```
///
/// The talk starts at the first node in `start` whose `when` holds. A node is
/// one line of the other character; its options are the player's answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialogue {
    pub id: Id,
    /// The thing the player talks to.
    pub with: Id,
    /// Node keys, in order. The first one whose `when` holds starts the talk.
    pub start: Vec<String>,
    /// The nodes, in the order of the file.
    #[serde(with = "ordered_nodes")]
    pub nodes: Vec<DialogueNode>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

impl Dialogue {
    pub fn node(&self, key: &str) -> Option<&DialogueNode> {
        self.nodes.iter().find(|n| n.key == key)
    }
}

/// What a node or an option changes when it applies.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Outcomes {
    /// Set variables or properties, as in a rule.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, Value>,
    /// Add to integer variables, as in a rule.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub add: BTreeMap<String, i64>,
    /// Add to the relation of the player to the dialogue's thing: `{ trust: 1 }`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relation: BTreeMap<String, i32>,
    /// Memories that the dialogue's thing keeps from now on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remember: Vec<String>,
    /// World effects: `move`, `goto`, `end`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Effect>,
}

impl Outcomes {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
            && self.add.is_empty()
            && self.relation.is_empty()
            && self.remember.is_empty()
            && self.then.is_empty()
    }
}

/// One line of the other character, and the answers the player can give.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueNode {
    /// The key of the node in `nodes`.
    pub key: String,
    /// The node can start the talk only when this holds.
    pub when: Option<String>,
    /// The line, in inline markup.
    pub say: String,
    /// Applied when the talk enters the node.
    pub outcomes: Outcomes,
    pub options: Vec<DialogueOption>,
}

/// An answer the player can choose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueOption {
    /// The player's line, also the label of the choice.
    pub say: String,
    /// The option shows only when this holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(flatten)]
    pub outcomes: Outcomes,
    /// A node key, `end` or `back`.
    #[serde(default = "end")]
    pub go: String,
}

fn end() -> String {
    "end".into()
}

/// A node as written in the file, without its key.
#[derive(Serialize, Deserialize)]
struct NodeBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<String>,
    say: String,
    #[serde(flatten)]
    outcomes: Outcomes,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    options: Vec<DialogueOption>,
}

/// `nodes` is a map in the file. Its order is the order of the outline in the
/// editor, so it is read into a list.
mod ordered_nodes {
    use super::*;

    pub fn serialize<S: Serializer>(nodes: &[DialogueNode], s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(nodes.len()))?;
        for n in nodes {
            let body = NodeBody {
                when: n.when.clone(),
                say: n.say.clone(),
                outcomes: n.outcomes.clone(),
                options: n.options.clone(),
            };
            map.serialize_entry(&n.key, &body)?;
        }
        map.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<DialogueNode>, D::Error> {
        struct Nodes;
        impl<'de> Visitor<'de> for Nodes {
            type Value = Vec<DialogueNode>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map of node keys to nodes")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some((key, body)) = map.next_entry::<String, NodeBody>()? {
                    out.push(DialogueNode {
                        key,
                        when: body.when,
                        say: body.say,
                        outcomes: body.outcomes,
                        options: body.options,
                    });
                }
                Ok(out)
            }
        }
        d.deserialize_map(Nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};

    #[test]
    fn nodes_keep_the_order_of_the_file() {
        let d: Dialogue = serde_json::from_value(serde_json::json!({
            "id": "bb",
            "with": "blackbeard",
            "start": ["meet"],
            "nodes": {
                "meet": {
                    "say": "Sit.",
                    "relation": { "trust": 1 },
                    "options": [{ "say": "Who are you?", "go": "names", "remember": ["asked"] }]
                },
                "names": { "say": "Blackbeard.", "set": { "known": true } }
            }
        }))
        .unwrap();
        let keys: Vec<&str> = d.nodes.iter().map(|n| n.key.as_str()).collect();
        check!(keys == ["meet", "names"]);
        assert!(let Some(meet) = d.node("meet"));
        check!(meet.outcomes.relation["trust"] == 1);
        check!(meet.options[0].outcomes.remember == ["asked"]);
        check!(d.node("names").unwrap().options.is_empty());

        let json = serde_json::to_value(&d).unwrap();
        check!(json["nodes"]["names"]["set"]["known"] == true);
        check!(json["nodes"]["meet"]["options"][0]["go"] == "names");
    }

    #[test]
    fn an_option_without_go_ends_the_talk() {
        let o: DialogueOption =
            serde_json::from_value(serde_json::json!({ "say": "Bye." })).unwrap();
        check!(o.go == "end");
    }
}
