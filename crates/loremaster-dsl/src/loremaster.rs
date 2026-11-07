//! [`Loremaster`]: the sources of a story, in order, joined into one world.

use loremaster_project::{
    Diagnostic, Origin, Pack, Project, Severity, SourceFile, assemble, check,
};
use loremaster_runtime::World;

/// The sources of a story, in a fixed order: packs from Rust and YAML
/// folders. A later pack can extend or replace an entity of an earlier one.
///
/// With one YAML source, its paths stay relative to its folder
/// (`rooms/shore.yaml`), as in a YAML project. With more than one, each
/// path starts with its source, so that two folders with the same file do
/// not mix: the folder as given (`story/rooms/shore.yaml`), or `yaml<n>`
/// for the `n`-th YAML source from [`yaml_files`](Self::yaml_files).
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

    /// A YAML folder on disk. It is read when the packs are joined. Not on
    /// `wasm32`: there, read the files and give them to
    /// [`yaml_files`](Self::yaml_files).
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
        let yaml_sources = self
            .sources
            .iter()
            .filter(|s| !matches!(s, Source::Pack(_)))
            .count();
        let mut packs = Vec::new();
        let mut errors = Vec::new();
        let mut n = 0;
        for source in self.sources {
            if !matches!(source, Source::Pack(_)) {
                n += 1;
            }
            let (files, root) = match source {
                Source::Pack(pack) => {
                    packs.push(*pack);
                    continue;
                }
                Source::Files(files) => (files, format!("yaml{n}")),
                #[cfg(not(target_family = "wasm"))]
                Source::Dir(dir) => {
                    let root = dir.display().to_string().replace('\\', "/");
                    let root = root.trim_end_matches('/').to_string();
                    match loremaster_project::read_dir(&dir) {
                        Ok(files) => (files, root),
                        Err(e) => {
                            errors.push(Diagnostic::error(
                                root,
                                "",
                                format!("cannot read the YAML folder: {e}"),
                            ));
                            continue;
                        }
                    }
                }
            };
            let prefix = (yaml_sources > 1).then_some(root);
            match Pack::from_files(files) {
                Ok(mut pack) => {
                    if let Some(prefix) = &prefix {
                        prefix_paths(&mut pack, prefix);
                    }
                    packs.push(pack);
                }
                Err(found) => errors.extend(found.into_iter().map(|mut d| {
                    if let Some(prefix) = &prefix {
                        d.file = format!("{prefix}/{}", d.file);
                    }
                    d
                })),
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
        let diagnostics: Vec<Diagnostic> = check(&project)
            .into_iter()
            .map(|d| rust_place(&project, d))
            .collect();
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
            .map_err(|e| vec![rust_place(&project, Diagnostic::from(&e))])
    }
}

/// Start each path of a YAML pack with `prefix`.
fn prefix_paths(pack: &mut Pack, prefix: &str) {
    let prefixed = |path: &mut String| *path = format!("{prefix}/{path}");
    for file in &mut pack.files {
        prefixed(&mut file.path);
    }
    for (key, _) in &mut pack.walkthroughs {
        prefixed(key);
    }
    let origins = pack
        .origins
        .values_mut()
        .chain(pack.extensions.iter_mut().map(|e| &mut e.origin));
    for origin in origins {
        if let Origin::Yaml { path } = origin {
            prefixed(path);
        }
    }
}

/// Give a diagnostic the file of its entity and, for an entity from Rust,
/// the line and column of the call that made it. `check` does this for the
/// entities; this adds compile errors (their `at` starts with the entity
/// key) and the story header, which the checks name `story.yaml`.
fn rust_place(project: &Project, diagnostic: Diagnostic) -> Diagnostic {
    let mut d = project.locate(diagnostic);
    if d.line.is_some() {
        return d;
    }
    // The entity key, as `Project::locate` finds it: `room:shore/exits[0]`
    // is `room:shore`, `story:intro` is `story`.
    let key = d.at.split(['/', '#']).next().unwrap_or_default();
    let key = if key.starts_with("story:") {
        "story"
    } else {
        key
    };
    let origin = match project.origins.get(key) {
        Some(origin) => Some(origin),
        None if d.file == "story.yaml" && project.file("story.yaml").is_none() => {
            project.origins.get("story")
        }
        None => None,
    };
    if let Some(Origin::Rust {
        file, line, column, ..
    }) = origin
        && (*file == d.file || d.file == "story.yaml")
    {
        d.file = file.clone();
        d.line = Some(*line);
        d.column = Some(*column);
    }
    d
}
