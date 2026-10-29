use dramaturge_model::{Condition, FactValue};
use lantern_model::Id;

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
}

/// The custom-condition key used for every derived comparison.
pub const CUSTOM_KEY: &str = "lantern.derived";

impl Derived {
    fn name(&self) -> (&'static str, Option<&Id>) {
        match self {
            Derived::Phase => ("phase", None),
            Derived::Hour => ("hour", None),
            Derived::Day => ("day", None),
            Derived::Turns => ("turns", None),
            Derived::Here => ("here", None),
            Derived::FirstVisit => ("first_visit", None),
            Derived::Visits(id) => ("visits", Some(id)),
            Derived::Visited(id) => ("visited", Some(id)),
            Derived::Carried(id) => ("carried", Some(id)),
            Derived::Present(id) => ("present", Some(id)),
            Derived::Location(id) => ("location", Some(id)),
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
            "first_visit" => Derived::FirstVisit,
            "visits" => Derived::Visits(id()),
            "visited" => Derived::Visited(id()),
            "carried" => Derived::Carried(id()),
            "present" => Derived::Present(id()),
            "location" => Derived::Location(id()),
            _ => return None,
        })
    }

    /// Encode `self <op> value` as a dramaturge `Custom` condition.
    pub fn to_condition(&self, op: CmpOp, value: FactValue) -> Condition {
        let (name, arg) = self.name();
        Condition::Custom {
            key: CUSTOM_KEY.to_string(),
            args: vec![
                FactValue::Text(name.to_string()),
                FactValue::Text(arg.map(|a| a.to_string()).unwrap_or_default()),
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
    ThingName(Id),
}
