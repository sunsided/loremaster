//! The turn log of a talk: each change that a dialogue makes gets one line,
//! with its new value, in the order of the changes.

use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{Game, World};
use std::path::Path;
use std::sync::Arc;

fn cantina(dialogue: impl Fn(String) -> String) -> Game {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let mut files: Vec<SourceFile> = read_dir(&dir).unwrap();
    for f in &mut files {
        if f.path == "dialogues/blackbeard.yaml" {
            f.text = dialogue(std::mem::take(&mut f.text));
        }
    }
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut game = Game::new(world);
    game.start();
    game.input("in");
    game.input("talk to figure");
    game
}

/// The log lines of a turn, without the rule lines.
fn changes(game: &mut Game, input: &str) -> Vec<String> {
    game.input(input)
        .log
        .iter()
        .filter(|e| e.kind != "rule")
        .map(|e| format!("{} {}", e.kind, e.text))
        .collect()
}

#[test]
fn a_dialogue_logs_each_change_with_its_new_value() {
    let mut game = cantina(|t| t);
    assert_eq!(
        changes(&mut game, "1"),
        [
            "set knows_blackbeard_name = true",
            "relation blackbeard.trust +1 = 1",
        ]
    );
}

#[test]
fn a_zero_delta_changes_nothing_and_logs_nothing() {
    let mut game = cantina(|t| t.replace("relation: { trust: 1 }", "relation: { trust: 0 }"));
    let lines = changes(&mut game, "1");
    assert_eq!(lines, ["set knows_blackbeard_name = true"], "{lines:?}");
    assert!(!game.state().relations.contains_key("blackbeard.trust"));
}
