use crate::{CmpOp, Derived, Expr, LangError, NameForm, Operand, Scope, Span, Ty, parse_expr};
use dramaturge_model::{Condition, FactId, FactScope, FactValue};
use lantern_model::Id;

/// Parse, type-check and lower a condition to a dramaturge condition tree.
pub fn compile_condition(src: &str, scope: &Scope) -> Result<Condition, LangError> {
    let expr = parse_expr(src)?;
    lower_bool(&expr, scope)
}

pub(crate) fn lower_bool(expr: &Expr, scope: &Scope) -> Result<Condition, LangError> {
    match expr {
        Expr::Bool(true, _) => Ok(Condition::Always),
        Expr::Bool(false, _) => Ok(Condition::Never),
        Expr::Not(inner, _) => Ok(Condition::negate(lower_bool(inner, scope)?)),
        Expr::And(items, _) => Ok(Condition::all(
            items
                .iter()
                .map(|e| lower_bool(e, scope))
                .collect::<Result<_, _>>()?,
        )),
        Expr::Or(items, _) => Ok(Condition::any(
            items
                .iter()
                .map(|e| lower_bool(e, scope))
                .collect::<Result<_, _>>()?,
        )),
        Expr::Call(name, args, span) if name == "in" => {
            let room = room_arg(args, span, scope)?;
            Ok(Derived::Here.to_condition(CmpOp::Eq, FactValue::Text(room.to_string())))
        }
        // A phase name on its own, such as `night`, tests the time of day.
        Expr::Path(parts, _)
            if parts.len() == 1
                && scope.phases.contains(&parts[0])
                && !scope.vars.contains_key(&parts[0]) =>
        {
            Ok(Derived::Phase.to_condition(CmpOp::Eq, FactValue::Text(parts[0].clone())))
        }
        Expr::Path(..) | Expr::Call(..) => {
            let (operand, ty) = resolve(expr, scope)?;
            if ty != Ty::Bool {
                return Err(LangError::new(
                    format!(
                        "this is {}, not a flag; compare it with something",
                        ty.describe()
                    ),
                    expr.span(),
                ));
            }
            Ok(compare(&operand, CmpOp::Eq, FactValue::Bool(true)))
        }
        Expr::Cmp(lhs, op, rhs, _) => {
            let (operand, ty, op, literal) = match resolve(lhs, scope) {
                Ok((operand, ty)) => (operand, ty, *op, rhs.as_ref()),
                Err(err) if is_literal(lhs) => {
                    let (operand, ty) = resolve(rhs, scope).map_err(|_| err)?;
                    (operand, ty, op.flipped(), lhs.as_ref())
                }
                Err(err) => return Err(err),
            };
            let value = literal_value(literal, &ty, scope)?;
            if matches!(value, FactValue::Text(_) | FactValue::Bool(_))
                && !matches!(op, CmpOp::Eq | CmpOp::Ne)
            {
                return Err(LangError::new(
                    format!("`{}` only works on numbers", op.as_str()),
                    expr.span(),
                ));
            }
            Ok(compare(&operand, op, value))
        }
        Expr::Int(_, span) | Expr::Str(_, span) => Err(LangError::new(
            "a value on its own is not a condition",
            span.clone(),
        )),
    }
}

fn is_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Int(..) | Expr::Str(..) | Expr::Bool(..))
        || matches!(expr, Expr::Path(parts, _) if parts.len() == 1)
}

fn compare(operand: &Operand, op: CmpOp, value: FactValue) -> Condition {
    match operand {
        Operand::Derived(d) => d.to_condition(op, value),
        Operand::ThingName(..) => Condition::Never,
        Operand::Fact(id) => {
            let fact = |value| Condition::FactEquals {
                scope: FactScope::World,
                fact_id: FactId::new(id.as_str()),
                value,
            };
            let at_least = |value| Condition::FactAtLeast {
                scope: FactScope::World,
                fact_id: FactId::new(id.as_str()),
                value,
            };
            let at_most = |value| Condition::FactAtMost {
                scope: FactScope::World,
                fact_id: FactId::new(id.as_str()),
                value,
            };
            match (op, value) {
                (CmpOp::Eq, v) => fact(v),
                (CmpOp::Ne, v) => Condition::negate(fact(v)),
                (CmpOp::Ge, FactValue::Int(n)) => at_least(n),
                (CmpOp::Gt, FactValue::Int(n)) => at_least(n.saturating_add(1)),
                (CmpOp::Le, FactValue::Int(n)) => at_most(n),
                (CmpOp::Lt, FactValue::Int(n)) => at_most(n.saturating_sub(1)),
                _ => Condition::Never,
            }
        }
    }
}

