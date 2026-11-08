use crate::{Effect, EventPattern, Id, Line, Note};
use serde::{Deserialize, Serialize};

/// A puzzle with named states and the steps between them, for example a dam
/// with its gates, or a ritual of bell, book and candles. The current state is
/// a story variable with the machine's id: `exorcism == lit` in conditions.
///
/// The loader turns each step into a rule, so the rules view, the explorer and
/// the checks see the steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// The machine is a quest (decision D8): its states give the player's
    /// journal (`journal:` text per state).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quest: bool,
    /// The quest's name in the journal. Without it, the journal uses the id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// One step of a machine: in one of the `from` states, on the event, while
/// `when` holds, the machine goes to `to`. With `on: next`, the step runs by
/// itself right after the machine enters a `from` state, in the same turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub from: States,
    pub on: EventPattern,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub to: String,
    /// Text for the step, before the effects. `line:` is the same key.
    #[serde(default, alias = "line", skip_serializing_if = "Option::is_none")]
    pub say: Option<Line>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Effect>,
    /// For verb events: the step replaces the default response.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub instead: bool,
    /// The endings that this step closes on purpose, as `closes` of a rule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<Id>,
    /// Why the step closes the endings in `closes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A state of a machine: `idle`, or `{ name: idle, description: "The spirits jeer." }`.
/// A quest state can also have `journal:` (the player's journal text) and
/// `done: true` (the quest is finished in this state).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StateDef {
    Name(String),
    Described {
        name: String,
        /// Author text about the state, for the Atlas. The player never sees it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// The journal text of a quest in this state. Without it, the quest
        /// is not in the journal (for example, before it starts).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        journal: Option<Line>,
        /// The quest is finished in this state.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        done: bool,
    },
}

/// The long form of a state, as written in the file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DescribedState {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    journal: Option<Line>,
    #[serde(default)]
    done: bool,
}

impl<'de> Deserialize<'de> for StateDef {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        crate::de::string_or_map(
            d,
            "a state name, or a map with `name` and `description`",
            StateDef::Name,
            |s: DescribedState| StateDef::Described {
                name: s.name,
                description: s.description,
                journal: s.journal,
                done: s.done,
            },
        )
    }
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
            StateDef::Described { description, .. } => description.as_deref(),
        }
    }

    /// The journal text of a quest in this state.
    pub fn journal(&self) -> Option<&Line> {
        match self {
            StateDef::Name(_) => None,
            StateDef::Described { journal, .. } => journal.as_ref(),
        }
    }

    /// The quest is finished in this state.
    pub fn done(&self) -> bool {
        matches!(self, StateDef::Described { done: true, .. })
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

    /// The declared state with this name.
    pub fn state(&self, state: &str) -> Option<&StateDef> {
        self.states.iter().find(|s| s.name() == state)
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
