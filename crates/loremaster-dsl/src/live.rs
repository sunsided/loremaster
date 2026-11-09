//! [`LiveStory`]: rebuild the world when the YAML files change, for live
//! reload in dev builds. Native only.

use crate::Loremaster;
use loremaster_model::Walkthrough;
use loremaster_project::{Diagnostic, SourceFile};
use loremaster_runtime::World;
use std::sync::Arc;

/// Watches the YAML folders of a [`Loremaster`] and builds a new world when
/// a file changes. Poll it from the host loop, for example twice a second.
///
/// ```no_run
/// use loremaster_dsl::prelude::*;
///
/// let mut live = LiveStory::new(Loremaster::new().yaml("story/"));
/// if let Some(result) = live.poll() {
///     match result {
///         Ok(rebuilt) => { /* session.reload(rebuilt.world, &rebuilt.walkthroughs) */ }
///         Err(diagnostics) => { /* show them, keep the old world */ }
///     }
/// }
/// ```
pub struct LiveStory {
    builder: Loremaster,
    behaviors: Vec<HostBehaviors>,
    files: Vec<Result<Vec<SourceFile>, String>>,
}

/// Registers host behaviors on a new world.
type HostBehaviors = Box<dyn Fn(&mut World)>;

/// A world built after a change, with what a session reload needs.
#[derive(Debug)]
pub struct Rebuilt {
    pub world: Arc<World>,
    pub walkthroughs: Vec<(String, Walkthrough)>,
    /// The warnings of the check. They do not stop the build.
    pub warnings: Vec<Diagnostic>,
}

impl LiveStory {
    /// Read the YAML folders once. A later [`poll`](Self::poll) reports a
    /// change against these files.
    pub fn new(builder: Loremaster) -> Self {
        let files = builder.read_dirs();
        Self {
            builder,
            behaviors: Vec::new(),
            files,
        }
    }

    /// Host behaviors: `f` runs on every new world, after the behaviors of
    /// the Rust packs are registered.
    pub fn behaviors(mut self, f: impl Fn(&mut World) + 'static) -> Self {
        self.behaviors.push(Box::new(f));
        self
    }

    /// `None` when no file changed since the last poll. Otherwise the new
    /// world, or the diagnostics when it does not build: an error of the
    /// check or the compile, or a behavior that nothing registered. The same
    /// broken files are reported once, not on every poll.
    pub fn poll(&mut self) -> Option<Result<Rebuilt, Vec<Diagnostic>>> {
        // shortcut: re-reads all files per poll; switch to notify if stories get large
        let files = self.builder.read_dirs();
        if files == self.files {
            return None;
        }
        self.files = files;
        Some(self.rebuild())
    }

    fn rebuild(&self) -> Result<Rebuilt, Vec<Diagnostic>> {
        let (mut world, project, warnings) = self.builder.clone().built()?;
        for f in &self.behaviors {
            f(&mut world);
        }
        let missing: Vec<Diagnostic> = world
            .missing_behaviors()
            .into_iter()
            .map(|id| {
                Diagnostic::error(
                    "",
                    format!("behavior:{id}"),
                    format!("the behavior `{id}` is not registered"),
                )
            })
            .collect();
        if !missing.is_empty() {
            return Err(missing);
        }
        Ok(Rebuilt {
            world: Arc::new(world),
            walkthroughs: project.walkthroughs,
            warnings,
        })
    }
}
