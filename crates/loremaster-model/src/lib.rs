//! World model for loremaster stories.
//!
//! These are the authored types: what a project folder deserializes into and
//! what the story bundle carries. Conditions and inline markup are kept as
//! source text here; `loremaster-lang` parses and checks them, and the runtime
//! evaluates the parsed form. Keeping source text in the model means the
//! bundle stays readable and survives changes to the expression language.

mod agenda;
mod awaits;
mod clock;
mod command;
mod de;
mod diagnostic;
mod dialogue;
mod ending;
mod fragment;
mod frame;
mod id;
mod interface;
mod line;
mod machine;
mod media;
mod note;
mod plan;
mod room;
mod rule;
mod story;
mod thing;
mod timer;
mod value;
mod verb;
mod walkthrough;

pub use agenda::{Agenda, AgendaStep};
pub use awaits::{AwaitOption, AwaitSpec, CombatSpec, Dice};
pub use clock::{ClockConfig, Phase, TimeOfDay};
pub use command::CommandPattern;
pub use diagnostic::{Diagnostic, Severity};
pub use dialogue::{Dialogue, DialogueId, DialogueNode, DialogueOption, DialogueTarget, Outcomes};
/// The dialogue AST of dramaturge (decision D3).
pub use dramaturge_syntax;
pub use ending::{Ending, EndingKind};
pub use fragment::{Candidate, Fragment, FragmentMode, Layer, Pick, TextPool};
pub use frame::{Frame, Frames};
pub use id::Id;
pub use interface::{ActionDecl, ArgType, EventDecl, Interface, InterfaceMismatch, check_args};
pub use line::{Line, LineSegment, PLAYER_SPEAKER, Voice};
pub use machine::{Machine, StateDef, States, Step};
pub use media::{Asset, Licence, Media, MediaVariant, Region};
pub use note::{Note, NoteRole, Thread, ThreadStatus};
pub use plan::{ChartStep, SERIAL_RUN, StepChart, step_chart};
pub use room::{Direction, Exit, Room};
pub use rule::{Effect, EventPattern, Rule, SumError, SumTerm, Term, parse_sum};
pub use story::{EpisodeItems, Limit, Rank, Score, Stat, Story, VarDef, VarType};
pub use thing::{Location, NameStage, Thing};
pub use timer::{StartTimers, Timer};
pub use value::Value;
pub use verb::{VerbDef, VerbSlots, standard_verbs};
pub use walkthrough::{Expect, Pending, StepPlan, WalkStep, Walkthrough};
