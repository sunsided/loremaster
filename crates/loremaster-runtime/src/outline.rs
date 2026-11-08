use crate::World;
use loremaster_model::{Dialogue, DialogueId, DialogueTarget, Line, Outcomes, Value};
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
    /// The lines after `say`, each with its speaker.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<OutlineLine>,
    /// Where the talk goes on without a choice: a node key, `end` or `back`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// `next` names a node that does not exist.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub next_missing: bool,
    pub outcomes: Vec<String>,
    pub options: Vec<OutlineOption>,
    /// No start node leads here.
    pub unreachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineLine {
    /// A thing id or `player`.
    pub speaker: String,
    pub say: String,
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
                let targets = n.options.iter().map(|o| &o.go).chain(&n.next);
                queue.extend(targets.filter_map(|go| match go {
                    DialogueTarget::Node(g) if keys.contains(g.as_str()) => Some(g.as_str()),
                    _ => None,
                }));
            }
        }
        Self {
            id: d.id_str().to_string(),
            with: d.with.to_string(),
            start: d.start.clone(),
            nodes: d
                .nodes
                .iter()
                .map(|n| OutlineNode {
                    key: n.key.clone(),
                    start: d.start.iter().position(|s| *s == n.key),
                    when: n.when.clone(),
                    say: n.say.source(),
                    lines: (n.lines.iter())
                        .map(|l| OutlineLine {
                            speaker: l.speaker.clone(),
                            say: l.say.source(),
                        })
                        .collect(),
                    next: n.next.as_ref().map(|t| t.as_key().to_string()),
                    next_missing: matches!(&n.next, Some(DialogueTarget::Node(g)) if !keys.contains(g.as_str())),
                    outcomes: describe(&n.outcomes),
                    options: n
                        .options
                        .iter()
                        .map(|o| OutlineOption {
                            say: o.say.as_ref().map(Line::source).unwrap_or_default(),
                            when: o.when.clone(),
                            go: o.go.as_key().to_string(),
                            missing: matches!(&o.go, DialogueTarget::Node(g) if !keys.contains(g.as_str())),
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
    out.extend(
        o.set
            .iter()
            .map(|(k, v)| format!("set {k} = {}", Value::from(v))),
    );
    out.extend(o.add.iter().map(|(k, n)| format!("add {k} {n:+}")));
    out.extend(o.relation.iter().map(|(k, n)| format!("{k} {n:+}")));
    out.extend(o.remember.iter().map(|k| format!("remember {k}")));
    out.extend(o.then.iter().map(|e| match e {
        Effect::Await(_) => "wait for the host".into(),
        Effect::Combat(c) => crate::authoring::describe_combat(c),
        Effect::Move(m) => format!(
            "move {}",
            m.iter()
                .map(|(t, to)| format!("{t} to {to}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Effect::Goto(r) => format!("go to {r}"),
        Effect::End(id) => format!("end the story ({id})"),
        Effect::Say(_) => "say a line".into(),
        Effect::Set(m) => format!("set {}", m.keys().cloned().collect::<Vec<_>>().join(", ")),
        Effect::Add(m) => format!(
            "add to {}",
            m.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
        Effect::Calc(m) => format!(
            "calculate {}",
            m.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
        Effect::Stop => "stop".into(),
        Effect::Emit(e) => format!("send {e}"),
        Effect::Call(b) => format!("run the behavior {b}"),
        Effect::Start(t) => format!(
            "start {}",
            t.entries()
                .iter()
                .map(|(id, _)| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Effect::Cancel(t) => format!("cancel {t}"),
        Effect::PlaySound(s) => format!("play the sound {s}"),
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
