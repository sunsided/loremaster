//! Author notes, TODOs and threads, collected from the whole project.

use crate::Project;
use loremaster_model::{Fragment, Frames, Note, NoteRole, ThreadStatus};
use serde::Serialize;

/// One note and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteRef {
    /// The entity that holds the note: `story`, `room:shore`, `thing:lamp`, `rule:find_key`
    /// or `thread:lost_key`.
    pub entity: String,
    pub file: String,
    /// Where in the entity, for example `notes[0]` or `description[2].notes[1]`.
    pub at: String,
    /// For a note on a fragment: the fragment's `id`, when it has one.
    pub fragment_id: Option<String>,
    /// 1-based line in the file, when the text could be found.
    pub line: Option<u32>,
    pub note: Note,
}

/// Every note of the project, in file order: story, rooms, things, rules, threads.
pub fn notes(project: &Project) -> Vec<NoteRef> {
    let mut c = Collector {
        project,
        out: Vec::new(),
        seen: Default::default(),
    };
    let story = &project.story;
    c.list("story", "notes", None, &story.notes);
    c.fragments("story", "intro", &story.intro);
    c.frames("story", &story.frames);
    for r in &story.rooms {
        let e = format!("room:{}", r.id);
        c.list(&e, "notes", None, &r.notes);
        c.fragments(&e, "description", &r.description);
        c.frames(&e, &r.frames);
    }
    for t in &story.things {
        let e = format!("thing:{}", t.id);
        c.list(&e, "notes", None, &t.notes);
        c.fragments(&e, "description", &t.description);
        c.fragments(&e, "presence", &t.presence);
        c.frames(&e, &t.frames);
    }
    for r in &story.rules {
        c.list(&format!("rule:{}", r.id), "notes", None, &r.notes);
    }
    for t in &story.threads {
        c.list(&format!("thread:{}", t.id), "notes", None, &t.notes);
    }
    c.out
}

/// The TODOs that are not done.
pub fn open_todos(project: &Project) -> Vec<NoteRef> {
    notes(project)
        .into_iter()
        .filter(|n| n.note.is_open_todo())
        .collect()
}

/// A thread with its beats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThreadView {
    pub id: String,
    pub name: String,
    pub status: ThreadStatus,
    pub summary: Option<String>,
    pub file: String,
    /// The notes that name this thread, and the notes of the thread file.
    pub beats: Vec<NoteRef>,
    pub setups: usize,
    pub payoffs: usize,
    pub open_todos: usize,
}

/// Every thread with its beats.
pub fn threads(project: &Project) -> Vec<ThreadView> {
    let all = notes(project);
    project
        .story
        .threads
        .iter()
        .map(|t| {
            let own = format!("thread:{}", t.id);
            let beats: Vec<NoteRef> = all
                .iter()
                .filter(|n| n.entity == own || n.note.thread.as_ref() == Some(&t.id))
                .cloned()
                .collect();
            let count = |role| beats.iter().filter(|n| n.note.role == Some(role)).count();
            ThreadView {
                id: t.id.to_string(),
                name: t.name.clone(),
                status: t.status,
                summary: t.summary.clone(),
                file: project
                    .origins
                    .get(&own)
                    .cloned()
                    .unwrap_or_else(|| format!("threads/{}.yaml", t.id)),
                setups: count(NoteRole::Setup),
                payoffs: count(NoteRole::Payoff),
                open_todos: beats.iter().filter(|n| n.note.is_open_todo()).count(),
                beats,
            }
        })
        .collect()
}

/// The note text on one line, for example `Keep it.` in `  - todo: "Keep it."` or in
/// `- { text: Keep it., thread: x }`. None when the line holds no `text` or `todo` value.
fn note_value(line: &str) -> Option<&str> {
    let rest = line
        .trim_start()
        .strip_prefix("- ")
        .unwrap_or(line.trim_start());
    let (rest, flow) = match rest.strip_prefix('{') {
        Some(r) => (r.trim_start(), true),
        None => (rest, false),
    };
    let rest = rest
        .strip_prefix("text:")
        .or_else(|| rest.strip_prefix("todo:"))
        .map(str::trim_start)
        .unwrap_or(rest);
    // A quoted value ends at the same quote sign that is not escaped.
    if let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let inner = &rest[1..];
        return inner
            .char_indices()
            .find(|&(i, c)| c == quote && !inner[..i].ends_with('\\'))
            .map(|(i, _)| &inner[..i]);
    }
    let end = if flow {
        rest.find([',', '}']).unwrap_or(rest.len())
    } else {
        rest.find(" #").unwrap_or(rest.len())
    };
    Some(rest[..end].trim_end())
}

/// The 1-based lines where `needle` is the text of a note: inside a list under a `notes:` key.
fn note_lines(text: &str, needle: &str) -> Vec<u32> {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start_matches([' ', '-']).len();
    let under_notes = |i: usize| {
        let mut level = indent(lines[i]);
        for l in lines[..i].iter().rev() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let own = indent(l);
            if own < level {
                if l.trim_start_matches([' ', '-']).starts_with("notes:") {
                    return true;
                }
                level = own;
                if level == 0 {
                    break;
                }
            }
        }
        false
    };
    lines
        .iter()
        .enumerate()
        .filter(|(i, l)| note_value(l) == Some(needle) && under_notes(*i))
        .map(|(i, _)| i as u32 + 1)
        .collect()
}

struct Collector<'a> {
    project: &'a Project,
    out: Vec<NoteRef>,
    /// How many notes with the same text each file had so far, to find the next one.
    seen: std::collections::HashMap<(String, String), usize>,
}

impl Collector<'_> {
    fn file_of(&self, entity: &str) -> String {
        self.project
            .origins
            .get(entity)
            .cloned()
            .unwrap_or_else(|| "story.yaml".into())
    }

    fn list(&mut self, entity: &str, at: &str, fragment_id: Option<&str>, notes: &[Note]) {
        let file = self.file_of(entity);
        let text = self.project.file(&file).map(|f| f.text.as_str());
        for (i, note) in notes.iter().enumerate() {
            // The same text can be in the story text or in another note: count only the
            // places under a `notes:` key, and take the next one of this file.
            let nth = self
                .seen
                .entry((file.clone(), note.text.clone()))
                .or_default();
            let line = text.and_then(|t| note_lines(t, &note.text).get(*nth).copied());
            *nth += 1;
            self.out.push(NoteRef {
                entity: entity.to_string(),
                file: file.clone(),
                at: format!("{at}[{i}]"),
                fragment_id: fragment_id.map(str::to_string),
                line,
                note: note.clone(),
            });
        }
    }

    fn fragments(&mut self, entity: &str, at: &str, fragments: &[Fragment]) {
        for (i, f) in fragments.iter().enumerate() {
            let id = f.id.as_ref().map(|id| id.as_str());
            self.list(entity, &format!("{at}[{i}].notes"), id, &f.notes);
        }
    }

    fn frames(&mut self, entity: &str, frames: &Frames) {
        for (verb, frame) in frames {
            let at = format!("frames.{verb}");
            self.fragments(entity, &format!("{at}.before"), &frame.before);
            self.fragments(entity, &format!("{at}.instead"), &frame.instead);
            self.fragments(entity, &format!("{at}.after"), &frame.after);
        }
    }
}
