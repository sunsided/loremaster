//! Events: what happens in play. Rules (and behaviors) react to them.

use crate::CommandRecord;
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
    /// The other slots of the Command record (D6) are in `actor`,
    /// `preposition` and `topic`; [`Event::command`] gives the whole record.
    Verb {
        phase: Phase,
        verb: Id,
        objects: Vec<Id>,
        /// Who acts: `player`, or an actor thing that runs an agenda step.
        actor: Id,
        preposition: Option<String>,
        topic: Option<String>,
    },
    /// A timer runs out.
    Timer(Id),
    /// An `emit` effect.
    Custom(Id),
    /// A `call` effect runs the behavior. The id is the owner of the effect:
    /// the rule's id, or `dialogue:<id>` / `timer:<id>`.
    Call(Id),
    /// A dialogue reaches a node.
    Talk {
        dialogue: Id,
        node: String,
    },
}

/// The kind of an event, without its data. A behavior names the kinds it
/// handles in its [`Manifest`](crate::Manifest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EventKind {
    Start,
    Turn,
    Enter,
    Leave,
    Time,
    Verb,
    Timer,
    Custom,
    Call,
    Talk,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Start => EventKind::Start,
            Event::Turn => EventKind::Turn,
            Event::Enter(_) => EventKind::Enter,
            Event::Leave(_) => EventKind::Leave,
            Event::Time { .. } => EventKind::Time,
            Event::Verb { .. } => EventKind::Verb,
            Event::Timer(_) => EventKind::Timer,
            Event::Custom(_) => EventKind::Custom,
            Event::Call(_) => EventKind::Call,
            Event::Talk { .. } => EventKind::Talk,
        }
    }

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
            (Event::Verb { phase, verb, .. }, pattern) => {
                let wanted = match (phase, pattern) {
                    (Phase::Check, EventPattern::Check(p)) => p,
                    (Phase::Instead, EventPattern::Verb(p)) if rule.instead => p,
                    (Phase::After, EventPattern::Verb(p)) if !rule.instead => p,
                    _ => return false,
                };
                // The verb first: most rules differ there, without a clone.
                wanted.verb == *verb && self.command().is_some_and(|c| c.matches(wanted))
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

    /// The Command record of a verb event.
    pub fn command(&self) -> Option<CommandRecord> {
        match self {
            Event::Verb {
                verb,
                objects,
                actor,
                preposition,
                topic,
                ..
            } => Some(CommandRecord {
                actor: actor.clone(),
                verb: verb.clone(),
                direct: objects.first().cloned(),
                indirect: objects.get(1).cloned(),
                preposition: preposition.clone(),
                topic: topic.clone(),
            }),
            _ => None,
        }
    }

    /// The verb event of a command in one phase.
    pub(crate) fn of_command(phase: Phase, c: &CommandRecord) -> Self {
        Event::Verb {
            phase,
            verb: c.verb.clone(),
            objects: c.objects(),
            actor: c.actor.clone(),
            preposition: c.preposition.clone(),
            topic: c.topic.clone(),
        }
    }
}
