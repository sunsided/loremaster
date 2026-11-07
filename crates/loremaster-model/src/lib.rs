//! World model for loremaster stories.
//!
//! These are the authored types: what a project folder deserializes into and
//! what the story bundle carries. Conditions and inline markup are kept as
//! source text here; `loremaster-lang` parses and checks them, and the runtime
//! evaluates the parsed form. Keeping source text in the model means the
//! bundle stays readable and survives changes to the expression language.

mod awaits;
mod clock;
mod de;
mod diagnostic;
mod dialogue;
mod ending;
mod fragment;
mod frame;
mod id;
mod line;
mod machine;
mod note;
mod room;
mod rule;
mod story;
mod thing;
mod timer;
mod value;
mod verb;
mod walkthrough;

pub use awaits::{AwaitOption, AwaitSpec, Dice};
pub use clock::{ClockConfig, Phase, TimeOfDay};
pub use diagnostic::{Diagnostic, Severity};
pub use dialogue::{Dialogue, DialogueId, DialogueNode, DialogueOption, DialogueTarget, Outcomes};
/// The dialogue AST of dramaturge (decision D3).
pub use dramaturge_syntax;
pub use ending::{Ending, EndingKind};
pub use fragment::{Fragment, FragmentMode, Layer};
pub use frame::{Frame, Frames};
pub use id::Id;
pub use line::{Line, LineSegment, PLAYER_SPEAKER, Voice};
pub use machine::{Machine, StateDef, States, Step};
pub use note::{Note, NoteRole, Thread, ThreadStatus};
pub use room::{Direction, Exit, Room};
pub use rule::{Effect, EventPattern, Rule};
pub use story::{Rank, Score, Story, VarDef, VarType};
pub use thing::{Location, NameStage, Thing};
pub use timer::{StartTimers, Timer};
pub use value::Value;
pub use verb::{VerbDef, VerbSlots, standard_verbs};
pub use walkthrough::{Expect, Pending, WalkStep, Walkthrough};
