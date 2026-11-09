//! A story bundle: the compiled-in copy of a story for a host, with its
//! walkthroughs for the hint ladder and a format version, in one value.
//!
//! A host's `build.rs` loads the project and writes
//! `Bundle::new(project.story, project.walkthroughs)` as JSON; at run time
//! `Bundle::from_json` reads it back and `Bundle::session` starts a game
//! with the hints set.

use crate::{CompileError, HintBook, SaveError, Session, World};
use loremaster_model::{Story, Walkthrough};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The `format` of a bundle.
pub const BUNDLE_FORMAT: &str = "loremaster-bundle";
/// The newest bundle version.
pub const BUNDLE_VERSION: u32 = 1;

/// A story with its walkthroughs, as a host ships it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    /// The loremaster version that wrote the bundle, for the host's report.
    pub engine: String,
    pub story: Story,
    /// The walkthroughs by name; their steps make the hint ladder.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub walkthroughs: Vec<(String, Walkthrough)>,
}

impl Bundle {
    pub fn new(story: Story, walkthroughs: Vec<(String, Walkthrough)>) -> Self {
        Self {
            format: BUNDLE_FORMAT.into(),
            version: BUNDLE_VERSION,
            engine: env!("CARGO_PKG_VERSION").into(),
            story,
            walkthroughs,
        }
    }

    /// Read a bundle. Refuses data of another format or a newer version.
    pub fn from_json(json: &str) -> Result<Self, SaveError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| SaveError::Invalid(e.to_string()))?;
        let format = value.get("format").and_then(|f| f.as_str()).unwrap_or("");
        if format != BUNDLE_FORMAT {
            return Err(SaveError::UnknownFormat(format.into()));
        }
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version > u64::from(BUNDLE_VERSION) {
            return Err(SaveError::NewerVersion {
                save: u32::try_from(version).unwrap_or(u32::MAX),
                supported: BUNDLE_VERSION,
            });
        }
        serde_json::from_value(value).map_err(|e| SaveError::Invalid(e.to_string()))
    }

    /// The bundle as JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a bundle serializes")
    }

    /// Compile the story and start a session with the hint ladder of the
    /// walkthroughs. Call [`Session::start`] next. A story with Rust
    /// behaviors compiles `story` itself, registers them on the `World`, and
    /// gives the walkthroughs to [`Session::set_hints_from`].
    pub fn session(&self) -> Result<Session, CompileError> {
        let world = Arc::new(World::compile(self.story.clone())?);
        let mut session = Session::new(world.clone());
        session.set_hints(HintBook::new(world, &self.walkthroughs));
        Ok(session)
    }
}
