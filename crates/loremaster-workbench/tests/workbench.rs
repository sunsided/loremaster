//! Tests of the Workbench API: the authoring views over a running game, and
//! the reload into a changed world. The stories come from `../../examples`.

// The cases are in `workbench/`: one test binary, with the helpers below.
mod workbench {
    mod lighthouse;
    mod machines;
    mod reload;
    mod threads;
    mod zork;
}

use loremaster_project::{SourceFile, check, load, read_dir};
use loremaster_runtime::{Choice, Command, Input, Scene, Turn, World};
use loremaster_workbench::Workbench;
use std::path::Path;
use std::sync::Arc;

/// Play as the editor does: whole turns, with the rule log.
pub trait Host {
    fn input(&mut self, text: &str) -> Turn;
    fn act(&mut self, command: Command) -> Turn;
    fn scene(&self) -> Scene;
    fn choices(&self) -> Vec<Choice>;
}

impl Host for Workbench {
    fn input(&mut self, text: &str) -> Turn {
        self.play(Input::Text(text.into()))
    }

    fn act(&mut self, command: Command) -> Turn {
        self.play(Input::Command(command))
    }

    fn scene(&self) -> Scene {
        self.session().scene()
    }

    fn choices(&self) -> Vec<Choice> {
        self.session().choices()
    }
}

/// The project folder of an example.
pub fn root(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// The world of an example, checked.
pub fn world(name: &str) -> Arc<World> {
    let project = load(read_dir(&root(name)).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    Arc::new(World::compile(project.story).unwrap())
}

/// The world of project files given as (path, text).
pub fn world_of(files: &[(&str, &str)]) -> Arc<World> {
    let files = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    Arc::new(World::compile(project.story).unwrap())
}

pub fn example(name: &str) -> Workbench {
    Workbench::new(world(name))
}

pub fn lighthouse() -> Workbench {
    example("lighthouse")
}
