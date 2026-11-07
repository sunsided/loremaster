//! Conversions for the arguments of the builders, so that a story reads with
//! plain literals: `false`, `"night"`, `"20:30"`, `["rung", "lit"]`.

use loremaster_model::{Direction, Id, States, TimeOfDay, Value};

/// A variable or property value: `bool`, an integer, or text (an enum value).
pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::Int(self)
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::Int(self.into())
    }
}

impl IntoValue for u32 {
    fn into_value(self) -> Value {
        Value::Int(self.into())
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Text(self.to_string())
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Text(self)
    }
}

/// A direction: [`Direction::South`], or a word such as `"south"`, `"s"` or a
/// named exit such as `"through the mirror"`.
pub trait IntoDirection {
    fn into_direction(self) -> Direction;
}

impl IntoDirection for Direction {
    fn into_direction(self) -> Direction {
        self
    }
}

impl IntoDirection for &str {
    fn into_direction(self) -> Direction {
        Direction::from(self.to_string())
    }
}

impl IntoDirection for String {
    fn into_direction(self) -> Direction {
        Direction::from(self)
    }
}

/// A time of day: a [`TimeOfDay`], or `"HH:MM"`. A malformed text is a bug in
/// the story code, so it panics at the line that wrote it.
pub trait IntoTime {
    #[track_caller]
    fn into_time(self) -> TimeOfDay;
}

impl IntoTime for TimeOfDay {
    fn into_time(self) -> TimeOfDay {
        self
    }
}

impl IntoTime for &str {
    #[track_caller]
    fn into_time(self) -> TimeOfDay {
        match self.parse() {
            Ok(time) => time,
            Err(e) => panic!("not a time of day: {e}"),
        }
    }
}

/// The `from` states of a machine step: one state (`"idle"`) or a list
/// (`["rung", "lit"]`). They give the same model as `from: idle` and
/// `from: [rung, lit]` in YAML.
pub trait IntoStates {
    fn into_states(self) -> States;
}

impl IntoStates for States {
    fn into_states(self) -> States {
        self
    }
}

impl IntoStates for &str {
    fn into_states(self) -> States {
        States::One(self.to_string())
    }
}

impl IntoStates for String {
    fn into_states(self) -> States {
        States::One(self)
    }
}

impl<const N: usize> IntoStates for [&str; N] {
    fn into_states(self) -> States {
        States::Many(self.iter().map(|s| s.to_string()).collect())
    }
}

impl IntoStates for Vec<String> {
    fn into_states(self) -> States {
        States::Many(self)
    }
}

pub(crate) fn id(id: impl AsRef<str>) -> Id {
    Id::new(id.as_ref())
}

pub(crate) fn ids(list: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Id> {
    list.into_iter().map(id).collect()
}

pub(crate) fn strings(list: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    list.into_iter().map(|s| s.as_ref().to_string()).collect()
}
