//! [`Loremaster`]: the sources of a story, in order, joined into one world.

use loremaster_project::{Diagnostic, Pack, Project, Severity, SourceFile, assemble, check};
use loremaster_runtime::World;

/// The sources of a story, in a fixed order: packs from Rust and YAML
/// folders. A later pack can extend or replace an entity of an earlier one.
///
/// ```no_run
/// use loremaster_dsl::prelude::*;
///
/// fn world() -> Pack {
///     let mut p = Pack::new("world");
///     p.story("The Lighthouse", "shore");
///     p.room("shore", "The Shore").text("Shingle crunches underfoot.");
///     p
/// }
///
/// let world = Loremaster::new()
///     .pack(world())
///     .yaml("story/")
///     .build()
///     .expect("the story checks");
/// ```
#[derive(Debug, Clone, Default)]
pub struct Loremaster {
    sources: Vec<Source>,
}

#[derive(Debug, Clone)]
enum Source {
    Pack(Box<Pack>),
    Files(Vec<SourceFile>),
    #[cfg(not(target_family = "wasm"))]
    Dir(std::path::PathBuf),
}

impl Loremaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// A pack, for example from a builder function.
    pub fn pack(mut self, pack: Pack) -> Self {
        self.sources.push(Source::Pack(Box::new(pack)));
        self
    }

    /// A YAML folder on disk. It is read when the packs are joined.
    #[cfg(not(target_family = "wasm"))]
    pub fn yaml(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.sources.push(Source::Dir(dir.into()));
        self
    }

    /// The files of a YAML folder, already in memory (for the browser, or
    /// `include_str!`). Paths use `/` and are relative to the folder.
    pub fn yaml_files(mut self, files: Vec<SourceFile>) -> Self {
        self.sources.push(Source::Files(files));
        self
    }

    /// Read the sources and join the packs in their order
    /// ([`assemble`]). The project is not checked yet.
    pub fn assemble(self) -> Result<Project, Vec<Diagnostic>> {
        let mut packs = Vec::new();
        let mut errors = Vec::new();
        for source in self.sources {
            let files = match source {
                Source::Pack(pack) => {
                    packs.push(*pack);
                    continue;
                }
                Source::Files(files) => files,
                #[cfg(not(target_family = "wasm"))]
                Source::Dir(dir) => match loremaster_project::read_dir(&dir) {
                    Ok(files) => files,
                    Err(e) => {
                        let file = dir.display().to_string();
                        errors.push(Diagnostic::error(
                            file,
                            "",
                            format!("cannot read the YAML folder: {e}"),
                        ));
                        continue;
                    }
                },
            };
            match Pack::from_files(files) {
                Ok(pack) => packs.push(pack),
                Err(found) => errors.extend(found),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        assemble(packs)
    }

    /// Join the packs and [`check`] the project. Returns the project and the
    /// warnings, or all diagnostics when one of them is an error.
    pub fn check(self) -> Result<(Project, Vec<Diagnostic>), Vec<Diagnostic>> {
        let project = self.assemble()?;
        let diagnostics = check(&project);
        if diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return Err(diagnostics);
        }
        Ok((project, diagnostics))
    }

    /// Join the packs, check the project and compile the world. Warnings of
    /// the check do not stop the build; [`check`](Self::check) returns them.
    pub fn build(self) -> Result<World, Vec<Diagnostic>> {
        let (project, _warnings) = self.check()?;
        World::compile(project.story.clone())
            .map_err(|e| vec![project.locate(Diagnostic::from(&e))])
    }
}
