use crate::World;
use loremaster_model::{Dialogue, Outcomes};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A dialogue as an outline for the editor: nodes, their options, and where
/// each option goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueOutline {
    pub id: String,
    /// The thing id.
    pub with: String,
    pub start: Vec<String>,
    pub nodes: Vec<OutlineNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineNode {
    pub key: String,
    /// The place of the node in `start`, if it is a start node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub say: String,
    pub outcomes: Vec<String>,
    pub options: Vec<OutlineOption>,
    /// No start node leads here.
    pub unreachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineOption {
    pub say: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// A node key, `end` or `back`.
    pub go: String,
    /// `go` names a node that does not exist.
    pub missing: bool,
    pub outcomes: Vec<String>,
}

impl DialogueOutline {
    pub fn of(d: &Dialogue) -> Self {
        let keys: BTreeSet<&str> = d.nodes.iter().map(|n| n.key.as_str()).collect();
        let mut reached: BTreeSet<&str> = BTreeSet::new();
        let mut queue: Vec<&str> = d
            .start
            .iter()
            .map(String::as_str)
            .filter(|k| keys.contains(k))
            .collect();
        while let Some(key) = queue.pop() {
            if !reached.insert(key) {
                continue;
            }
            if let Some(n) = d.node(key) {
                queue.extend(
                    n.options
                        .iter()
                        .map(|o| o.go.as_str())
                        .filter(|g| keys.contains(g)),
                );
            }
        }
        Self {
            id: d.id.to_string(),
            with: d.with.to_string(),
            start: d.start.clone(),
            nodes: d
                .nodes
                .iter()
                .map(|n| OutlineNode {
                    key: n.key.clone(),
                    start: d.start.iter().position(|s| *s == n.key),
                    when: n.when.clone(),
                    say: n.say.clone(),
                    outcomes: describe(&n.outcomes),
                    options: n
                        .options
                        .iter()
                        .map(|o| OutlineOption {
                            say: o.say.clone(),
                            when: o.when.clone(),
                            go: o.go.clone(),
                            missing: !matches!(o.go.as_str(), "end" | "back")
                                && !keys.contains(o.go.as_str()),
                            outcomes: describe(&o.outcomes),
                        })
                        .collect(),
                    unreachable: !reached.contains(n.key.as_str()),
                })
                .collect(),
        }
    }
}

impl World {
    /// The dialogues as outlines, in story order.
    pub fn dialogue_outlines(&self) -> Vec<DialogueOutline> {
        self.story
            .dialogues
            .iter()
            .map(DialogueOutline::of)
            .collect()
    }
}

/// Outcomes in short: `set known = true`, `trust +1`, `remember asked`.
pub(crate) fn describe(o: &Outcomes) -> Vec<String> {
    use loremaster_model::Effect;
    let mut out: Vec<String> = Vec::new();
    out.extend(o.set.iter().map(|(k, v)| format!("set {k} = {v}")));
    out.extend(o.add.iter().map(|(k, n)| format!("add {k} {n:+}")));
    out.extend(o.relation.iter().map(|(k, n)| format!("{k} {n:+}")));
    out.extend(o.remember.iter().map(|k| format!("remember {k}")));
    out.extend(o.then.iter().map(|e| match e {
        Effect::Move(m) => format!(
            "move {}",
            m.iter()
                .map(|(t, to)| format!("{t} to {to}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Effect::Goto(r) => format!("go to {r}"),
        Effect::End(_) => "end the story".into(),
        Effect::Say(_) => "say a line".into(),
        Effect::Set(m) => format!("set {}", m.keys().cloned().collect::<Vec<_>>().join(", ")),
        Effect::Add(m) => format!(
            "add to {}",
            m.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
    }));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn the_outline_marks_missing_targets_and_unreachable_nodes() {
        let d: Dialogue = serde_json::from_value(serde_json::json!({
            "id": "bb", "with": "bb", "start": ["meet"],
            "nodes": {
                "meet": { "say": "Sit.", "options": [
                    { "say": "Who?", "go": "names" },
                    { "say": "Ship?", "go": "ship", "relation": { "trust": 1 } }
                ] },
                "names": { "say": "Blackbeard." },
                "lost": { "say": "Nobody comes here." }
            }
        }))
        .unwrap();
        let o = DialogueOutline::of(&d);
        let node = |k: &str| o.nodes.iter().find(|n| n.key == k).unwrap();
        check!(node("meet").start == Some(0));
        check!(!node("meet").options[0].missing);
        check!(node("meet").options[1].missing);
        check!(node("meet").options[1].outcomes == ["trust +1"]);
        check!(!node("names").unreachable);
        check!(node("lost").unreachable);
    }
}
