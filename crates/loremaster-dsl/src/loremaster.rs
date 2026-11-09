//! [`Loremaster`]: the sources of a story, in order, joined into one world.

use loremaster_project::{
    Diagnostic, Origin, Pack, Project, Severity, SourceFile, assemble, check,
};
use loremaster_runtime::{Behavior, Verdict, World, run_walkthrough};
use std::sync::Arc;

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
    strict: bool,
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

    /// With [`assert_world`](Self::assert_world), fail on warnings too, for
    /// example an active override (DS-15).
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Write each Rust pack to `<dir>/packs/<name>.yaml`, for the generic
    /// editor (`loremaster export-pack`), with the git commit of `dir` when
    /// there is one. `packs/` belongs to this export: after the new packs are
    /// written, the other `.yaml` and `.yml` files there are removed, because
    /// the loader reads every one, and old `.json` packs too. Returns the paths.
    #[cfg(not(target_family = "wasm"))]
    pub fn export_packs(
        &self,
        dir: impl AsRef<std::path::Path>,
    ) -> std::io::Result<Vec<std::path::PathBuf>> {
        let out = dir.as_ref().join("packs");
        let commit = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.as_ref())
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        let mut packs = Vec::new();
        for source in &self.sources {
            let Source::Pack(pack) = source else {
                continue;
            };
            let mut pack = pack.as_ref().clone();
            pack.commit.clone_from(&commit);
            // A world! pack is named after its module path.
            let path = out.join(format!("{}.yaml", pack.name.replace("::", "-")));
            if packs.iter().any(|(p, _)| *p == path) {
                return Err(std::io::Error::other(format!(
                    "two packs are named `{}`",
                    pack.name
                )));
            }
            packs.push((path, pack));
        }
        std::fs::create_dir_all(&out)?;
        // Each pack goes to a temporary file first and is renamed into place,
        // so a failed export never leaves a half-written pack.
        let mut paths = Vec::new();
        for (path, mut pack) in packs {
            let tmp = path.with_extension("yaml.tmp");
            let written =
                std::fs::write(&tmp, pack.to_yaml()).and_then(|()| std::fs::rename(&tmp, &path));
            if let Err(e) = written {
                let _ = std::fs::remove_file(&tmp);
                return Err(e);
            }
            paths.push(path);
        }
        for entry in std::fs::read_dir(&out)? {
            let path = entry?.path();
            let pack = path
                .extension()
                .is_some_and(|e| e == "yaml" || e == "yml" || e == "json");
            if pack && !paths.contains(&path) {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(paths)
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
    /// The behaviors that the Rust packs keep ([`BuildPack::register`](crate::BuildPack::register))
    /// are registered in the world.
    pub fn build(self) -> Result<World, Vec<Diagnostic>> {
        self.built().map(|(world, _, _)| world)
    }

    /// The whole-world test (DS-15): join the packs, check them and run all
    /// walkthroughs. Panics on an error, for example an override whose Rust
    /// part changed, and with [`strict`](Self::strict) on a warning. An
    /// override that is still active stays a warning without it.
    /// [`world_test!`](crate::world_test) makes the `#[test]`.
    pub fn assert_world(self) {
        let strict = self.strict;
        let (world, project, warnings) = match self.built() {
            Ok(built) => built,
            Err(found) => panic!("{}", list(&found)),
        };
        let warnings: Vec<_> = warnings
            .into_iter()
            .filter(|d| strict && d.severity == Severity::Warning)
            .collect();
        assert!(warnings.is_empty(), "strict: {}", list(&warnings));
        let world = Arc::new(world);
        for (key, walk) in &project.walkthroughs {
            let result = run_walkthrough(world.clone(), walk, key);
            assert!(
                !matches!(result.verdict(), Verdict::Failed | Verdict::UnexpectedPass),
                "walkthrough {key}: {:?}",
                result.failures
            );
        }
    }

    /// The files of each YAML folder, read now, in source order. An
    /// unreadable folder gives its error text, so a fix shows as a change.
    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn read_dirs(&self) -> Vec<Result<Vec<SourceFile>, String>> {
        self.sources
            .iter()
            .filter_map(|s| match s {
                Source::Dir(dir) => {
                    Some(loremaster_project::read_dir(dir).map_err(|e| e.to_string()))
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn built(self) -> Result<(World, Project, Vec<Diagnostic>), Vec<Diagnostic>> {
        let behaviors: Vec<Arc<dyn Behavior>> = self
            .sources
            .iter()
            .filter_map(|s| match s {
                Source::Pack(p) => Some(&p.instances),
                _ => None,
            })
            .flatten()
            .filter_map(|i| i.downcast_ref::<Arc<dyn Behavior>>().cloned())
            .collect();
        let (project, warnings) = self.check()?;
        let mut world = World::compile(project.story.clone())
            .map_err(|e| vec![rust_place(&project, Diagnostic::from(&e))])?;
        // Reversed: the world takes the first with an id, and the `rust` block
        // of a replacing machine must win.
        for b in behaviors.into_iter().rev() {
            world.register_shared(b);
        }
        Ok((world, project, warnings))
    }
}

/// A `#[test]` that runs [`Loremaster::assert_world`] on the given sources
/// (the generated whole-world test of DS-15).
///
/// ```text
/// loremaster::dsl::world_test!(Loremaster::new().pack(world()).yaml("story/"));
/// ```
#[macro_export]
macro_rules! world_test {
    ($sources:expr) => {
        #[test]
        fn whole_world() {
            $sources.assert_world();
        }
    };
}

fn list(found: &[Diagnostic]) -> String {
    found
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
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
    for warning in &mut pack.warnings {
        prefixed(&mut warning.file);
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
