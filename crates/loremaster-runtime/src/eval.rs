use crate::{State, World};
use dramaturge_model::{Condition, FactValue};
use dramaturge_runtime::{ConditionEvaluator, DefaultConditionEvaluator, DialogWorldState};
use loremaster_lang::{CmpOp, Derived, NameForm, Operand};
use loremaster_model::{Location, TimeOfDay, Value};

/// Evaluates conditions against a game: stored facts through dramaturge's
/// default evaluator, derived values here.
///
/// dramaturge's evaluator takes `DialogWorldState` only and answers `Custom`
/// with `false`, so this walks `All` / `Any` / `Not` itself and hands every
/// other leaf to dramaturge. (See the dramaturge issue on making evaluation
/// generic over state; once that lands this becomes a `Custom` handler.)
pub(crate) struct Eval<'a> {
    pub world: &'a World,
    pub state: &'a State,
    pub facts: &'a DialogWorldState,
}

impl Eval<'_> {
    pub fn holds(&self, c: &Condition) -> bool {
        match c {
            Condition::All(items) => items.iter().all(|c| self.holds(c)),
            Condition::Any(items) => items.iter().any(|c| self.holds(c)),
            Condition::Not(inner) => !self.holds(inner),
            Condition::Custom { key, args } => match Derived::from_condition(key, args) {
                Some((d, op, value)) => compare(&self.derived(&d), op, &value),
                None => false,
            },
            leaf => DefaultConditionEvaluator.evaluate(leaf, self.facts),
        }
    }

    pub fn holds_opt(&self, c: &Option<Condition>) -> bool {
        c.as_ref().is_none_or(|c| self.holds(c))
    }

    pub fn time_of_day(&self) -> TimeOfDay {
        TimeOfDay((self.state.minutes % TimeOfDay::MINUTES_PER_DAY) as u16)
    }

    pub fn phase(&self) -> String {
        self.world
            .story
            .clock
            .phase_at(self.time_of_day())
            .unwrap_or_default()
            .to_string()
    }

    pub fn derived(&self, d: &Derived) -> FactValue {
        let s = self.state;
        match d {
            Derived::Phase => FactValue::Text(self.phase()),
            Derived::Hour => FactValue::Int(i64::from(self.time_of_day().0 / 60)),
            Derived::Day => FactValue::Int(i64::from(s.minutes / TimeOfDay::MINUTES_PER_DAY) + 1),
            Derived::Turns => FactValue::Int(i64::from(s.turns)),
            Derived::Here => FactValue::Text(s.room.to_string()),
            Derived::Level => FactValue::Int(self.world.level_of(s.room.as_str()).into()),
            Derived::FirstVisit => FactValue::Bool(s.visits_to(s.room.as_str()) <= 1),
            Derived::Visits(r) => FactValue::Int(i64::from(s.visits_to(r.as_str()))),
            Derived::Visited(r) => FactValue::Bool(s.visits_to(r.as_str()) > 0),
            Derived::Carried(t) => FactValue::Bool(s.is_carried(t.as_str())),
            Derived::Present(t) => FactValue::Bool(
                s.is_carried(t.as_str()) || s.is_in_room(t.as_str(), s.room.as_str()),
            ),
            Derived::ActionRepeat => FactValue::Int(i64::from(s.repeat)),
            Derived::ActionCount => FactValue::Int(i64::from(
                s.action
                    .as_ref()
                    .and_then(|a| s.action_counts.get(&format!("{a}@{}", s.room)))
                    .copied()
                    .unwrap_or(0),
            )),
            Derived::ActionTotal => FactValue::Int(i64::from(
                s.action
                    .as_ref()
                    .and_then(|a| s.action_counts.get(a.as_str()))
                    .copied()
                    .unwrap_or(0),
            )),
            Derived::ActionVerb => {
                FactValue::Text(s.action.as_ref().map(|a| a.to_string()).unwrap_or_default())
            }
            Derived::Interactions(t) => {
                FactValue::Int(i64::from(s.interactions.get(t).copied().unwrap_or(0)))
            }
            Derived::Talked(t) => FactValue::Int(i64::from(s.talked.get(t).copied().unwrap_or(0))),
            Derived::Location(t) => FactValue::Text(match s.location_of(t.as_str()) {
                Some(Location::Room(r)) => r.to_string(),
                Some(Location::Carried) => "carried".into(),
                _ => "nowhere".into(),
            }),
        }
    }

    /// The index of the thing's current name stage: the last one whose condition holds.
    /// None when the thing has no stages.
    pub fn name_stage(&self, id: &str) -> Option<usize> {
        let ti = self.world.thing_idx(id)?;
        let stages = &self.world.things[ti].names;
        if stages.is_empty() {
            return None;
        }
        Some(stages.iter().rposition(|c| self.holds_opt(c)).unwrap_or(0))
    }

    pub fn is_proper(&self, id: &str) -> bool {
        let Some(t) = self.world.story.thing(id) else {
            return false;
        };
        match self.name_stage(id) {
            Some(i) => t.names[i].proper,
            None => t.proper,
        }
    }

    /// The current name of a thing in a form, with the right article.
    pub fn thing_name(&self, id: &str, form: NameForm) -> String {
        let Some(t) = self.world.story.thing(id) else {
            return id.to_string();
        };
        let name = match self.name_stage(id) {
            Some(i) => t.names[i].text.clone(),
            None => t.name.clone(),
        };
        let proper = self.is_proper(id);
        let with = |article: &str| {
            if proper {
                name.clone()
            } else {
                format!("{article} {name}")
            }
        };
        let cap = |s: String| {
            let mut c = s.chars();
            c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        };
        match form {
            NameForm::Bare => name.clone(),
            NameForm::The => with("the"),
            NameForm::TheCap => cap(with("the")),
            NameForm::A => with(crate::text::article(&name)),
            NameForm::ACap => cap(with(crate::text::article(&name))),
        }
    }

    pub fn operand_text(&self, op: &Operand) -> String {
        match op {
            Operand::Fact(id) => self
                .state
                .facts
                .get(id)
                .map(Value::to_string)
                .unwrap_or_default(),
            Operand::Derived(d) => match self.derived(d) {
                FactValue::Text(t) => t,
                FactValue::Int(i) => i.to_string(),
                FactValue::Bool(b) => b.to_string(),
                other => format!("{other:?}"),
            },
            Operand::ThingName(id, form) => self.thing_name(id.as_str(), *form),
        }
    }
}

fn compare(actual: &FactValue, op: CmpOp, expected: &FactValue) -> bool {
    match (actual, expected) {
        (FactValue::Int(a), FactValue::Int(b)) => match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        },
        (a, b) => match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            _ => false,
        },
    }
}

pub(crate) fn to_fact_value(v: &Value) -> FactValue {
    match v {
        Value::Bool(b) => FactValue::Bool(*b),
        Value::Int(i) => FactValue::Int(*i),
        Value::Text(t) => FactValue::Text(t.clone()),
    }
}
