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

#[test]
fn inline_branches_are_marked_active_or_not() {
    use lantern_runtime::SourceRole;
    let mut game = lighthouse();
    game.start();
    let text_of = |game: &Game, active: bool| -> Vec<String> {
        game.assembly()[0].variants[0]
            .iter()
            .filter(|p| p.role == SourceRole::Text && p.active == active)
            .map(|p| p.text.clone())
            .collect()
    };
    // First visit, at dusk: `{first: ...}` applies, `{night: ...}` takes the second branch.
    assert!(text_of(&game, true).contains(&", its windows dark".to_string()));
    assert!(text_of(&game, false).contains(&"black against the stars".to_string()));
    // Second visit: `{first: ...}` no longer applies.
    game.input("n");
    game.input("s");
    assert!(text_of(&game, false).contains(&", its windows dark".to_string()));
}

#[test]
fn once_only_text_on_screen_counts_as_shown() {
    let mut game = lighthouse();
    game.start();
    let turn = game.input("n");
    assert!(turn.plain_text().contains("Hal's oilskin"));
    let oilskin = game
        .assembly()
        .into_iter()
        .find(|f| f.text.starts_with("Hal's oilskin"))
        .unwrap();
    assert!(oilskin.shown, "{oilskin:#?}");
    // After leaving and coming back it is used up.
    game.input("s");
    game.input("n");
    let oilskin = game
        .assembly()
        .into_iter()
        .find(|f| f.text.starts_with("Hal's oilskin"))
        .unwrap();
    assert!(!oilskin.shown);
    assert_eq!(oilskin.why.as_deref(), Some("already shown once"));
}

#[test]
fn room_parts_list_things_and_exits() {
    let mut game = lighthouse();
    game.start();
    game.input("n");
    let parts = game.room_parts();
    let names: Vec<&str> = parts.objects.iter().map(|o| o.label.as_str()).collect();
    assert!(
        names.contains(&"oil lamp") && names.contains(&"drawer"),
        "{names:?}"
    );
    let drawer = parts.objects.iter().find(|o| o.id == "drawer").unwrap();
    assert!(!drawer.shown);
    assert_eq!(parts.exits.len(), 1);
    assert_eq!(
        parts.exits[0].detail.as_deref(),
        Some("door: cottage door, open")
    );
}
