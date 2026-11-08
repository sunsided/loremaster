use crate::{Id, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A test that plays the story: commands, and what must show or be true.
/// Walkthroughs are in `tests/*.yaml`, and `loremaster test` runs them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Walkthrough {
    /// The name in the report. Without it, the file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Commands, in order. A step can also say what its response must show.
    pub steps: Vec<WalkStep>,
    /// The seed of the RNG for this test, instead of the story's seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// What must hold after the last step.
    #[serde(default)]
    pub expect: Expect,
    /// The walkthrough cannot pass yet: `pending: { phase: 2, needs: SE-4 }`.
    /// The runner expects it to fail, and reports a pass as an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

/// Why a test cannot pass yet: the roadmap phase and the requirement that make it pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    /// The roadmap phase after which the test must pass.
    pub phase: u8,
    /// The requirement id or the issue that the test waits for, for example `SE-4` or `#45`.
    pub needs: String,
}

impl std::fmt::Display for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pending: phase {}, needs {}", self.phase, self.needs)
    }
}

/// One command: `"open drawer"`, or `{ do: "open drawer", see: "an iron key" }`.
///
/// `rolls: [4, 11]` gives the totals of the rolls in this step, in order. A
/// roll without a forced total takes one from the RNG.
///
/// A step with an `id` is a node of the dependency chart and can have hints
/// (CK-4): `{ id: key, do: "open drawer", at: cottage, needs: [lamp],
/// hints: ["…", "…"] }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum WalkStep {
    Command(String),
    Checked {
        #[serde(rename = "do")]
        command: String,
        #[serde(flatten)]
        expect: Expect,
        /// Boxed: most steps have no plan, and the enum stays small.
        #[serde(flatten)]
        plan: Box<StepPlan>,
        /// The totals of the rolls in this step, in order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        rolls: Vec<i64>,
    },
}

/// Where a step stands in the solution: its id, its room, the steps it
/// needs, and the hints that lead to it (CK-4). All fields are optional. The
/// hint ladder (CK-6) and the dependency chart (CK-7) come from them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepPlan {
    /// The name of the step, for `needs`, `before` and the chart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// The room where the player does the step. Without it, the room that
    /// the walkthrough is in before the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Id>,
    /// Steps of this walkthrough that must be done first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs: Vec<Id>,
    /// Later steps of this walkthrough that need this one: `needs` written
    /// on the other side.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<Id>,
    /// Up to three hints, from a nudge to almost the command. After the last
    /// one, the hint ladder shows the command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
    /// The step reports this host event instead of a command (IF-6):
    /// `{ event: killed, args: { boss: ogre } }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<Id>,
    /// The arguments of `event`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, Value>,
}

impl StepPlan {
    pub fn is_empty(&self) -> bool {
        self == &StepPlan::default()
    }
}

/// A step with checks, as written in the file. The fields of [`Expect`] are
/// spelled out instead of `#[serde(flatten)]`, so that an unknown key is an
/// error: serde does not reject unknown fields of a flattened struct.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedStep {
    #[serde(default, rename = "do")]
    command: Option<String>,
    #[serde(default)]
    event: Option<Id>,
    #[serde(default)]
    args: BTreeMap<String, Value>,
    #[serde(default, deserialize_with = "one_or_many")]
    see: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    not_see: Vec<String>,
    #[serde(default)]
    facts: BTreeMap<String, Value>,
    #[serde(default)]
    room: Option<Id>,
    #[serde(default)]
    ended: Option<bool>,
    #[serde(default)]
    ending: Option<Id>,
    #[serde(default, deserialize_with = "one_or_many")]
    sound: Vec<String>,
    #[serde(default)]
    id: Option<Id>,
    #[serde(default)]
    at: Option<Id>,
    #[serde(default, deserialize_with = "one_or_many")]
    needs: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    before: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    hints: Vec<String>,
    #[serde(default)]
    rolls: Vec<i64>,
}

impl<'de> Deserialize<'de> for WalkStep {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        crate::de::string_or_map(
            d,
            "a command, or a map with `do` and what the response must show",
            WalkStep::Command,
            |s: CheckedStep| WalkStep::Checked {
                // An event step needs no command; the report shows this text.
                command: s.command.unwrap_or_else(|| {
                    s.event
                        .as_ref()
                        .map(|e| format!("event {e}"))
                        .unwrap_or_default()
                }),
                expect: Expect {
                    see: s.see,
                    not_see: s.not_see,
                    facts: s.facts,
                    room: s.room,
                    ended: s.ended,
                    ending: s.ending,
                    sound: s.sound,
                },
                plan: Box::new(StepPlan {
                    id: s.id,
                    at: s.at,
                    needs: s.needs.into_iter().map(Id::new).collect(),
                    before: s.before.into_iter().map(Id::new).collect(),
                    hints: s.hints,
                    event: s.event,
                    args: s.args,
                }),
                rolls: s.rolls,
            },
        )
    }
}

