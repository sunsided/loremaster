//! Events: what happens in play. Rules (and behaviors) react to them.

use loremaster_model::{EventPattern, Id, Rule};

/// The phase of an action. A verb runs `check`, then `instead` or the
/// default action, then `after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Before the action. A rule with `stop` refuses it.
    Check,
    /// Replaces the default action.
    Instead,
    /// After the default action.
    After,
}

/// Something that happens in play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Start,
    /// A turn passes, after the command.
    Turn,
    Enter(Id),
    Leave(Id),
    /// The clock moves from `before` to `after`, in minutes since day one.
    Time {
        before: u32,
        after: u32,
    },
    /// A verb in one of its phases. For `go`, the object is the direction.
    Verb {
        phase: Phase,
        verb: Id,
        objects: Vec<Id>,
    },
    /// A timer runs out.
    Timer(Id),
    /// An `emit` effect.
    Custom(Id),
    /// A dialogue reaches a node.
    Talk {
        dialogue: Id,
        node: String,
    },
}

impl Event {
    /// Does the rule listen for this event? Scope (`within`) and the
    /// condition are not part of this test.
    pub fn matches(&self, rule: &Rule) -> bool {
        match (self, &rule.on) {
            (Event::Start, EventPattern::Start) | (Event::Turn, EventPattern::Turn) => true,
            (Event::Enter(a), EventPattern::Enter(b))
            | (Event::Leave(a), EventPattern::Leave(b)) => a == b,
            (Event::Time { before, after }, EventPattern::Time(t)) => {
                crate::game::crossed(*before, *after, *t)
            }
            (
                Event::Verb {
                    phase,
                    verb,
                    objects,
                },
                pattern,
            ) => {
                let parts = match (phase, pattern) {
                    (Phase::Check, EventPattern::Check(parts)) => parts,
                    (Phase::Instead, EventPattern::Verb(parts)) if rule.instead => parts,
                    (Phase::After, EventPattern::Verb(parts)) if !rule.instead => parts,
                    _ => return false,
                };
                verb_matches(parts, verb, objects)
            }
            (Event::Timer(a), EventPattern::Timer(b))
            | (Event::Custom(a), EventPattern::Event(b)) => a == b,
            (Event::Talk { dialogue, node }, EventPattern::Talk(parts)) => {
                parts.first() == Some(dialogue) && parts.get(1).is_none_or(|n| n.as_str() == node)
            }
            _ => false,
        }
    }

    /// The things that the event is about, for rules that belong to a thing.
    pub fn objects(&self) -> Option<&[Id]> {
        match self {
            Event::Verb { objects, .. } => Some(objects),
            _ => None,
        }
    }
}

/// `[verb, thing, other]` matches a verb and its objects; fewer parts match more.
fn verb_matches(parts: &[Id], verb: &Id, objects: &[Id]) -> bool {
    parts.first() == Some(verb)
        && parts.len() - 1 <= objects.len()
        && parts[1..].iter().zip(objects.iter()).all(|(a, b)| a == b)
}
