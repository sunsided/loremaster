//! Save games: the envelope (ST-4), migration and refusal (ST-5), and a
//! restore into a changed story (ST-6).

use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{Game, SAVE_FORMAT, SAVE_VERSION, SaveError, World};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

fn lighthouse_files() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

fn world(files: Vec<SourceFile>) -> Arc<World> {
    Arc::new(World::compile(load(files).unwrap().story).unwrap())
}

/// A lighthouse game after a few moves, and its save as JSON.
fn played() -> (Arc<World>, Game, Value) {
    let world = world(lighthouse_files());
    let mut game = Game::new(world.clone());
    game.start();
    for command in ["n", "open drawer", "take key"] {
        game.input(command);
    }
    let save = serde_json::to_value(game.save()).unwrap();
    (world, game, save)
}

fn error(world: Arc<World>, save: Value) -> SaveError {
    match Game::load_save(world, save) {
        Ok(_) => panic!("the save loaded"),
        Err(e) => e,
    }
}

#[test]
fn a_save_is_an_envelope() {
    let (_, _, save) = played();
    assert_eq!(save["format"], SAVE_FORMAT);
    assert_eq!(save["version"], SAVE_VERSION);
    assert_eq!(save["story"]["id"], "the-lighthouse");
    assert_eq!(save["story"]["hash"].as_str().unwrap().len(), 16);
    assert_eq!(save["state"]["room"], "cottage");
}

#[test]
fn a_save_restores_into_the_same_story() {
    let (world, game, save) = played();
    let restored = Game::load_save(world, save).unwrap();
    assert!(!restored.story_changed);
    assert_eq!(restored.from_version, SAVE_VERSION);
    assert!(restored.dropped.is_empty(), "{:?}", restored.dropped);
    assert_eq!(restored.game.state(), game.state());
}

#[test]
fn a_bare_state_of_version_0_migrates() {
    let (world, game, save) = played();
    let restored = Game::load_save(world, save["state"].clone()).unwrap();
    assert_eq!(restored.from_version, 0);
    // Version 0 has no story stamp, so the restore checks each value.
    assert!(restored.story_changed);
    assert!(restored.dropped.is_empty(), "{:?}", restored.dropped);
    assert_eq!(restored.game.state(), game.state());
}

#[test]
fn a_newer_version_is_refused_with_both_versions() {
    let (world, _, mut save) = played();
    save["version"] = json!(SAVE_VERSION + 1);
    let e = error(world, save);
    assert_eq!(
        e,
        SaveError::NewerVersion {
            save: SAVE_VERSION + 1,
            supported: SAVE_VERSION
        }
    );
    let message = e.to_string();
    assert!(
        message.contains(&format!("version {}", SAVE_VERSION + 1)),
        "{message}"
    );
    assert!(
        message.contains(&format!("version {SAVE_VERSION}")),
        "{message}"
    );
}

#[test]
fn other_formats_stories_and_broken_saves_are_refused() {
    let (world, _, save) = played();
    assert!(matches!(
        error(
            world.clone(),
            json!({ "format": "zork-save", "version": 1 })
        ),
        SaveError::UnknownFormat(_)
    ));
    assert!(matches!(
        error(world.clone(), json!([1, 2, 3])),
        SaveError::UnknownFormat(_)
    ));
    let mut other = save.clone();
    other["story"]["id"] = json!("the-tower");
    assert!(matches!(
        error(world.clone(), other),
        SaveError::OtherStory { .. }
    ));
    let mut broken = save;
    broken["state"]["facts"] = json!("not a map");
    assert!(matches!(error(world, broken), SaveError::Invalid(_)));
}

#[test]
fn a_changed_story_keeps_what_is_valid_and_lists_the_rest() {
    let (_, game, save) = played();
    // The new version of the story: no matches, no `drawer_searched`, and a new timer.
    let mut files = lighthouse_files();
    files.retain(|f| f.path != "things/matches.yaml");
    for f in &mut files {
        if f.path == "story.yaml" {
            f.text = f
                .text
                .replace(
                    "  - { id: drawer_searched, type: bool, initial: false }\n",
                    "",
                )
                .replace(
                    "vars:\n",
                    "timers:\n  - { id: tide, turns: 30, running: true }\nvars:\n",
                );
        }
        if f.path == "rules/drawer.yaml" {
            f.text = f
                .text
                .replace("  when: not drawer_searched\n", "")
                .replace("    - set: { drawer_searched: true }\n", "");
        }
        if f.path == "rules/lamp.yaml" {
            f.text = f
                .text
                .replace("not matches.carried", "false")
                .replace("matches.carried", "true");
        }
    }
    let changed = world(files);
    let restored = Game::load_save(changed, save).unwrap();
    assert!(restored.story_changed);
    assert_eq!(
        restored.dropped,
        ["fact:drawer_searched", "thing:matches"],
        "{:?}",
        restored.dropped
    );
    let state = restored.game.state();
    assert_eq!(state.room, game.state().room);
    assert_eq!(
        state.locations.get("iron_key"),
        game.state().locations.get("iron_key")
    );
    // The new timer starts as at the start of a game.
    assert_eq!(state.timers.get("tide"), Some(&30));
}

