use dramaturge_model::{Condition, FactValue};
use loremaster_model::Id;

/// Comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    pub fn as_str(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "==" => CmpOp::Eq,
            "!=" => CmpOp::Ne,
            "<" => CmpOp::Lt,
            "<=" => CmpOp::Le,
            ">" => CmpOp::Gt,
            ">=" => CmpOp::Ge,
            _ => return None,
        })
    }

    /// The operator with its operands swapped: `3 < x` is `x > 3`.
    pub fn flipped(self) -> Self {
        match self {
            CmpOp::Lt => CmpOp::Gt,
            CmpOp::Le => CmpOp::Ge,
            CmpOp::Gt => CmpOp::Lt,
            CmpOp::Ge => CmpOp::Le,
            other => other,
        }
    }
}

/// A value the runtime computes instead of storing: time, location, visits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Derived {
    /// `time.phase`, text.
    Phase,
    /// `time.hour`, int.
    Hour,
    /// `time.day`, int, starting at 1.
    Day,
    /// `turns`, int.
    Turns,
    /// `here`, the current room id.
    Here,
    /// `level`, int: the level of the current room. Up exits go one level higher.
    Level,
    /// `first_visit`: the current room is being seen for the first time.
    FirstVisit,
    /// `room.visits`, int.
    Visits(Id),
    /// `room.visited` or `visited(room)`.
    Visited(Id),
    /// `thing.carried` or `carried(thing)`.
    Carried(Id),
    /// `thing.present` or `present(thing)`: carried or in the current room.
    Present(Id),
    /// `thing.location`: a room id, `carried` or `nowhere`.
    Location(Id),
    /// `action.repeat`: how many times in a row the player gave the current command, this time included.
    ActionRepeat,
    /// `action.count`: how many times the player used the current verb in this room, this time included.
    ActionCount,
    /// `action.total`: how many times the player used the current verb in the game, this time included.
    ActionTotal,
    /// `action.verb`: the id of the current verb, text.
    ActionVerb,
    /// `thing.interactions`: how many commands named the thing.
    Interactions(Id),
    /// `thing.talked`: how many times the player talked to the thing.
    Talked(Id),
    /// `timer.id`, int: the turns left on a running timer, 0 when it does not run.
    Timer(Id),
    /// `timer.id.running`: the timer runs.
    TimerRunning(Id),
    /// `in(thing, holder)`: the thing is in or on the holder, also deeper inside.
    Inside(Id, Id),
    /// `chance(n)`: true in about n of 100 turns. The salt keeps two chances
    /// apart; the result is fixed for one state, so play replays.
    Chance(u8, u64),
}

/// The custom-condition key used for every derived comparison.
pub const CUSTOM_KEY: &str = "loremaster.derived";

impl Derived {
    fn name(&self) -> (&'static str, Option<&Id>) {
        match self {
            Derived::Phase => ("phase", None),
            Derived::Hour => ("hour", None),
            Derived::Day => ("day", None),
            Derived::Turns => ("turns", None),
            Derived::Here => ("here", None),
            Derived::Level => ("level", None),
            Derived::FirstVisit => ("first_visit", None),
            Derived::Visits(id) => ("visits", Some(id)),
            Derived::Visited(id) => ("visited", Some(id)),
            Derived::Carried(id) => ("carried", Some(id)),
            Derived::Present(id) => ("present", Some(id)),
            Derived::Location(id) => ("location", Some(id)),
            Derived::ActionRepeat => ("action_repeat", None),
            Derived::ActionCount => ("action_count", None),
            Derived::ActionTotal => ("action_total", None),
            Derived::ActionVerb => ("action_verb", None),
            Derived::Interactions(id) => ("interactions", Some(id)),
            Derived::Talked(id) => ("talked", Some(id)),
            Derived::Timer(id) => ("timer", Some(id)),
            Derived::TimerRunning(id) => ("timer_running", Some(id)),
            Derived::Chance(..) => ("chance", None),
            Derived::Inside(..) => ("inside", None),
        }
    }

    /// The argument of the encoded condition.
    fn arg(&self) -> String {
        match self {
            Derived::Chance(n, salt) => format!("{n}:{salt}"),
            Derived::Inside(thing, holder) => format!("{thing}:{holder}"),
            other => other.name().1.map(|a| a.to_string()).unwrap_or_default(),
        }
    }

    fn from_name(name: &str, arg: &str) -> Option<Self> {
        let id = || Id::new(arg);
        Some(match name {
            "phase" => Derived::Phase,
            "hour" => Derived::Hour,
            "day" => Derived::Day,
            "turns" => Derived::Turns,
            "here" => Derived::Here,
            "level" => Derived::Level,
            "first_visit" => Derived::FirstVisit,
            "visits" => Derived::Visits(id()),
            "visited" => Derived::Visited(id()),
            "carried" => Derived::Carried(id()),
            "present" => Derived::Present(id()),
            "location" => Derived::Location(id()),
            "action_repeat" => Derived::ActionRepeat,
            "action_count" => Derived::ActionCount,
            "action_total" => Derived::ActionTotal,
            "action_verb" => Derived::ActionVerb,
            "interactions" => Derived::Interactions(id()),
            "talked" => Derived::Talked(id()),
            "timer" => Derived::Timer(id()),
            "timer_running" => Derived::TimerRunning(id()),
            "inside" => {
                let (thing, holder) = arg.split_once(':')?;
                Derived::Inside(Id::new(thing), Id::new(holder))
            }
            "chance" => {
                let (n, salt) = arg.split_once(':')?;
                Derived::Chance(n.parse().ok()?, salt.parse().ok()?)
            }
            _ => return None,
        })
    }

    /// Encode `self <op> value` as a dramaturge `Custom` condition.
    pub fn to_condition(&self, op: CmpOp, value: FactValue) -> Condition {
        let (name, _) = self.name();
        Condition::Custom {
            key: CUSTOM_KEY.to_string(),
            args: vec![
                FactValue::Text(name.to_string()),
                FactValue::Text(self.arg()),
                FactValue::Text(op.as_str().to_string()),
                value,
            ],
        }
    }

    /// Decode a condition made by [`Derived::to_condition`].
    pub fn from_condition(key: &str, args: &[FactValue]) -> Option<(Derived, CmpOp, FactValue)> {
        if key != CUSTOM_KEY {
            return None;
        }
        let [
            FactValue::Text(name),
            FactValue::Text(arg),
            FactValue::Text(op),
            value,
        ] = args
        else {
            return None;
        };
        Some((
            Derived::from_name(name, arg)?,
            CmpOp::parse(op)?,
            value.clone(),
        ))
    }
}

/// Something whose value can be read: a stored fact, a derived value, or a
/// thing's display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    /// A world fact: a story variable `x` or a thing property `lamp.lit`.
    Fact(String),
    Derived(Derived),
    /// The current name of a thing, in a form.
    ThingName(Id, NameForm),
    /// The player's relation to a thing on an axis: `blackbeard.trust`.
    Relation(Id, String),
}

/// How to write a thing's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameForm {
    /// `{lamp.name}`: "oil lamp".
    Bare,
    /// `{lamp.the}`: "the oil lamp", or "Blackbeard" for a proper name.
    The,
    /// `{lamp.The}`: "The oil lamp".
    TheCap,
    /// `{lamp.a}`: "an oil lamp", or "Blackbeard" for a proper name.
    A,
    /// `{lamp.A}`: "An oil lamp".
    ACap,
}
