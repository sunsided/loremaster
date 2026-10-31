use crate::{Effect, EventPattern, Id, Note};
use serde::{Deserialize, Serialize};

/// A puzzle with named states and the steps between them, for example a dam
/// with its gates, or a ritual of bell, book and candles. The current state is
/// a story variable with the machine's id: `exorcism == lit` in conditions.
///
/// The loader turns each step into a rule, so the rules view, the explorer and
/// the checks see the steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub id: Id,
    /// The state at the start of the game.
    pub start: String,
    /// All states: a name, or `{ name, description }`. Without the list, the
    /// states are the start state and every state that a step names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<StateDef>,
    #[serde(default)]
    pub steps: Vec<Step>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// One step of a machine: in one of the `from` states, on the event, while
/// `when` holds, the machine goes to `to`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub from: States,
    pub on: EventPattern,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub to: String,
    /// Text for the step, before the effects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Effect>,
    /// For verb events: the step replaces the default response.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub instead: bool,
}

/// A state of a machine: `idle`, or `{ name: idle, description: "The spirits jeer." }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StateDef {
    Name(String),
    Described {
        name: String,
        /// Author text about the state, for the Atlas. The player never sees it.
        description: String,
    },
}

impl StateDef {
    pub fn name(&self) -> &str {
        match self {
            StateDef::Name(n) | StateDef::Described { name: n, .. } => n,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            StateDef::Name(_) => None,
            StateDef::Described { description, .. } => Some(description),
        }
    }
}

/// One state, or a list: `from: idle` or `from: [rung, lit]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum States {
    One(String),
    Many(Vec<String>),
}

impl States {
    pub fn list(&self) -> Vec<&str> {
        match self {
            States::One(s) => vec![s.as_str()],
            States::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

impl Machine {
    /// The declared states, or the start state and every state a step names.
    pub fn all_states(&self) -> Vec<String> {
        if !self.states.is_empty() {
            return self.states.iter().map(|s| s.name().to_string()).collect();
        }
        let mut out = vec![self.start.clone()];
        for step in &self.steps {
            for s in step.from.list().into_iter().chain([step.to.as_str()]) {
                if !out.iter().any(|x| x == s) {
                    out.push(s.to_string());
                }
            }
        }
        out
    }

    /// The description of a state, if the list of states has one.
    pub fn description(&self, state: &str) -> Option<&str> {
        self.states
            .iter()
            .find(|s| s.name() == state)
            .and_then(StateDef::description)
    }

    /// The id of the rule that the loader makes for step `index`.
    pub fn step_rule(&self, index: usize) -> Id {
        Id::new(format!("{}#{}", self.id, index + 1))
    }
}
