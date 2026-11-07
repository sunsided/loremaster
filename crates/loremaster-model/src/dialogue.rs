use crate::{Effect, Id, Line, Note, Value};
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
///     say: ["He pulls out a chair.", { says: "Sit, if you like." }]
///     options:
///       - { say: [{ says: "Who are you?" }], go: names }
///       - { say: "Leave.", go: end }
/// ```
///
/// The talk starts at the first node in `start` whose `when` holds. A node is
/// one line of the other character; its options are the player's answers.
/// Each `say` is a [`Line`](crate::Line); the speech of a node is spoken by
/// the thing `with`, the speech of an option by the player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    /// The line. Its speech is spoken by the dialogue's thing unless a
    /// segment names another speaker with `by`.
    pub say: Line,
    /// Applied when the talk enters the node.
    pub outcomes: Outcomes,
    pub options: Vec<DialogueOption>,
}

/// An answer the player can choose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "OptionRepr")]
pub struct DialogueOption {
    /// The player's line, also the label of the choice. Its speech is
    /// spoken by the player unless a segment names another speaker.
    pub say: Line,
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

/// An option as written in the file. The fields of [`Outcomes`] are spelled
/// out instead of `#[serde(flatten)]`, so that an unknown key is an error:
/// serde does not reject unknown fields of a flattened struct.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionRepr {
    #[serde(alias = "line")]
    say: Line,
    #[serde(default)]
    when: Option<String>,
    #[serde(default = "end")]
    go: String,
    #[serde(default)]
    set: BTreeMap<String, Value>,
    #[serde(default)]
    add: BTreeMap<String, i64>,
    #[serde(default)]
    relation: BTreeMap<String, i32>,
    #[serde(default)]
    remember: Vec<String>,
    #[serde(default)]
    then: Vec<Effect>,
}

impl From<OptionRepr> for DialogueOption {
    fn from(o: OptionRepr) -> Self {
        DialogueOption {
            say: o.say,
            when: o.when,
            outcomes: Outcomes {
                set: o.set,
                add: o.add,
                relation: o.relation,
                remember: o.remember,
                then: o.then,
            },
            go: o.go,
        }
    }
}

/// A node as written in the file, without its key, with the fields of
/// [`Outcomes`] spelled out as in [`OptionRepr`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeRepr {
    #[serde(default)]
    when: Option<String>,
    #[serde(alias = "line")]
    say: Line,
    #[serde(default)]
    options: Vec<DialogueOption>,
    #[serde(default)]
    set: BTreeMap<String, Value>,
    #[serde(default)]
    add: BTreeMap<String, i64>,
    #[serde(default)]
    relation: BTreeMap<String, i32>,
    #[serde(default)]
    remember: Vec<String>,
    #[serde(default)]
    then: Vec<Effect>,
}

/// A node as it is written out, without its key.
#[derive(Serialize)]
struct NodeBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    when: Option<String>,
    say: Line,
    #[serde(flatten)]
    outcomes: Outcomes,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
                while let Some((key, body)) = map.next_entry::<String, NodeRepr>()? {
                    out.push(DialogueNode {
                        key,
                        when: body.when,
                        say: body.say,
                        outcomes: Outcomes {
                            set: body.set,
                            add: body.add,
                            relation: body.relation,
                            remember: body.remember,
                            then: body.then,
                        },
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
