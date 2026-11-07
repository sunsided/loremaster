use crate::Id;
use serde::de::value::MapAccessDeserializer;
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The speaker id that names the player in `by:`.
pub const PLAYER_SPEAKER: &str = "player";

/// A text that shows: the `say:` of a rule effect, of a dialogue node or of an
/// option. `line:` is the same key.
///
/// A line is one string (narration), or a list of segments:
///
/// ```yaml
/// say: "The lock-keeper shrugs."
/// say:
///   - "He turns the poker in his hands."
///   - { says: "Nobody.", by: keeper }
///   - { narrate: "He does not look at you.", when: ash_is_fresh }
/// ```
///
/// The renderer puts straight double quotes around speech and joins the
/// segments that show with one space.
///
/// A line that is one plain string is written out as that string, so stories
/// without segments keep their model hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub segments: Vec<LineSegment>,
}

/// One piece of a [`Line`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSegment {
    pub voice: Voice,
    /// The text, in inline markup. Speech is without its quotes.
    pub text: String,
    /// The segment shows only while this condition holds.
    pub when: Option<String>,
}

/// How a segment is spoken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Voice {
    /// A bare string: narration.
    Plain,
    /// `{ narrate: "…" }`: narration, written out.
    Narrate,
    /// `{ says: "…", by: <thing or player> }`: speech. Without `by`, the
    /// speaker of a dialogue node is the dialogue's thing, the speaker of an
    /// option is the player, and a rule's speech has no speaker.
    Says { by: Option<Id> },
}

impl Line {
    pub fn new(segments: Vec<LineSegment>) -> Self {
        Self { segments }
    }

    /// A line of one bare string.
    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(vec![LineSegment::plain(text)])
    }

    /// The text of a line that is one bare string.
    pub fn as_plain(&self) -> Option<&str> {
        match self.segments.as_slice() {
            [s] if s.voice == Voice::Plain && s.when.is_none() => Some(&s.text),
            _ => None,
        }
    }

    /// The line as one readable string for outlines and editors: speech in
    /// quotes, segments joined with a space, markup and conditions as
    /// written. A plain line is its text.
    pub fn source(&self) -> String {
        if let Some(text) = self.as_plain() {
            return text.to_string();
        }
        self.segments
            .iter()
            .map(|s| match s.voice {
                Voice::Says { .. } => format!("\"{}\"", s.text),
                _ => s.text.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The bare strings that follow a `says` segment. They read like more
    /// speech; `{ narrate: … }` says what is meant. Indexes into `segments`.
    pub fn bare_after_speech(&self) -> Vec<usize> {
        self.segments
            .windows(2)
            .enumerate()
            .filter(|(_, w)| matches!(w[0].voice, Voice::Says { .. }) && w[1].voice == Voice::Plain)
            .map(|(i, _)| i + 1)
            .collect()
    }
}

impl From<&str> for Line {
    fn from(text: &str) -> Self {
        Self::plain(text)
    }
}

impl From<String> for Line {
    fn from(text: String) -> Self {
        Self::plain(text)
    }
}

impl From<Vec<LineSegment>> for Line {
    fn from(segments: Vec<LineSegment>) -> Self {
        Self::new(segments)
    }
}

impl LineSegment {
    /// A bare string: narration.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            voice: Voice::Plain,
            text: text.into(),
            when: None,
        }
    }

    /// `{ narrate: … }`.
    pub fn narrate(text: impl Into<String>) -> Self {
        Self {
            voice: Voice::Narrate,
            text: text.into(),
            when: None,
        }
    }

    /// `{ says: … }`, with the default speaker.
    pub fn says(text: impl Into<String>) -> Self {
        Self {
            voice: Voice::Says { by: None },
            text: text.into(),
            when: None,
        }
    }

    /// Set the speaker of speech: a thing id or `player`. A narration
    /// segment becomes speech.
    pub fn by(mut self, speaker: impl Into<Id>) -> Self {
        self.voice = Voice::Says {
            by: Some(speaker.into()),
        };
        self
    }

    /// Show the segment only while `condition` holds. A bare string becomes
    /// `{ narrate: … }`, which is the form that can carry a condition.
    pub fn when(mut self, condition: impl Into<String>) -> Self {
        if self.voice == Voice::Plain {
            self.voice = Voice::Narrate;
        }
        self.when = Some(condition.into());
        self
    }

    pub fn is_speech(&self) -> bool {
        matches!(self.voice, Voice::Says { .. })
    }

    /// The explicit speaker of speech.
    pub fn speaker(&self) -> Option<&Id> {
        match &self.voice {
            Voice::Says { by } => by.as_ref(),
            _ => None,
        }
    }
}

impl Serialize for Line {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if let Some(text) = self.as_plain() {
            return s.serialize_str(text);
        }
        let mut seq = s.serialize_seq(Some(self.segments.len()))?;
        for segment in &self.segments {
            seq.serialize_element(segment)?;
        }
        seq.end()
    }
}

