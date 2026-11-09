//! Story YAML: the input is up to five files, split at NUL bytes. A story
//! that checks without errors compiles, starts and takes a few commands.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_project::{Severity, SourceFile, check_files};
use loremaster_runtime::{Session, World};
use std::sync::Arc;

const PATHS: [&str; 5] = [
    "story.yaml",
    "rooms/hall.yaml",
    "things/lamp.yaml",
    "rules/rules.yaml",
    "machines/door.yaml",
];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let files = PATHS
        .iter()
        .zip(text.split('\0'))
        .map(|(path, text)| SourceFile {
            path: (*path).into(),
            text: text.into(),
        })
        .collect();
    let checked = check_files(files);
    let errors = checked
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    let Some(project) = checked.project.filter(|_| !errors) else {
        return;
    };
    let Ok(world) = World::compile(project.story) else {
        return;
    };
    let mut game = Session::new(Arc::new(world));
    game.start();
    loremaster_fuzz::play(&mut game, ["look", "take all", "n", "wait", "i"]);
});
