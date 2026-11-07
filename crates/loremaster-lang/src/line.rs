use crate::{LangError, Scope, Segment, closest, compile_condition, compile_markup};
use dramaturge_model::Condition;
use loremaster_model::{Line, PLAYER_SPEAKER, Voice};

/// One segment of a line, compiled.
#[derive(Debug, Clone, PartialEq)]
pub struct LinePart {
    /// The markup of the segment. Speech is without its quotes; the renderer
    /// adds them.
    pub text: Vec<Segment>,
    /// True for a `says` segment.
    pub speech: bool,
    /// Who speaks: a thing id, or `player`. Only speech has a speaker, and
    /// speech in a rule without `by` has none.
    pub speaker: Option<String>,
    /// The segment shows only while this holds.
    pub when: Option<Condition>,
    /// The condition as written, for the debugger.
    pub when_source: Option<String>,
}

/// A compiled [`Line`]: its segments in order.
pub type CompiledLine = Vec<LinePart>;

/// A problem in one segment of a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineError {
    /// The place inside the line: empty for a plain line, else `[i]`,
    /// `[i].when` or `[i].by`.
    pub at: String,
    /// The source text the error is in.
    pub source: String,
    pub error: LangError,
}

/// Is `by` a speaker: `player` or a thing of the story? The error suggests a
/// close name.
pub fn check_speaker(by: &str, scope: &Scope) -> Result<(), LangError> {
    if by == PLAYER_SPEAKER || (!by.starts_with('@') && scope.things.contains_key(by)) {
        return Ok(());
    }
    let names = scope
        .things
        .keys()
        .map(String::as_str)
        .chain([PLAYER_SPEAKER]);
    let hint = closest(by, names)
        .map(|n| format!("; did you mean `{n}`?"))
        .unwrap_or_default();
    Err(LangError::new(
        format!("unknown speaker `{by}`: `by` names a thing or `player`{hint}"),
        0..by.len(),
    ))
}

/// Compile a line. `default_speaker` speaks the `says` segments without `by`:
/// the dialogue's thing for a node, `player` for an option, none in a rule.
pub fn compile_line(
    line: &Line,
    default_speaker: Option<&str>,
    scope: &Scope,
) -> Result<CompiledLine, LineError> {
    let plain = line.as_plain().is_some();
    line.segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let at = |field: &str| {
                if plain {
                    field.trim_start_matches('.').to_string()
                } else {
                    format!("[{i}]{field}")
                }
            };
            let text = compile_markup(&s.text, scope).map_err(|error| LineError {
                at: at(""),
                source: s.text.clone(),
                error,
            })?;
            let when = s
                .when
                .as_deref()
                .map(|w| {
                    compile_condition(w, scope).map_err(|error| LineError {
                        at: at(".when"),
                        source: w.to_string(),
                        error,
                    })
                })
                .transpose()?;
            let (speech, speaker) = match &s.voice {
                Voice::Says { by: Some(by) } => {
                    check_speaker(by.as_str(), scope).map_err(|error| LineError {
                        at: at(".by"),
                        source: by.to_string(),
                        error,
                    })?;
                    (true, Some(by.to_string()))
                }
                Voice::Says { by: None } => (true, default_speaker.map(String::from)),
                Voice::Plain | Voice::Narrate => (false, None),
            };
            Ok(LinePart {
                text,
                speech,
                speaker,
                when,
                when_source: s.when.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use loremaster_model::{LineSegment, Story};

    fn scope() -> Scope {
        let story: Story = serde_json::from_value(serde_json::json!({
            "title": "t",
            "start": "bar",
            "vars": [{"id": "late", "type": "bool", "initial": false}],
            "rooms": [{"id": "bar", "name": "Bar"}],
            "things": [{"id": "keeper", "name": "keeper", "location": {"room": "bar"}}],
        }))
        .unwrap();
        Scope::from_story(&story)
    }

    #[test]
    fn speech_gets_its_speaker() {
        let line = Line::new(vec![
            LineSegment::plain("He shrugs."),
            LineSegment::says("Prove it,"),
            LineSegment::says("Fine.").by("player"),
            LineSegment::narrate("he says.").when("late"),
        ]);
        let c = compile_line(&line, Some("keeper"), &scope()).unwrap();
        let speakers: Vec<_> = c.iter().map(|p| p.speaker.as_deref()).collect();
        check!(speakers == [None, Some("keeper"), Some("player"), None]);
        check!(c.iter().map(|p| p.speech).collect::<Vec<_>>() == [false, true, true, false]);
        check!(c[3].when.is_some());
        check!(c[3].when_source.as_deref() == Some("late"));
        let c = compile_line(&line, None, &scope()).unwrap();
        check!(c[1].speaker.is_none());
    }

    #[test]
    fn errors_name_the_segment() {
        let s = scope();
        let err = compile_line(&Line::plain("{nope}"), None, &s).unwrap_err();
        check!(err.at == "");
        let line = Line::new(vec![
            LineSegment::plain("Hm."),
            LineSegment::says("No.").when("lte"),
        ]);
        let err = compile_line(&line, None, &s).unwrap_err();
        check!(err.at == "[1].when");
        let line = Line::new(vec![LineSegment::says("No.").by("keper")]);
        let err = compile_line(&line, None, &s).unwrap_err();
        check!(err.at == "[0].by");
        assert!(err.error.message.contains("did you mean `keeper`"));
        check!(check_speaker("@player", &s).is_err());
    }
}
