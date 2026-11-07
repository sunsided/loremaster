//! The loremaster runtime: a pure, deterministic library.
//!
//! It takes a story and a save state, accepts a command (typed, or picked
//! from the verb bar) and returns output plus the new state. No IO, no clock,
//! no global randomness, so the editor, the web player and the CLI all see
//! exactly the same game.
//!
//! Conditions are dramaturge [`Condition`](dramaturge_model::Condition)
//! trees. The game implements dramaturge's
//! [`NarrativeState`](dramaturge_runtime::NarrativeState) on the save state:
//! story variables and thing properties are world facts, and relations and
//! memories come from dialogues. There is no second store. The runtime
//! answers derived values, such as the time of day or the player's room,
//! through dramaturge's `Custom` condition.

#[doc(hidden)]
pub mod authoring;
mod behavior;
mod eval;
mod events;
mod explore;
mod game;
mod matrix;
mod outline;
mod output;
mod parser;
mod rng;
mod save;
mod session;
mod state;
mod text;
mod threads;
mod walkthrough;
mod world;

pub use behavior::{Behavior, Cx, Flow, Manifest, Unstable};
pub use events::{Event, EventKind, Phase};
pub(crate) use game::Game;
pub use game::{ARRIVED, REFUSED};
pub use game::{ExitInfo, Scene, ThingInfo, VerbInfo};
pub use outline::{DialogueOutline, OutlineNode, OutlineOption};
pub use output::{Block, BlockKind, Choice, Link, LinkKind, LogEntry, Run, Source, TalkInfo, Turn};
pub use parser::{Command, Preview};
pub(crate) use save::Restored;
pub use save::{SAVE_FORMAT, SAVE_VERSION, Save, SaveError, StoryStamp};
pub use session::{
    DEFAULT_UNDO_DEPTH, Effect, EndingInfo, HINT, Hint, HintBook, Input, Loaded, RESTART, RESTORE,
    Session, Snapshot, Status, Step, UNDO,
};
pub use state::{AwaitOptionInfo, AwaitRequest, Awaiting, FragmentProgress, State, Talk};
pub use text::{SourcePiece, SourceRole};
pub use walkthrough::{Verdict, WalkResult, run_walkthrough};
pub use world::{CompileError, World};