#[test]
fn a_timer_that_ran_out_does_not_start_again() {
    let files = || {
        vec![
            SourceFile {
                path: "story.yaml".into(),
                text:
                    "title: Fuse\nstart: hall\ntimers:\n  - { id: fuse, turns: 1, running: true }\n"
                        .into(),
            },
            SourceFile {
                path: "rooms/hall.yaml".into(),
                text: "name: Hall\ndescription: [\"A hall.\"]\n".into(),
            },
        ]
    };
    let mut game = Game::new(world(files()));
    game.start();
    game.input("wait");
    assert!(game.state().timers.is_empty(), "{:?}", game.state().timers);
    let save = serde_json::to_value(game.save()).unwrap();
    // A changed story: a new room. The fuse ran out before the save.
    let mut changed = files();
    changed.push(SourceFile {
        path: "rooms/yard.yaml".into(),
        text: "name: Yard\ndescription: [\"A yard.\"]\n".into(),
    });
    let restored = Game::load_save(world(changed), save).unwrap();
    assert!(restored.story_changed);
    assert!(restored.game.state().timers.is_empty());
}

fn files_of(dir: &str) -> Vec<SourceFile> {
    read_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(dir),
    )
    .unwrap()
}

#[test]
fn a_thing_in_a_holder_that_is_no_container_any_more_goes_back_to_its_start() {
    let files = |container: bool| {
        vec![
            SourceFile {
                path: "story.yaml".into(),
                text: "title: Box\nstart: hall\n".into(),
            },
            SourceFile {
                path: "rooms/hall.yaml".into(),
                text: "name: Hall\ndescription: [\"A hall.\"]\n".into(),
            },
            SourceFile {
                path: "things/box.yaml".into(),
                text: format!(
                    "name: box\nlocation: {{ room: hall }}\ncontainer: {container}\nprops: {{ open: true }}\n"
                ),
            },
            SourceFile {
                path: "things/coin.yaml".into(),
                text: "name: coin\nlocation: { room: hall }\nportable: true\n".into(),
            },
        ]
    };
    let mut game = Game::new(world(files(true)));
    game.start();
    game.input("take coin");
    game.input("put coin in box");
    assert_eq!(
        serde_json::to_value(game.state().locations.get("coin")).unwrap(),
        json!({ "in": "box" })
    );
    let save = serde_json::to_value(game.save()).unwrap();
    let restored = Game::load_save(world(files(false)), save).unwrap();
    assert_eq!(
        serde_json::to_value(restored.game.state().locations.get("coin")).unwrap(),
        json!({ "room": "hall" })
    );
}

#[test]
fn the_back_stack_of_a_talk_keeps_only_nodes_that_still_exist() {
    let world_of = |files: Vec<SourceFile>| world(files);
    let mut game = Game::new(world_of(files_of("cantina")));
    game.start();
    game.input("in");
    game.input("talk to figure");
    game.input("1");
    let talk = game.state().talk.clone().expect("in a talk");
    assert_eq!(
        (talk.node.as_str(), talk.back.clone()),
        ("names", vec!["meet".to_string()])
    );
    let save = serde_json::to_value(game.save()).unwrap();

    // The new version of the dialogue calls its first node `greet`.
    let mut files = files_of("cantina");
    for f in &mut files {
        if f.path == "dialogues/blackbeard.yaml" {
            f.text = f
                .text
                .replace("start: [again, meet]", "start: [again, greet]")
                .replace("\n  meet:\n", "\n  greet:\n");
        }
    }
    let restored = Game::load_save(world_of(files), save).unwrap();
    let talk = restored
        .game
        .state()
        .talk
        .clone()
        .expect("still in the talk");
    assert_eq!(talk.node, "names");
    assert!(talk.back.is_empty(), "{:?}", talk.back);
    assert_eq!(restored.dropped, ["dialogue:blackbeard#meet"]);
}
