//! The beats of long-range threads, as the explorer sees them: which text or
//! rule is a setup or a payoff, and whether a turn showed it.

use crate::output::Turn;
use loremaster_model::{Fragment, Frames, Note, NoteRole, Story, ThreadStatus};
use serde::{Deserialize, Serialize};

/// What the explorer found for one thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadReach {
    pub thread: String,
    /// True when an explored path showed a payoff.
    pub paid_off: bool,
    /// The shortest explored path on which a payoff shows before any setup, if there is one.
    pub payoff_before_setup: Vec<String>,
    /// Where the payoffs are, for messages, for example `room:vault#2` or `rule:open_vault`.
    pub payoffs: Vec<String>,
}

/// Where a beat is: one fragment, all the text of an entity, or a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    /// A fragment: its runtime owner and index.
    Fragment(String, usize),
    /// Any text of an entity, for a note on a room or a thing: `room:shore`, `thing:lamp`.
    Entity(String),
    Rule(String),
}

impl Place {
    fn label(&self) -> String {
        match self {
            Place::Fragment(owner, i) => format!("{owner}#{}", i + 1),
            Place::Entity(e) => e.clone(),
            Place::Rule(r) => format!("rule:{r}"),
        }
    }

    fn hit(&self, turn: &Turn) -> bool {
        match self {
            Place::Rule(r) => turn.log.iter().any(|l| l.kind == "rule" && l.text == *r),
            _ => turn.blocks.iter().flat_map(|b| &b.runs).any(|run| {
                run.source.as_ref().is_some_and(|s| match self {
                    Place::Fragment(owner, i) => s.owner == *owner && s.fragment == *i,
                    Place::Entity(e) => s.owner == *e || s.owner.starts_with(&format!("{e}/")),
                    Place::Rule(_) => false,
                })
            }),
        }
    }
}

/// One setup or payoff of an open or resolved thread.
#[derive(Debug, Clone)]
pub(crate) struct Beat {
    pub thread: usize,
    pub payoff: bool,
    place: Place,
}

impl Beat {
    pub fn hit(&self, turn: &Turn) -> bool {
        self.place.hit(turn)
    }
}

/// The threads to check (open and resolved ones with a payoff) and their beats.
pub(crate) fn beats(story: &Story) -> (Vec<String>, Vec<Beat>) {
    let threads: Vec<String> = story
        .threads
        .iter()
        .filter(|t| t.status != ThreadStatus::Dropped)
        .map(|t| t.id.to_string())
        .collect();
    let mut out = Vec::new();
    let mut add = |notes: &[Note], place: Place| {
        for n in notes {
            let (Some(thread), Some(role)) = (&n.thread, n.role) else {
                continue;
            };
            let Some(t) = threads.iter().position(|id| id == thread.as_str()) else {
                continue;
            };
            if role != NoteRole::Mention {
                out.push(Beat {
                    thread: t,
                    payoff: role == NoteRole::Payoff,
                    place: place.clone(),
                });
            }
        }
    };
    let fragments = |add: &mut dyn FnMut(&[Note], Place), owner: &str, list: &[Fragment]| {
        for (i, f) in list.iter().enumerate() {
            add(&f.notes, Place::Fragment(owner.to_string(), i));
        }
    };
    let frames = |add: &mut dyn FnMut(&[Note], Place), owner: &str, frames: &Frames| {
        for (verb, frame) in frames {
            for (part, list) in [
                ("before", &frame.before),
                ("instead", &frame.instead),
                ("after", &frame.after),
            ] {
                for (i, f) in list.iter().enumerate() {
                    add(
                        &f.notes,
                        Place::Fragment(format!("{owner}/frames/{verb}/{part}"), i),
                    );
                }
            }
        }
    };
    fragments(&mut add, "story:intro", &story.intro);
    frames(&mut add, "story", &story.frames);
    for r in &story.rooms {
        let owner = format!("room:{}", r.id);
        add(&r.notes, Place::Entity(owner.clone()));
        fragments(&mut add, &owner, &r.description);
        frames(&mut add, &owner, &r.frames);
    }
    for t in &story.things {
        let owner = format!("thing:{}", t.id);
        add(&t.notes, Place::Entity(owner.clone()));
        fragments(&mut add, &owner, &t.description);
        fragments(&mut add, &format!("{owner}/presence"), &t.presence);
        frames(&mut add, &owner, &t.frames);
    }
    for r in &story.rules {
        add(&r.notes, Place::Rule(r.id.to_string()));
    }
    (threads, out)
}

/// The places of the payoffs of each thread, for the report.
pub(crate) fn payoff_labels(beats: &[Beat], thread: usize) -> Vec<String> {
    beats
        .iter()
        .filter(|b| b.thread == thread && b.payoff)
        .map(|b| b.place.label())
        .collect()
}