/// Resolve a readable name to an operand and its type.
pub(crate) fn resolve(expr: &Expr, scope: &Scope) -> Result<(Operand, Ty), LangError> {
    match expr {
        Expr::Path(parts, span) => resolve_path(parts, span, scope),
        Expr::Call(name, args, span) => {
            let d = match name.as_str() {
                "visited" => Derived::Visited(room_arg(args, span, scope)?),
                "visits" => Derived::Visits(room_arg(args, span, scope)?),
                "carried" => Derived::Carried(thing_arg(args, span, scope)?),
                "present" => Derived::Present(thing_arg(args, span, scope)?),
                other => {
                    return Err(LangError::new(
                        format!(
                            "unknown function `{other}`; try visited, visits, carried, present or in"
                        ),
                        span.clone(),
                    ));
                }
            };
            let ty = if matches!(d, Derived::Visits(_)) {
                Ty::Int
            } else {
                Ty::Bool
            };
            Ok((Operand::Derived(d), ty))
        }
        other => Err(LangError::new("expected a name", other.span())),
    }
}

fn resolve_path(parts: &[String], span: &Span, scope: &Scope) -> Result<(Operand, Ty), LangError> {
    let derived = |d, ty| Ok((Operand::Derived(d), ty));
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    match parts.as_slice() {
        ["time", "phase"] => derived(Derived::Phase, Ty::Enum(scope.phases.clone())),
        ["time", "hour"] => derived(Derived::Hour, Ty::Int),
        ["time", "day"] => derived(Derived::Day, Ty::Int),
        ["turns"] => derived(Derived::Turns, Ty::Int),
        ["here"] => derived(Derived::Here, Ty::Room),
        ["first_visit"] => derived(Derived::FirstVisit, Ty::Bool),
        ["action", "repeat"] => derived(Derived::ActionRepeat, Ty::Int),
        ["action", "count"] => derived(Derived::ActionCount, Ty::Int),
        ["action", "total"] => derived(Derived::ActionTotal, Ty::Int),
        ["action", "verb"] => derived(Derived::ActionVerb, Ty::Text),
        [var] if scope.vars.contains_key(*var) => {
            Ok((Operand::Fact(var.to_string()), scope.vars[*var].clone()))
        }
        [room, prop] if scope.rooms.contains(*room) => {
            let id = Id::new(*room);
            match *prop {
                "visits" => derived(Derived::Visits(id), Ty::Int),
                "visited" => derived(Derived::Visited(id), Ty::Bool),
                _ => Err(LangError::new(
                    format!("rooms have `visits` and `visited`, not `{prop}`"),
                    span.clone(),
                )),
            }
        }
        [thing, prop] if scope.things.contains_key(*thing) => {
            let id = Id::new(*thing);
            match *prop {
                "carried" => derived(Derived::Carried(id), Ty::Bool),
                "present" => derived(Derived::Present(id), Ty::Bool),
                "location" => derived(Derived::Location(id), Ty::Room),
                "name" => Ok((Operand::ThingName(id, NameForm::Bare), Ty::Text)),
                "the" => Ok((Operand::ThingName(id, NameForm::The), Ty::Text)),
                "The" => Ok((Operand::ThingName(id, NameForm::TheCap), Ty::Text)),
                "a" => Ok((Operand::ThingName(id, NameForm::A), Ty::Text)),
                "A" => Ok((Operand::ThingName(id, NameForm::ACap), Ty::Text)),
                "interactions" => derived(Derived::Interactions(id), Ty::Int),
                "talked" => derived(Derived::Talked(id), Ty::Int),
                _ => match scope.things[*thing].get(*prop) {
                    Some(ty) => Ok((Operand::Fact(format!("{thing}.{prop}")), ty.clone())),
                    None => {
                        let props = &scope.things[*thing];
                        let known = [
                            "name", "the", "The", "a", "A", "carried", "present", "location",
                        ];
                        let hint = props
                            .keys()
                            .map(String::as_str)
                            .chain(known)
                            .map(|p| (p, crate::scope::edit_distance(p, prop)))
                            .filter(|(_, d)| *d <= 2)
                            .min_by_key(|(_, d)| *d)
                            .map(|(p, _)| format!("; did you mean `{thing}.{p}`?"))
                            .unwrap_or_default();
                        Err(LangError::new(
                            format!("`{thing}` has no property `{prop}`{hint}"),
                            span.clone(),
                        ))
                    }
                },
            }
        }
        [first, ..] => {
            let hint = scope
                .suggest(first)
                .map(|s| format!("; did you mean `{s}`?"))
                .unwrap_or_default();
            Err(LangError::new(
                format!("unknown name `{}`{hint}", parts.join(".")),
                span.clone(),
            ))
        }
        [] => Err(LangError::new("empty name", span.clone())),
    }
}

fn single_ident<'a>(args: &'a [Expr], span: &Span) -> Result<&'a str, LangError> {
    match args {
        [Expr::Path(parts, _)] if parts.len() == 1 => Ok(&parts[0]),
        _ => Err(LangError::new(
            "expected one name in parentheses",
            span.clone(),
        )),
    }
}

fn room_arg(args: &[Expr], span: &Span, scope: &Scope) -> Result<Id, LangError> {
    let name = single_ident(args, span)?;
    if scope.rooms.contains(name) {
        Ok(Id::new(name))
    } else {
        Err(LangError::new(
            format!("unknown room `{name}`"),
            args[0].span(),
        ))
    }
}

