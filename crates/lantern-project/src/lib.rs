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
//! ```
//!
//! An entity's `id` defaults to its file name. Loading works on in-memory
//! files so the same code runs in the browser; [`read_dir`] collects them
//! from disk.

mod check;
mod diagnostic;
mod load;

pub use check::check;
pub use diagnostic::{Diagnostic, Severity};
pub use load::{Project, SourceFile, load};

#[cfg(not(target_family = "wasm"))]
pub use load::read_dir;
