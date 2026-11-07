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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WalkStep {
    Command(String),
    Checked {
        #[serde(rename = "do")]
        command: String,
        #[serde(flatten)]
        expect: Expect,
    },
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
}

/// What a response, or the game after the last step, must show and hold.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
