use lantern_project::{check, load, read_dir};
use lantern_runtime::{Game, World};
use std::path::Path;
use std::sync::Arc;

fn lighthouse() -> Game {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&root).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    Game::new(Arc::new(World::compile(project.story).unwrap()))
}

#[test]
fn the_lighthouse_can_be_won() {
    let mut game = lighthouse();
    let mut transcript = game.start().plain_text();
    for cmd in [
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
        "light beacon",
    ] {
        let turn = game.input(cmd);
        transcript.push_str("\n\n");
        transcript.push_str(&turn.plain_text());
    }
    println!("{transcript}");
    assert!(game.state().ended, "{transcript}");
    assert!(transcript.contains("Under a fold of oilcloth lies an iron key."));
    assert!(transcript.contains("The last of the light drains out of the sky."));
}

#[test]
fn the_stairwell_is_dark_without_a_lamp() {
    let mut game = lighthouse();
    game.start();
    for cmd in [
        "n",
        "open drawer",
        "take key",
        "s",
        "unlock iron door with key",
    ] {
        game.input(cmd);
    }
    let turn = game.input("e");
    assert!(
        turn.plain_text().contains("pitch dark"),
        "{}",
        turn.plain_text()
    );
}

#[test]
fn playthroughs_are_deterministic() {
    let run = || {
        let mut g = lighthouse();
        let mut out = g.start().plain_text();
        for _ in 0..6 {
            out.push_str(&g.input("look").plain_text());
        }
        out
    };
    assert_eq!(run(), run());
}

#[test]
fn the_map_has_every_room_and_one_link_per_pair_of_exits() {
    let mut game = lighthouse();
    game.start();
    let map = game.map();
    assert_eq!(map.rooms.len(), 4);
    assert_eq!(map.links.len(), 3, "{:#?}", map.links);
    assert!(
        map.rooms
            .iter()
            .any(|r| r.id == "shore" && r.current && r.visited)
    );
    assert!(map.rooms.iter().any(|r| r.id == "cottage" && !r.visited));
}
