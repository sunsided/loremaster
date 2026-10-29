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
mod explore;
mod game;
mod matrix;
mod output;
mod parser;
mod rng;
mod state;
mod text;
mod world;

pub use explore::{Budget, EndingFound, Exploration, RoomCoverage, TextVariant, explore};
pub use game::{
    AuthorCommand, ExitInfo, FragmentView, FrameFragment, FramePart, FrameUse, Game, Inspect,
    MapLink, MapRoom, MapView, MarkupCheck, PartView, PhaseView, RoomParts, RuleView, Scene,
    TextError, ThingInfo, VarView, VerbInfo,
};
pub use matrix::{Dimension, MAX_CELLS, MatrixCell, RoomMatrix, room_matrix};
pub use output::{Block, BlockKind, Link, LinkKind, LogEntry, Run, Source, Turn};
pub use parser::{Command, Preview};
pub use state::{FragmentProgress, State};
pub use text::{SourcePiece, SourceRole};
pub use world::{CompileError, World};
