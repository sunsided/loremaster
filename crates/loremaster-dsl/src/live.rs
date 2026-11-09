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
    /// A folder could not be read at the start, and no poll has said so yet.
    report_unreadable: bool,
}

/// Registers host behaviors on a new world.
type HostBehaviors = Box<dyn Fn(&mut World) + Send + Sync>;

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
    /// change against these files; the first poll reports a folder that
    /// could not be read.
    pub fn new(builder: Loremaster) -> Self {
        let files = builder.read_dirs();
        let report_unreadable = files.iter().any(Result::is_err);
        Self {
            builder,
            behaviors: Vec::new(),
            files,
            report_unreadable,
        }
    }

    /// Host behaviors: `f` runs on every new world, after the behaviors of
    /// the Rust packs are registered.
    pub fn behaviors(mut self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
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
        let report_unreadable = std::mem::take(&mut self.report_unreadable);
        if files == self.files && !report_unreadable {
            return None;
        }
        let rebuilt = self.rebuild(&files);
        self.files = files;
        Some(rebuilt)
    }

    fn rebuild(
        &self,
        files: &[Result<Vec<SourceFile>, String>],
    ) -> Result<Rebuilt, Vec<Diagnostic>> {
        let (mut world, project, warnings) = self.builder.with_read_dirs(files).built()?;
        for f in &self.behaviors {
            f(&mut world);
        }
        let missing: Vec<Diagnostic> = world
            .missing_behaviors()
            .into_iter()
            .map(|id| {
                Diagnostic::error(
                    "story.yaml",
                    "behaviors",
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