fn thing_arg(args: &[Expr], span: &Span, scope: &Scope) -> Result<Id, LangError> {
    let name = single_ident(args, span)?;
    if scope.things.contains_key(name) {
        Ok(Id::new(name))
    } else {
        Err(LangError::new(
            format!("unknown thing `{name}`"),
            args[0].span(),
        ))
    }
}

fn literal_value(expr: &Expr, ty: &Ty, scope: &Scope) -> Result<FactValue, LangError> {
    let mismatch = |found: &str| {
        LangError::new(
            format!("expected {}, found {found}", ty.describe()),
            expr.span(),
        )
    };
    match (expr, ty) {
        (Expr::Int(n, _), Ty::Int) => Ok(FactValue::Int(*n)),
        (Expr::Bool(b, _), Ty::Bool) => Ok(FactValue::Bool(*b)),
        (Expr::Str(s, _), Ty::Text) => Ok(FactValue::Text(s.clone())),
        (Expr::Path(parts, _), Ty::Enum(values)) if parts.len() == 1 => {
            if values.contains(&parts[0]) {
                Ok(FactValue::Text(parts[0].clone()))
            } else {
                Err(mismatch(&format!("`{}`", parts[0])))
            }
        }
        (Expr::Str(s, _), Ty::Enum(values)) if values.contains(s) => Ok(FactValue::Text(s.clone())),
        (Expr::Path(parts, _), Ty::Room) if parts.len() == 1 => {
            let name = &parts[0];
            if scope.rooms.contains(name) || name == "carried" || name == "nowhere" {
                Ok(FactValue::Text(name.clone()))
            } else {
                Err(LangError::new(
                    format!("unknown room `{name}`"),
                    expr.span(),
                ))
            }
        }
        (Expr::Int(..), _) => Err(mismatch("a number")),
        (Expr::Bool(..), _) => Err(mismatch("true or false")),
        (Expr::Str(..), _) => Err(mismatch("text")),
        _ => Err(LangError::new(
            "compare against a fixed value, such as a number or an enum value",
            expr.span(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use lantern_model::*;

    fn scope() -> Scope {
        let story: Story = serde_json::from_value(serde_json::json!({
            "title": "t",
            "start": "hall",
            "vars": [
                {"id": "sanity", "type": "int", "initial": 5},
                {"id": "weather", "type": {"enum": ["clear", "fog"]}, "initial": "clear"},
                {"id": "flooded", "type": "bool", "initial": false}
            ],
            "rooms": [{"id": "hall", "name": "Hall"}, {"id": "cellar", "name": "Cellar"}],
            "things": [{"id": "lamp", "name": "lamp", "location": {"room": "hall"}, "props": {"lit": false}}]
        }))
        .unwrap();
        Scope::from_story(&story)
    }

    #[test]
    fn flags_and_props_become_facts() {
        let c = compile_condition("flooded and not lamp.lit", &scope()).unwrap();
        assert!(let Condition::All(items) = c);
        check!(
            items[0]
                == Condition::FactEquals {
                    scope: FactScope::World,
                    fact_id: FactId::new("flooded"),
                    value: FactValue::Bool(true)
                }
        );
        check!(matches!(&items[1], Condition::Not(_)));
    }

    #[test]
    fn integer_comparisons_use_at_least_and_at_most() {
        let c = compile_condition("sanity > 2", &scope()).unwrap();
        check!(matches!(c, Condition::FactAtLeast { value: 3, .. }));
        let c = compile_condition("2 > sanity", &scope()).unwrap();
        check!(matches!(c, Condition::FactAtMost { value: 1, .. }));
    }

    #[test]
    fn derived_values_become_custom_conditions() {
        let c = compile_condition("time.phase == night", &scope()).unwrap();
        assert!(let Condition::Custom { key, args } = c);
        let decoded = Derived::from_condition(&key, &args).unwrap();
        check!(decoded == (Derived::Phase, CmpOp::Eq, FactValue::Text("night".into())));
    }

    #[test]
    fn action_counters_and_thing_counters_are_derived() {
        let c = compile_condition("action.repeat >= 2 and lamp.talked == 0", &scope()).unwrap();
        assert!(let Condition::All(items) = c);
        check!(matches!(&items[0], Condition::Custom { .. }));
        check!(matches!(&items[1], Condition::Custom { .. }));
    }

    #[test]
    fn type_errors_are_reported() {
        let err = compile_condition("weather == sunny", &scope()).unwrap_err();
        check!(err.message == "expected one of clear, fog, found `sunny`");
        let err = compile_condition("flodded", &scope()).unwrap_err();
        check!(err.message == "unknown name `flodded`; did you mean `flooded`?");
        let err = compile_condition("sanity", &scope()).unwrap_err();
        check!(err.message.starts_with("this is a number"));
        let err = compile_condition("in(attic)", &scope()).unwrap_err();
        check!(err.span == (3..8));
    }
}
