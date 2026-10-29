//! The lantern runtime: a pure, deterministic library.
//!
//! It takes a story and a save state, accepts a command (typed, or picked
//! from the verb bar) and returns output plus the new state. No IO, no clock,
//! no global randomness, so the editor, the web player and the CLI all see
//! exactly the same game.
//!
//! Conditions are dramaturge [`Condition`](dramaturge_model::Condition)
//! trees. Story variables and thing properties live as world facts in a
//! [`DialogWorldState`](dramaturge_runtime::DialogWorldState); derived values
//! such as the time of day or the player's room are answered by the runtime
//! through dramaturge's `Custom` condition.

mod eval;
mod game;
mod output;
mod parser;
mod rng;
mod state;
mod text;
mod world;

pub use game::{ExitInfo, Game, Scene, ThingInfo, VerbInfo};
pub use output::{Block, BlockKind, Link, LinkKind, LogEntry, Run, Source, Turn};
pub use parser::Command;
pub use state::{FragmentProgress, State};
pub use world::{CompileError, World};
