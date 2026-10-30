use crate::Id;
use serde::{Deserialize, Serialize};

/// How many objects a verb takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbSlots {
    None,
    #[default]
    One,
    /// `use X on Y`, `give X to Y`.
    Two,
}

/// A verb the player can use. The standard library is built in; stories add
/// custom verbs with the same shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerbDef {
    pub id: Id,
    /// Label on the verb bar, e.g. "Pick up".
    pub label: String,
    /// Words and phrases the parser accepts, e.g. `take`, `pick up`, `get`.
    pub words: Vec<String>,
    #[serde(default)]
    pub slots: VerbSlots,
    /// Preposition between the two objects, e.g. `with`, `on`, `to`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preposition: Option<String>,
    /// Shown on the SCUMM-style verb bar. Parser-only verbs set this to false.
    #[serde(default = "yes")]
    pub bar: bool,
    /// Response when no rule handles the verb.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

fn yes() -> bool {
    true
}

/// The standard verb library. The runtime implements these by id; stories
/// add their own verbs, handled by rules.
pub fn standard_verbs() -> Vec<VerbDef> {
    let verb = |id: &str, label: &str, words: &[&str], slots, prep: Option<&str>, bar| VerbDef {
        id: Id::new(id),
        label: label.to_string(),
        words: words.iter().map(|w| w.to_string()).collect(),
        slots,
        preposition: prep.map(String::from),
        bar,
        default: None,
    };
    let (zero, one, two) = (VerbSlots::None, VerbSlots::One, VerbSlots::Two);
    vec![
        verb(
            "examine",
            "Look at",
            &[
                "examine", "x", "look at", "l at", "look", "l", "inspect", "read", "check",
            ],
            one,
            None,
            true,
        ),
        verb(
            "take",
            "Pick up",
            &["take", "get", "pick up", "grab", "collect"],
            one,
            None,
            true,
        ),
        verb("use", "Use", &["use"], two, Some("on"), true),
        verb("open", "Open", &["open"], one, None, true),
        verb("close", "Close", &["close", "shut"], one, None, true),
        verb(
            "talk",
            "Talk to",
            &["talk to", "talk", "speak to", "greet"],
            one,
            None,
            true,
        ),
        verb("give", "Give", &["give", "offer"], two, Some("to"), true),
        verb("push", "Push", &["push", "press", "move"], one, None, true),
        verb("pull", "Pull", &["pull", "yank", "tug"], one, None, true),
        verb("unlock", "Unlock", &["unlock"], two, Some("with"), false),
        verb("lock", "Lock", &["lock"], two, Some("with"), false),
        verb(
            "turn_on",
            "Turn on",
            &["turn on", "switch on", "light", "ignite"],
            one,
            None,
            false,
        ),
        verb(
            "turn_off",
            "Turn off",
            &["turn off", "switch off", "extinguish", "blow out", "douse"],
            one,
            None,
            false,
        ),
        verb(
            "drop",
            "Drop",
            &["drop", "put down", "discard"],
            one,
            None,
            false,
        ),
        verb(
            "go",
            "Walk to",
            &["go", "walk", "go to", "walk to", "enter", "climb"],
            one,
            None,
            false,
        ),
        verb(
            "look",
            "Look around",
            &["look", "l", "look around"],
            zero,
            None,
            true,
        ),
        verb(
            "inventory",
            "Inventory",
            &["inventory", "inv", "i"],
            zero,
            None,
            false,
        ),
        verb("wait", "Wait", &["wait", "z"], zero, None, false),
    ]
}
