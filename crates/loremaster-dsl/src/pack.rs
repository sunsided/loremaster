//! [`BuildPack`]: the builder methods of a [`Pack`], and the parts that all
//! entity builders share.

use crate::convert::id;
use crate::{
    DialogueBuilder, EndingBuilder, MachineBuilder, RoomBuilder, RuleBuilder, RustBlock,
    StoryBuilder, ThingBuilder, ThreadBuilder, TimerBuilder, VerbBuilder, WalkthroughBuilder,
};
use loremaster_model::{EndingKind, EventPattern, Fragment, Frames, Note};
use loremaster_project::{Origin, Pack};
use loremaster_runtime::{Behavior, Cx, Flow};
use std::sync::Arc;

/// The builder methods of a [`Pack`]. Each method adds one entity to the pack
/// and returns a builder for its fields. The entity gets an
/// [`Origin::Rust`] with the file, line and column of the call, so that a
/// diagnostic and the editor can point at the Rust source.
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("shore");
/// p.room("shore", "The Shore")
///     .text("Shingle crunches underfoot.")
///     .exit(Direction::North, "cottage")
///     .door("cottage_door");
/// assert_eq!(p.rooms[0].exits[0].door.as_ref().unwrap().as_str(), "cottage_door");
/// assert!(matches!(p.origins["room:shore"], Origin::Rust { .. }));
/// ```
pub trait BuildPack {
    /// The story header: title and start room. Exactly one pack of a project
    /// gives it. Calling it twice on one pack panics.
    #[track_caller]
    fn story(&mut self, title: impl Into<String>, start: impl AsRef<str>) -> StoryBuilder<'_>;
    /// A room (`rooms/<id>.yaml`).
    #[track_caller]
    fn room(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> RoomBuilder<'_>;
    /// A thing (`things/<id>.yaml`). It starts `nowhere`.
    #[track_caller]
    fn thing(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> ThingBuilder<'_>;
    /// A rule (an entry of `rules/*.yaml`). Use [`on`](crate::on) for the event.
    #[track_caller]
    fn rule(&mut self, id: impl AsRef<str>, on: EventPattern) -> RuleBuilder<'_>;
    /// An ending (`endings/<id>.yaml`).
    #[track_caller]
    fn ending(
        &mut self,
        id: impl AsRef<str>,
        kind: EndingKind,
        title: impl Into<String>,
    ) -> EndingBuilder<'_>;
    /// A timer (`timers/<id>.yaml`) that runs `turns` turns.
    #[track_caller]
    fn timer(&mut self, id: impl AsRef<str>, turns: u32) -> TimerBuilder<'_>;
    /// A puzzle machine (`machines/<id>.yaml`) that starts in `start`.
    #[track_caller]
    fn machine(&mut self, id: impl AsRef<str>, start: impl Into<String>) -> MachineBuilder<'_>;
    /// A custom verb (`verbs/<id>.yaml`).
    #[track_caller]
    fn verb(&mut self, id: impl AsRef<str>, label: impl Into<String>) -> VerbBuilder<'_>;
    /// A dialogue with a thing (`dialogues/<id>.yaml`).
    #[track_caller]
    fn dialogue(&mut self, id: impl AsRef<str>, with: impl AsRef<str>) -> DialogueBuilder<'_>;
    /// A long-range thread (`threads/<id>.yaml`).
    #[track_caller]
    fn thread(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> ThreadBuilder<'_>;
    /// A walkthrough test (`tests/*.yaml`). Its key in the project is the
    /// Rust file and line, where a YAML test has its path.
    #[track_caller]
    fn test(&mut self, name: impl Into<String>) -> WalkthroughBuilder<'_>;
    /// A behavior in Rust, by id (`behaviors/<id>.yaml`).
    #[track_caller]
    fn behavior(&mut self, id: impl AsRef<str>) -> &mut Self;
    /// A behavior in Rust, with its instance: the pack names its id, as
    /// [`behavior`](Self::behavior) does, and [`Loremaster::build`](crate::Loremaster::build)
    /// registers it in the world.
    #[track_caller]
    fn register(&mut self, behavior: impl Behavior + 'static) -> &mut Self;
    /// The behavior of a `rust` block. It belongs to its rule, which a later
    /// pack may replace, so it has no origin and `assemble` does not call it
    /// a duplicate. The later pack's block runs.
    fn register_block<F>(&mut self, block: RustBlock<F>) -> &mut Self
    where
        F: Fn(&mut Cx<'_>) -> Flow + Send + Sync + 'static;
}

impl BuildPack for Pack {
    #[track_caller]
    fn story(&mut self, title: impl Into<String>, start: impl AsRef<str>) -> StoryBuilder<'_> {
        StoryBuilder::new(self, title.into(), id(start))
    }

    #[track_caller]
    fn room(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> RoomBuilder<'_> {
        RoomBuilder::new(self, crate::convert::id(id), name.into())
    }

    #[track_caller]
    fn thing(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> ThingBuilder<'_> {
        ThingBuilder::new(self, crate::convert::id(id), name.into())
    }

    #[track_caller]
    fn rule(&mut self, id: impl AsRef<str>, on: EventPattern) -> RuleBuilder<'_> {
        RuleBuilder::new(self, crate::convert::id(id), on)
    }

    #[track_caller]
    fn ending(
        &mut self,
        id: impl AsRef<str>,
        kind: EndingKind,
        title: impl Into<String>,
    ) -> EndingBuilder<'_> {
        EndingBuilder::new(self, crate::convert::id(id), kind, title.into())
    }

    #[track_caller]
    fn timer(&mut self, id: impl AsRef<str>, turns: u32) -> TimerBuilder<'_> {
        TimerBuilder::new(self, crate::convert::id(id), turns)
    }

    #[track_caller]
    fn machine(&mut self, id: impl AsRef<str>, start: impl Into<String>) -> MachineBuilder<'_> {
        MachineBuilder::new(self, crate::convert::id(id), start.into())
    }

