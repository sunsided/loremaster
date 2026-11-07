//! The `end` effect ends the game with an ending from `endings/`.

use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{Game, State, World};
use std::path::Path;
use std::sync::Arc;

fn lighthouse() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

const WIN: &[&str] = &[
    "n",
    "open drawer",
    "take key",
    "take lamp",
    "take matches",
    "light lamp",
    "s",
    "unlock iron door with key",
    "e",
    "up",
];

#[test]
fn the_game_shows_the_ending_and_keeps_its_id() {
    let world = Arc::new(World::compile(load(lighthouse()).unwrap().story).unwrap());
    let mut game = Game::new(world);
    game.start();
    for command in WIN {
        game.input(command);
    }
    assert_eq!(game.state().ending, None);
    let text = game.input("light beacon").plain_text();
    assert!(text.contains("The beacon is lit"), "{text}");
    assert!(text.contains("masthead light"), "{text}");
    assert!(game.state().ended);
    assert_eq!(
        game.state().ending.as_ref().map(|e| e.as_str()),
        Some("beacon_lit")
    );
}

#[test]
fn an_unknown_ending_does_not_compile() {
    let mut files = lighthouse();
    files.retain(|f| !f.path.starts_with("endings/"));
    let project = load(files).unwrap();
    let err = World::compile(project.story.clone()).unwrap_err();
    // As a diagnostic, the compile error points at the file of its rule (XP-6).
    let d = project.locate(loremaster_model::Diagnostic::from(&err));
    assert_eq!(d.file, "rules/beacon.yaml", "{d}");
    assert!(d.is_error());
    assert!(
        err.to_string().contains("unknown ending `beacon_lit`"),
        "{err}"
    );
}

#[test]
fn a_save_without_an_ending_still_loads() {
    let world = Arc::new(World::compile(load(lighthouse()).unwrap().story).unwrap());
    let mut game = Game::new(world.clone());
    game.start();
    let mut save = serde_json::to_value(game.state()).unwrap();
    assert!(save.get("ending").is_none(), "no ending yet: {save:#}");
    save.as_object_mut().unwrap().remove("ending");
    let state: State = serde_json::from_value(save).unwrap();
    assert_eq!(state.ending, None);
    Game::restore(world, state);
}
