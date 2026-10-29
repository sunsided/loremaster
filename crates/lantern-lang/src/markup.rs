use crate::lower::{lower_bool, resolve};
use crate::{CmpOp, Derived, LangError, Operand, Scope, parse_expr};
use dramaturge_model::{Condition, FactValue};

/// One piece of compiled fragment text.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Text(String),
    /// `{cond: then | otherwise}` or `{cond ? then : otherwise}`.
    Choice {
        condition: Condition,
        /// The condition as written, for the debugger.
        source: String,
        then: Vec<Segment>,
        otherwise: Vec<Segment>,
        /// The markup around the branches as written, for the editor.
        syntax: ChoiceSyntax,
    },
    /// `{weather}`, `{lamp.name}`.
    Insert {
        operand: Operand,
        /// The markup as written, for example `{time.phase}`.
        source: String,
    },
}

/// The written markup of a choice, so the editor can show the source with
/// each branch marked as active or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceSyntax {
    /// The opening brace, the condition and its separator, for example `{night:`.
    pub open: String,
    /// The separator between the branches, for example `|`. None when there is no second branch.
    pub separator: Option<String>,
    /// The closing brace.
    pub close: String,
}

/// Compile fragment text with inline markup.
///
/// * `{night: moonlit | sunlit}`: a phase name on its own tests `time.phase`.
/// * `{first: , which you have never seen before,}`: only on the first visit.
/// * `{lamp.lit ? glowing : dark}`: any condition, with `?` and `:` or `|`.
/// * `{weather}`, `{lamp.name}`: insert a value.
/// * `{{` and `}}` print literal braces.
pub fn compile_markup(src: &str, scope: &Scope) -> Result<Vec<Segment>, LangError> {
    compile_at(src, 0, scope)
}

fn compile_at(src: &str, base: usize, scope: &Scope) -> Result<Vec<Segment>, LangError> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut chars = src.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' if src[i + 1..].starts_with('{') => {
                chars.next();
                text.push('{');
            }
            '}' if src[i + 1..].starts_with('}') => {
                chars.next();
                text.push('}');
            }
            '{' => {
                let end = matching_brace(src, i).ok_or_else(|| {
                    LangError::new("`{` is never closed", base + i..base + src.len())
                })?;
                if !text.is_empty() {
                    out.push(Segment::Text(std::mem::take(&mut text)));
                }
                let inner = &src[i + 1..end];
                out.push(compile_inner(inner, base + i + 1, scope)?);
                while chars.peek().is_some_and(|(j, _)| *j <= end) {
                    chars.next();
                }
            }
            '}' => return Err(LangError::new("`}` without `{`", base + i..base + i + 1)),
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        out.push(Segment::Text(text));
    }
    Ok(out)
}

fn matching_brace(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Top-level positions of `chars` in `src`, ignoring nested braces.
fn find_top(src: &str, chars: &[char]) -> Option<(usize, char)> {
    let mut depth = 0;
    for (i, c) in src.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if depth == 0 && chars.contains(&c) => return Some((i, c)),
            _ => {}
        }
    }
    None
}

fn compile_inner(inner: &str, base: usize, scope: &Scope) -> Result<Segment, LangError> {
    let Some((sep, kind)) = find_top(inner, &['?', ':']) else {
        let expr = parse_expr(inner).map_err(|e| e.offset(base))?;
        let (operand, _) = resolve(&expr, scope).map_err(|e| e.offset(base))?;
        return Ok(Segment::Insert {
            operand,
            source: format!("{{{inner}}}"),
        });
    };
    let head = &inner[..sep];
    let rest = &inner[sep + 1..];
    let rest_base = base + sep + 1;
    let else_seps: &[char] = if kind == '?' { &['|', ':'] } else { &['|'] };
    let (then_src, else_src, else_base, separator) = match find_top(rest, else_seps) {
        Some((j, c)) => (
            &rest[..j],
            &rest[j + 1..],
            rest_base + j + 1,
            Some(c.to_string()),
        ),
        None => (rest, "", rest_base + rest.len(), None),
    };
    let condition = head_condition(head, base, scope)?;
    Ok(Segment::Choice {
        condition,
        source: head.trim().to_string(),
        then: branch(then_src, rest_base, scope)?,
        otherwise: branch(else_src, else_base, scope)?,
        syntax: ChoiceSyntax {
            open: format!("{{{}{kind}", head.trim()),
            separator,
            close: "}".to_string(),
        },
    })
}

