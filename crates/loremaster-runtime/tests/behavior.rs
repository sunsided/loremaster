//! Behaviors in Rust: the Zork thief steals, replays the same way, and keeps
//! all of its state in the save game.

#[path = "../../../examples/zork1/thief.rs"]
mod thief;

use loremaster_model::{Id, Location};
use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Game, State, World};
use std::sync::Arc;

fn world(with_thief: bool) -> Arc<World> {
    let files = [
        ("story.yaml", "title: Cave\nstart: cave\nbehaviors: [thief]\n"),
        (
            "rooms/cave.yaml",
            "name: Cave\ntags: [dark]\ndescription: [\"A cave.\"]\nexits:\n  - { direction: east, to: hole }\n",
        ),
        (
            "rooms/hole.yaml",
            "name: Hole\ntags: [dark]\ndescription: [\"A hole.\"]\nexits:\n  - { direction: west, to: cave }\n",
        ),
        (
            "things/lamp.yaml",
            "name: lamp\nlocation: carried\nportable: true\nprops: { lit: true }\n",
        ),
        ("things/gem.yaml", "name: gem\nlocation: carried\nportable: true\n"),
        (
            "things/thief.yaml",
            "name: thief\nlocation: nowhere\ncontainer: true\nprops: { open: true }\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap();
    let mut world = World::compile(project.story).unwrap();
    if with_thief {
        world.register(thief::Thief {
            thief: Id::new("thief"),
            home: Id::new("cave"),
            treasures: vec![Id::new("gem")],
        });
    }
    Arc::new(world)
}

/// Wait until the gem is gone, at most 60 turns. Returns the turn it went and the state.
fn wait_for_theft(world: Arc<World>) -> (Option<u32>, State) {
    let mut game = Game::new(world);
    game.start();
    for _ in 0..60 {
        game.input("wait");
        if game.state().location_of("gem") == Some(&Location::In(Id::new("thief"))) {
            return (Some(game.state().turns), game.state().clone());
        }
    }
    (None, game.state().clone())
}

#[test]
fn the_thief_steals_and_replays_the_same_way() {
    let (when, _) = wait_for_theft(world(true));
    assert!(
        when.is_some(),
        "the thief did not steal the gem in 60 turns"
    );
    assert_eq!(when, wait_for_theft(world(true)).0);
}

#[test]
fn without_the_code_the_story_plays_without_the_thief() {
    let w = world(false);
    assert_eq!(w.missing_behaviors(), [&Id::new("thief")]);
    let (when, state) = wait_for_theft(w);
    assert_eq!(when, None);
    assert_eq!(state.location_of("thief"), Some(&Location::Nowhere));
}

#[test]
fn a_saved_game_goes_on_like_the_game_it_saved() {
    let w = world(true);
    let mut game = Game::new(w.clone());
    game.start();
    for _ in 0..5 {
        game.input("wait");
    }
    let save = serde_json::to_string(game.state()).unwrap();
    let mut restored = Game::restore(w, serde_json::from_str(&save).unwrap());
    for _ in 0..30 {
        let a = game.input("wait").plain_text();
        let b = restored.input("wait").plain_text();
        assert_eq!(a, b);
    }
    assert_eq!(game.state(), restored.state());
}

#[test]
fn the_zork_thief_knows_the_things_of_zork() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/zork1");
    let project = load(loremaster_project::read_dir(&root).unwrap()).unwrap();
    assert!(project.story.behaviors.contains(&Id::new("thief")));
    let t = thief::Thief::zork();
    for id in t.treasures.iter().chain([&t.thief]) {
        assert!(
            project.story.thing(id.as_str()).is_some(),
            "no thing `{id}`"
        );
    }
    assert!(project.story.room(t.home.as_str()).is_some());
    let mut world = World::compile(project.story).unwrap();
    world.register(t);
    assert!(world.missing_behaviors().is_empty());
}