impl Serialize for LineSegment {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.voice == Voice::Plain && self.when.is_none() {
            return s.serialize_str(&self.text);
        }
        let mut map = s.serialize_map(None)?;
        match &self.voice {
            // A bare string cannot carry a condition: write it out as narration.
            Voice::Plain | Voice::Narrate => map.serialize_entry("narrate", &self.text)?,
            Voice::Says { by } => {
                map.serialize_entry("says", &self.text)?;
                if let Some(by) = by {
                    map.serialize_entry("by", by)?;
                }
            }
        }
        if let Some(when) = &self.when {
            map.serialize_entry("when", when)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Line {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct LineVisitor;
        impl<'de> Visitor<'de> for LineVisitor {
            type Value = Line;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a text, or a list of segments: strings, `{ narrate: … }` and `{ says: …, by: … }`")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Line, E> {
                Ok(Line::plain(v))
            }

            fn visit_string<E: Error>(self, v: String) -> Result<Line, E> {
                Ok(Line::plain(v))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Line, A::Error> {
                let mut segments = Vec::new();
                while let Some(s) = seq.next_element::<LineSegment>()? {
                    segments.push(s);
                }
                if segments.is_empty() {
                    return Err(A::Error::custom("a line needs at least one segment"));
                }
                Ok(Line::new(segments))
            }

            fn visit_map<A: MapAccess<'de>>(self, _map: A) -> Result<Line, A::Error> {
                Err(A::Error::custom(
                    "a line is a text or a list of segments; put one segment in a list: `[{ says: … }]`",
                ))
            }
        }
        d.deserialize_any(LineVisitor)
    }
}

/// A segment in a list, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SegmentRepr {
    #[serde(default)]
    narrate: Option<String>,
    #[serde(default)]
    says: Option<String>,
    #[serde(default)]
    by: Option<Id>,
    #[serde(default)]
    when: Option<String>,
}

impl<'de> Deserialize<'de> for LineSegment {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct SegmentVisitor;
        impl<'de> Visitor<'de> for SegmentVisitor {
            type Value = LineSegment;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a text, `{ narrate: … }` or `{ says: …, by: … }`")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<LineSegment, E> {
                Ok(LineSegment::plain(v))
            }

            fn visit_string<E: Error>(self, v: String) -> Result<LineSegment, E> {
                Ok(LineSegment::plain(v))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<LineSegment, A::Error> {
                let r = SegmentRepr::deserialize(MapAccessDeserializer::new(map))?;
                let (voice, text) = match (r.narrate, r.says) {
                    (Some(text), None) => {
                        if r.by.is_some() {
                            return Err(A::Error::custom(
                                "`by` names a speaker, so it needs `says`, not `narrate`",
                            ));
                        }
                        (Voice::Narrate, text)
                    }
                    (None, Some(text)) => (Voice::Says { by: r.by }, text),
                    (Some(_), Some(_)) => {
                        return Err(A::Error::custom(
                            "a segment is `narrate` or `says`, not both; make two segments",
                        ));
                    }
                    (None, None) => {
                        return Err(A::Error::custom("a segment needs `narrate` or `says`"));
                    }
                };
                Ok(LineSegment {
                    voice,
                    text,
                    when: r.when,
                })
            }
        }
        d.deserialize_any(SegmentVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use serde_json::json;

    fn line(v: serde_json::Value) -> Line {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn a_string_is_one_bare_segment() {
        let l = line(json!("The lock-keeper shrugs."));
        check!(l == Line::plain("The lock-keeper shrugs."));
        check!(l.as_plain() == Some("The lock-keeper shrugs."));
        check!(serde_json::to_value(&l).unwrap() == json!("The lock-keeper shrugs."));
    }

    #[test]
    fn a_list_mixes_narration_and_speech() {
        let l = line(json!([
            "He turns the poker.",
            { "says": "Nobody.", "by": "keeper" },
            { "narrate": "He looks away.", "when": "ash_is_fresh" },
            { "says": "Who?" }
        ]));
        check!(
            l.segments
                == [
                    LineSegment::plain("He turns the poker."),
                    LineSegment::says("Nobody.").by("keeper"),
                    LineSegment::narrate("He looks away.").when("ash_is_fresh"),
                    LineSegment::says("Who?"),
                ]
        );
        check!(l.as_plain().is_none());
        check!(l.source() == "He turns the poker. \"Nobody.\" He looks away. \"Who?\"");
        let back: Line = serde_json::from_value(serde_json::to_value(&l).unwrap()).unwrap();
        check!(back == l);
    }

    #[test]
    fn a_list_of_one_string_stays_a_list() {
        // Only one bare string is written as a string; `[{ narrate: … }]` keeps its form.
        let l = line(json!([{ "narrate": "Quiet." }]));
        check!(serde_json::to_value(&l).unwrap() == json!([{ "narrate": "Quiet." }]));
    }

    #[test]
    fn bad_segments_say_why() {
        let err = |v| serde_json::from_value::<Line>(v).unwrap_err().to_string();
        check!(err(json!([])).contains("at least one segment"));
        check!(err(json!([{ "says": "a", "narrate": "b" }])).contains("not both"));
        check!(err(json!([{ "narrate": "a", "by": "keeper" }])).contains("needs `says`"));
        check!(err(json!([{ "when": "x" }])).contains("`narrate` or `says`"));
        check!(err(json!([{ "say": "a" }])).contains("unknown field `say`"));
        check!(err(json!({ "says": "a" })).contains("in a list"));
    }

    #[test]
    fn a_bare_string_after_speech_is_found() {
        let l = line(json!([{ "says": "Prove it," }, "he says.", { "narrate": "Fine." }]));
        assert!(l.bare_after_speech() == [1]);
        let l = line(json!(["He shrugs.", { "says": "Prove it," }, { "narrate": "he says." }]));
        check!(l.bare_after_speech().is_empty());
    }
}