fn head_condition(head: &str, base: usize, scope: &Scope) -> Result<Condition, LangError> {
    let word = head.trim();
    if word == "first" {
        return Ok(Derived::FirstVisit.to_condition(CmpOp::Eq, FactValue::Bool(true)));
    }
    let expr = parse_expr(head).map_err(|e| e.offset(base))?;
    lower_bool(&expr, scope).map_err(|e| e.offset(base))
}

/// A branch keeps its text but loses the spaces around the separators,
/// so `{night: moonlit | sunlit}` yields `moonlit` and `sunlit`.
fn branch(src: &str, base: usize, scope: &Scope) -> Result<Vec<Segment>, LangError> {
    let lead = src.len() - src.trim_start().len();
    compile_at(src.trim(), base + lead, scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use lantern_model::Story;

    fn scope() -> Scope {
        let story: Story = serde_json::from_value(serde_json::json!({
            "title": "t",
            "start": "hall",
            "vars": [{"id": "weather", "type": {"enum": ["clear", "fog"]}, "initial": "clear"}],
            "rooms": [{"id": "hall", "name": "Hall"}],
            "things": [{"id": "lamp", "name": "brass lamp", "location": "carried", "props": {"lit": false}}]
        }))
        .unwrap();
        Scope::from_story(&story)
    }

    #[test]
    fn phase_shorthand_and_first_visit() {
        let segs = compile_markup(
            "The {night: moonlit | sunlit} courtyard{first: , new to you,} lies quiet.",
            &scope(),
        )
        .unwrap();
        check!(segs.len() == 5);
        assert!(let Segment::Choice { then, otherwise, source, syntax, .. } = &segs[1]);
        check!(source == "night");
        check!(syntax.open == "{night:");
        check!(syntax.separator.as_deref() == Some("|"));
        check!(then == &vec![Segment::Text("moonlit".into())]);
        check!(otherwise == &vec![Segment::Text("sunlit".into())]);
        assert!(let Segment::Choice { then, otherwise, .. } = &segs[3]);
        check!(then == &vec![Segment::Text(", new to you,".into())]);
        check!(otherwise.is_empty());
    }

    #[test]
    fn question_mark_form_and_inserts() {
        let segs = compile_markup(
            "A {lamp.lit ? glowing : dark} {lamp.name}. {{ok}}",
            &scope(),
        )
        .unwrap();
        check!(matches!(&segs[1], Segment::Choice { .. }));
        check!(
            segs[3]
                == Segment::Insert {
                    operand: Operand::ThingName("lamp".into()),
                    source: "{lamp.name}".into()
                }
        );
        check!(segs[4] == Segment::Text(". {ok}".into()));
    }

    #[test]
    fn nested_choices() {
        let segs = compile_markup("{weather == fog: {night: black | grey} fog}", &scope()).unwrap();
        assert!(let Segment::Choice { then, .. } = &segs[0]);
        check!(matches!(&then[0], Segment::Choice { .. }));
        check!(then[1] == Segment::Text(" fog".into()));
    }

    #[test]
    fn errors_have_absolute_spans() {
        let err = compile_markup("It is {lamp.lot ? on}.", &scope()).unwrap_err();
        check!(err.span == (7..15));
        check!(compile_markup("open { never", &scope()).is_err());
    }
}
