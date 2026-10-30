use crate::Id;
use serde::{Deserialize, Serialize};

/// An author note: context for the authors, never shown to the player.
///
/// In project files a note is a plain string, a map with `todo` for a task,
/// or a map with `text` and more fields:
///
/// ```yaml
/// notes:
///   - Hal's boat should feel abandoned, not wrecked.
///   - todo: Decide if the ferryman comes back at dawn.
///   - { text: The gull knows where the key is., thread: lost_key, role: setup }
/// ```
///
/// It serializes in one flat form (`{ id, text, todo, done, thread, role }`),
/// so that the editor and the tools read every note the same way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "NoteRepr")]
pub struct Note {
    /// Optional stable name, so that other notes or an assistant can point at it.
    pub id: Option<Id>,
    pub text: String,
    /// A task for the authors. `done` closes it.
    pub todo: bool,
    pub done: bool,
    /// The thread that this note is a beat of.
    pub thread: Option<Id>,
    /// What the beat does for its thread. Only with `thread`.
    pub role: Option<NoteRole>,
}

impl Note {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: None,
            text: text.into(),
            todo: false,
            done: false,
            thread: None,
            role: None,
        }
    }

    /// A TODO that is not done.
    pub fn is_open_todo(&self) -> bool {
        self.todo && !self.done
    }
}

/// What a note does for its thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteRole {
    /// The reader learns or sees something that matters later.
    Setup,
    /// The thread resolves here.
    Payoff,
    /// The thread shows again, to keep it in the reader's mind.
    Mention,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NoteRepr {
    Plain(String),
    Full(NoteFull),
    /// An unquoted note with a colon, such as `- Premise: a lighthouse without a keeper.`,
    /// reads as a map with one entry. It is the text the author wrote.
    Colon(std::collections::BTreeMap<String, String>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteFull {
    #[serde(default)]
    id: Option<Id>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    todo: Option<String>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    thread: Option<Id>,
    #[serde(default)]
    role: Option<NoteRole>,
}

impl TryFrom<NoteRepr> for Note {
    type Error = String;

    fn try_from(repr: NoteRepr) -> Result<Self, String> {
        let f = match repr {
            NoteRepr::Plain(text) => return Ok(Note::text(text)),
            NoteRepr::Full(f) => f,
            NoteRepr::Colon(map) if map.len() == 1 => {
                let (key, value) = map.into_iter().next().unwrap_or_default();
                return Ok(Note::text(format!("{key}: {value}")));
            }
            NoteRepr::Colon(_) => {
                return Err("a note is a text, or a map with `text` or `todo`".into());
            }
        };
        let (text, todo) = match (f.text, f.todo) {
            (Some(text), None) => (text, false),
            (None, Some(text)) => (text, true),
            (Some(_), Some(_)) => return Err("a note has `text` or `todo`, not both".into()),
            (None, None) => return Err("a note needs `text` or `todo`".into()),
        };
        if f.done && !todo {
            return Err("only a `todo` note can be `done`".into());
        }
        if f.role.is_some() && f.thread.is_none() {
            return Err("a note with a `role` needs a `thread`".into());
        }
        Ok(Note {
            id: f.id,
            text,
            todo,
            done: f.done,
            thread: f.thread,
            role: f.role,
        })
    }
}

/// A thread: a promise to the reader that spans the story, such as a hint in
/// the first act that resolves in the last. Its beats are the notes with
/// `thread: <id>` on rooms, things, rules and fragments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thread {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub status: ThreadStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Variables or thing properties that carry the thread, for example `knows_key`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    #[default]
    Open,
    Resolved,
    /// A cut idea, kept for its history. The checker does not warn about it.
    Dropped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn plain_string_is_a_note() {
        let n: Note = serde_json::from_str(r#""Keep it eerie.""#).unwrap();
        check!(n == Note::text("Keep it eerie."));
        check!(
            serde_json::to_string(&n).unwrap()
                == r#"{"id":null,"text":"Keep it eerie.","todo":false,"done":false,"thread":null,"role":null}"#
        );
    }

    #[test]
    fn a_done_todo_reads() {
        let json = r#"{"todo":"Name the ferryman.","done":true}"#;
        let n: Note = serde_json::from_str(json).unwrap();
        check!(n.text == "Name the ferryman.");
        check!(n.todo);
        check!(n.done);
        check!(!n.is_open_todo());
    }

    #[test]
    fn a_thread_beat_reads() {
        let json = r#"{"text":"The gull knows.","thread":"lost_key","role":"setup"}"#;
        let n: Note = serde_json::from_str(json).unwrap();
        check!(n.thread.as_ref().map(|t| t.as_str()) == Some("lost_key"));
        check!(n.role == Some(NoteRole::Setup));
    }

    #[test]
    fn an_unquoted_colon_keeps_the_text() {
        let n: Note = serde_json::from_str(r#"{"Premise":"a lighthouse."}"#).unwrap();
        check!(n == Note::text("Premise: a lighthouse."));
    }

    #[test]
    fn bad_notes_are_refused() {
        for json in [
            r#"{"text":"a","todo":"b"}"#,
            r#"{"thread":"x"}"#,
            r#"{"text":"a","done":true}"#,
            r#"{"text":"a","role":"payoff"}"#,
            r#"{"text":"a","colour":"red"}"#,
            r#"{"a":"b","c":"d"}"#,
        ] {
            check!(serde_json::from_str::<Note>(json).is_err(), "{json}");
        }
    }
}