    #[track_caller]
    fn verb(&mut self, id: impl AsRef<str>, label: impl Into<String>) -> VerbBuilder<'_> {
        VerbBuilder::new(self, crate::convert::id(id), label.into())
    }

    #[track_caller]
    fn dialogue(&mut self, id: impl AsRef<str>, with: impl AsRef<str>) -> DialogueBuilder<'_> {
        DialogueBuilder::new(self, crate::convert::id(id), crate::convert::id(with))
    }

    #[track_caller]
    fn thread(&mut self, id: impl AsRef<str>, name: impl Into<String>) -> ThreadBuilder<'_> {
        ThreadBuilder::new(self, crate::convert::id(id), name.into())
    }

    #[track_caller]
    fn test(&mut self, name: impl Into<String>) -> WalkthroughBuilder<'_> {
        WalkthroughBuilder::new(self, name.into())
    }

    #[track_caller]
    fn behavior(&mut self, behavior: impl AsRef<str>) -> &mut Self {
        let behavior = id(behavior);
        let origin = here(self);
        self.origins.insert(format!("behavior:{behavior}"), origin);
        self.behaviors.push(behavior);
        self
    }

    #[track_caller]
    fn register(&mut self, behavior: impl Behavior + 'static) -> &mut Self {
        let shared: Arc<dyn Behavior> = Arc::new(behavior);
        self.behavior(shared.id());
        self.instances.push(Arc::new(shared));
        self
    }

    fn register_block<F>(&mut self, block: RustBlock<F>) -> &mut Self
    where
        F: Fn(&mut Cx<'_>) -> Flow + Send + Sync + 'static,
    {
        self.behaviors.push(id(block.id()));
        let shared: Arc<dyn Behavior> = Arc::new(block);
        self.instances.push(Arc::new(shared));
        self
    }
}

/// The origin of the caller: the line in the story code that made an entity.
#[track_caller]
pub(crate) fn here(pack: &Pack) -> Origin {
    let at = std::panic::Location::caller();
    Origin::Rust {
        file: at.file().to_string(),
        line: at.line(),
        column: at.column(),
        pack: pack.name.clone(),
    }
}

/// Record where the entity `key` (`room:cellar`) comes from.
#[track_caller]
pub(crate) fn register(pack: &mut Pack, key: String) {
    let origin = here(pack);
    pack.origins.insert(key, origin);
}

/// Mark the entity `key` as one that replaces the entity of an earlier pack
/// (`replace: true` in YAML).
pub(crate) fn replace(pack: &mut Pack, key: String) {
    pack.replaces.insert(key);
}

/// Author notes, on every entity that has them.
pub trait HasNotes: Sized {
    #[doc(hidden)]
    fn notes_mut(&mut self) -> &mut Vec<Note>;

    /// An author note. The player never sees it.
    fn note(mut self, text: impl Into<String>) -> Self {
        self.notes_mut().push(Note::text(text));
        self
    }

    /// A task for the authors (`todo:`).
    fn todo(mut self, text: impl Into<String>) -> Self {
        let mut note = Note::text(text);
        note.todo = true;
        self.notes_mut().push(note);
        self
    }

    /// A note in its full form, for example a thread beat.
    fn note_full(mut self, note: Note) -> Self {
        self.notes_mut().push(note);
        self
    }
}

/// Author text around the responses to verbs: on the story, a room or a thing.
pub trait HasFrames: Sized {
    #[doc(hidden)]
    fn frames_mut(&mut self) -> &mut Frames;

    /// A fragment before the response to `verb`.
    fn frame_before(mut self, verb: impl AsRef<str>, f: impl crate::IntoFragment) -> Self {
        let f: Fragment = f.into_fragment();
        self.frames_mut()
            .entry(id(verb))
            .or_default()
            .before
            .push(f);
        self
    }

    /// A fragment instead of the default response to `verb`.
    fn frame_instead(mut self, verb: impl AsRef<str>, f: impl crate::IntoFragment) -> Self {
        let f: Fragment = f.into_fragment();
        self.frames_mut()
            .entry(id(verb))
            .or_default()
            .instead
            .push(f);
        self
    }

    /// A fragment after the response to `verb`.
    fn frame_after(mut self, verb: impl AsRef<str>, f: impl crate::IntoFragment) -> Self {
        let f: Fragment = f.into_fragment();
        self.frames_mut().entry(id(verb)).or_default().after.push(f);
        self
    }
}

/// The `replace` method of an entity builder.
macro_rules! replaceable {
    ($builder:ident, $list:ident, $kind:literal) => {
        impl $builder<'_> {
            /// This entity replaces the one with the same id from an earlier
            /// pack (`replace: true` in YAML). [`assemble`](loremaster_project::assemble)
            /// warns about it.
            pub fn replace(self) -> Self {
                let key = format!("{}:{}", $kind, self.pack.$list[self.i].id);
                crate::pack::replace(self.pack, key);
                self
            }
        }
    };
}
pub(crate) use replaceable;

/// [`HasNotes`] for an entity builder.
macro_rules! with_notes {
    ($builder:ident, $list:ident) => {
        impl crate::HasNotes for $builder<'_> {
            fn notes_mut(&mut self) -> &mut Vec<loremaster_model::Note> {
                &mut self.pack.$list[self.i].notes
            }
        }
    };
}
pub(crate) use with_notes;
