//! World model for loremaster stories.
//!
//! These are the authored types: what a project folder deserializes into and
//! what the story bundle carries. Conditions and inline markup are kept as
//! source text here; `loremaster-lang` parses and checks them, and the runtime
//! evaluates the parsed form. Keeping source text in the model means the
//! bundle stays readable and survives changes to the expression language.

mod clock;
mod dialogue;
mod fragment;
mod frame;
mod id;
mod note;
mod room;
mod rule;
mod story;
mod thing;
mod value;
mod verb;

pub use clock::{ClockConfig, Phase, TimeOfDay};
pub use dialogue::{Dialogue, DialogueNode, DialogueOption, Outcomes};
pub use fragment::{Fragment, FragmentMode, Layer};
pub use frame::{Frame, Frames};
pub use id::Id;
pub use note::{Note, NoteRole, Thread, ThreadStatus};
pub use room::{Direction, Exit, Room};
pub use rule::{Effect, EventPattern, Rule};
pub use story::{Story, VarDef, VarType};
pub use thing::{Location, NameStage, Thing};
pub use value::Value;
pub use verb::{VerbDef, VerbSlots, standard_verbs};
