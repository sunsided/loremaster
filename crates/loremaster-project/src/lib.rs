//! The on-disk project format and the checker.
//!
//! A project is a folder of YAML files, one per entity, so diffs read like
//! edits to a manuscript:
//!
//! ```text
//! story.yaml        title, start room, intro, clock, variables, custom verbs
//! rooms/*.yaml      one room per file
//! things/*.yaml     one thing per file
//! rules/*.yaml      one rule, or a list of rules, per file
//! threads/*.yaml    one long-range thread per file
//! ```
//!
//! The other folders are in [`FOLDERS`]. A YAML file in an unknown folder, and
//! an unknown field in any file, is an error. An entity's `id` defaults to its
//! file name. Loading works on in-memory files so the same code runs in the
//! browser; [`read_dir`] collects them from disk.
//!
//! [`check_packs`] is the one function for the schema checks: the YAML loader
//! runs it through [`check_files`], and the `world!` macro runs it on its
//! packs at compile time.

mod bindings;
mod check;
mod hash;
mod load;
mod merge;
mod notes;
mod origin;
mod quality;
mod scaffold;

pub use bindings::{json_schema, rust_bindings, typescript};
pub use check::{Checked, MAX_HINTS, check, check_files, check_packs};
pub use hash::model_hash;
pub use load::{FOLDERS, Pack, Project, SourceFile, StoryHeader, assemble, load};
/// The one diagnostic form of loremaster (XP-6). It lives in `loremaster-model`.
pub use loremaster_model::{Diagnostic, Severity};
pub use merge::{Extension, MergeMode};
pub use notes::{NoteRef, ThreadView, notes, open_todos, threads};
pub use origin::Origin;
pub use quality::{
    Deferred, Finding, PuzzleWidth, QualityCheck, QualityReport, VerbCoverage, quality,
};
pub use scaffold::{START_TEXT, scaffold};

#[cfg(not(target_family = "wasm"))]
pub use load::{missing_asset_files, read_dir};
