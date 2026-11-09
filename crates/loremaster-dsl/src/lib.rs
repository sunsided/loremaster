//! A builder API in plain Rust for loremaster stories (DSL step L2, DS-3).
//!
//! The builders make the same model values as the YAML loader: a [`Pack`]
//! with rooms, things, rules, machines, timers, endings and the story
//! header. [`Loremaster`] joins packs from Rust and from YAML folders in a
//! fixed order, then the checks and [`World::compile`](loremaster_runtime::World::compile)
//! run as for a YAML project. rustfmt and rust-analyzer work on all of it.
//!
//! Each entity gets an [`Origin::Rust`] with the file, line and column of the
//! call that made it, and the pack's name. A diagnostic about the entity
//! names that place.
//!
//! ```
//! use loremaster_dsl::prelude::*;
//!
//! fn lighthouse() -> Pack {
//!     let mut p = Pack::new("lighthouse");
//!     p.story("The Lighthouse", "cottage")
//!         .var("beacon_lit", false);
//!     p.room("cottage", "Keeper's Cottage")
//!         .at(0.0, -1.0)
//!         .text("One room, smelling of tar and pipe smoke.")
//!         .text(frag("Through the salt-crusted window the sea has gone black.").ambient().when("night"))
//!         .exit(Direction::South, "shore")
//!         .door("cottage_door");
//!     p.room("shore", "The Shore").exit("north", "cottage").door("cottage_door");
//!     p.thing("cottage_door", "cottage door").in_room("shore").unlisted().prop("open", false);
//!     p.thing("oil_lamp", "oil lamp").in_room("cottage").portable().prop("lit", false);
//!     p.rule("light_lamp", on::verb(["turn_on", "oil_lamp"]))
//!         .set("oil_lamp.lit", true)
//!         .say("The wick catches.");
//!     p
//! }
//!
//! let world = Loremaster::new().pack(lighthouse()).build().unwrap();
//! assert_eq!(world.story.title, "The Lighthouse");
//! ```
//!
//! The order of the entities in a pack is the order of the calls. For rules
//! this order matters: equal rules run in file order.

mod behavior;
mod convert;
mod dialogue;
mod ending;
mod fragment;
#[cfg(not(target_family = "wasm"))]
mod live;
mod loremaster;
mod machine;
mod pack;
mod room;
mod rule;
mod story;
mod thing;
mod thread;
mod timer;
mod verb;
mod walkthrough;

pub use behavior::{IntoFlow, RustBlock};
pub use convert::{IntoDirection, IntoStates, IntoTime, IntoValue, IntoVarValue};
pub use dialogue::DialogueBuilder;
pub use ending::EndingBuilder;
pub use fragment::{FragmentExt, IntoFragment, frag, variants};
#[cfg(not(target_family = "wasm"))]
pub use live::{LiveStory, Rebuilt};
pub use loremaster::Loremaster;
pub use machine::MachineBuilder;
pub use pack::{BuildPack, HasFrames, HasNotes};
pub use room::RoomBuilder;
pub use rule::{Effects, RuleBuilder, on};
pub use story::StoryBuilder;
pub use thing::ThingBuilder;
pub use thread::ThreadBuilder;
pub use timer::TimerBuilder;
pub use verb::VerbBuilder;
pub use walkthrough::WalkthroughBuilder;

pub use loremaster_model as model;
pub use loremaster_project as project;
pub use loremaster_project::{Origin, Pack};
pub use loremaster_runtime as runtime;

/// Everything a story module needs: `use loremaster_dsl::prelude::*;`.
pub mod prelude {
    pub use crate::{
        BuildPack, Effects, FragmentExt, HasFrames, HasNotes, IntoDirection, IntoFragment,
        IntoStates, IntoTime, IntoValue, IntoVarValue, Loremaster, Origin, Pack, frag, on,
        variants,
    };
    #[cfg(not(target_family = "wasm"))]
    pub use crate::{LiveStory, Rebuilt};
    pub use loremaster_model::{
        AwaitOption, AwaitSpec, CombatSpec, CommandPattern, Direction, Effect, EndingKind,
        EventPattern, Fragment, FragmentMode, Layer, NameStage, Note, NoteRole, States,
        ThreadStatus, TimeOfDay, Value, VerbSlots,
    };
    pub use loremaster_runtime::{Behavior, Cx, Event, Flow, Manifest};
}