impl WalkStep {
    pub fn command(&self) -> &str {
        match self {
            WalkStep::Command(c) | WalkStep::Checked { command: c, .. } => c,
        }
    }

    pub fn expect(&self) -> Option<&Expect> {
        match self {
            WalkStep::Command(_) => None,
            WalkStep::Checked { expect, .. } => Some(expect),
        }
    }

    /// The id, room, needs and hints of the step, if it has any.
    pub fn plan(&self) -> Option<&StepPlan> {
        match self {
            WalkStep::Command(_) => None,
            WalkStep::Checked { plan, .. } => Some(plan.as_ref()),
        }
    }

    /// The forced totals of the rolls in this step.
    pub fn rolls(&self) -> &[i64] {
        match self {
            WalkStep::Command(_) => &[],
            WalkStep::Checked { rolls, .. } => rolls,
        }
    }

    /// The id of the step, for `needs` and the chart.
    pub fn id(&self) -> Option<&Id> {
        self.plan()?.id.as_ref()
    }
}

impl Walkthrough {
    /// Some step has an id or hints: the walkthrough feeds the hint ladder
    /// and the dependency chart.
    pub fn is_planned(&self) -> bool {
        self.steps.iter().any(|s| {
            s.plan()
                .is_some_and(|p| p.id.is_some() || !p.hints.is_empty())
        })
    }
}

/// What a response, or the game after the last step, must show and hold.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// Text that must show: in the response of a step, or anywhere in the transcript at the end.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "one_or_many"
    )]
    pub see: Vec<String>,
    /// Text that must not show.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "one_or_many"
    )]
    pub not_see: Vec<String>,
    /// Variables and properties with their values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub facts: BTreeMap<String, Value>,
    /// The room the player is in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<Id>,
    /// The game has ended (or not).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<bool>,
    /// The game has ended with this ending, from `endings/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<Id>,
    /// Sound assets that must play: in the step, or anywhere in the game at the end.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "one_or_many"
    )]
    pub sound: Vec<String>,
}

fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use serde_json::json;

    #[test]
    fn steps_round_trip_in_both_forms() {
        let steps = json!(["look", { "do": "open drawer", "see": ["a key"], "room": "cottage" }]);
        let read: Vec<WalkStep> = serde_json::from_value(steps.clone()).unwrap();
        check!(read[0] == WalkStep::Command("look".into()));
        check!(read[1].expect().unwrap().see == ["a key"]);
        check!(serde_json::to_value(&read).unwrap() == steps);
    }

    #[test]
    fn a_planned_step_reads_its_id_room_needs_and_hints() {
        let step = json!({
            "id": "key", "do": "open drawer", "at": "cottage", "needs": "lamp",
            "before": ["door"], "hints": ["Look around.", "The drawer."], "see": "a key"
        });
        let read: WalkStep = serde_json::from_value(step.clone()).unwrap();
        let plan = read.plan().unwrap();
        check!(read.id() == Some(&Id::new("key")));
        check!(plan.at == Some(Id::new("cottage")));
        check!(plan.needs == [Id::new("lamp")]);
        check!(plan.before == [Id::new("door")]);
        check!(plan.hints.len() == 2);
        check!(read.expect().unwrap().see == ["a key"]);
        // One value reads as a list and writes back as one.
        let back = serde_json::to_value(&read).unwrap();
        check!(back["needs"] == json!(["lamp"]));
        check!(back["see"] == json!(["a key"]));
    }

    #[test]
    fn a_step_reads_its_forced_rolls() {
        let step = json!({ "do": "test luck", "rolls": [4, 11] });
        let read: WalkStep = serde_json::from_value(step.clone()).unwrap();
        check!(read.rolls() == [4, 11]);
        check!(serde_json::to_value(&read).unwrap() == step);
    }

    #[test]
    fn an_unknown_field_of_a_step_names_the_field() {
        let err = serde_json::from_value::<WalkStep>(json!({ "do": "look", "sees": "x" }))
            .unwrap_err()
            .to_string();
        check!(err.contains("unknown field `sees`"), "{err}");
    }
}
