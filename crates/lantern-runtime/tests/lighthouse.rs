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

#[test]
fn rules_report_events_and_state() {
    let mut game = lighthouse();
    game.start();
    game.input("n");
    game.input("open drawer");
    let rules = game.rules();
    let key = rules.iter().find(|r| r.id == "find_iron_key").unwrap();
    assert_eq!(key.when, "the player uses \"open drawer\"");
    assert!(key.fired);
    assert!(!key.condition_holds, "drawer_searched is now true");
    assert!(
        key.then.iter().any(|t| t == "move iron key to cottage"),
        "{:?}",
        key.then
    );
}

#[test]
fn unlisted_things_say_honestly_why() {
    let mut game = lighthouse();
    game.start();
    let parts = game.room_parts();
    let door = parts
        .objects
        .iter()
        .find(|o| o.id == "cottage_door")
        .unwrap();
    assert_eq!(
        door.why.as_deref(),
        Some("not listed: it is the door of the north exit")
    );
    game.input("n");
    let parts = game.room_parts();
    let drawer = parts.objects.iter().find(|o| o.id == "drawer").unwrap();
    assert_eq!(
        drawer.why.as_deref(),
        Some("not listed: the room text mentions it")
    );
}

#[test]
fn author_commands_change_state_directly() {
    use lantern_model::Value;
    use lantern_runtime::AuthorCommand;
    let mut game = lighthouse();
    game.start();
    let turn = game.author(AuthorCommand::Goto {
        room: "lantern_room".into(),
    });
    assert!(turn.plain_text().contains("Lantern Room"));
    assert_eq!(game.scene().room, "lantern_room");
    game.author(AuthorCommand::Set {
        name: "beacon_lit".into(),
        value: Value::Bool(true),
    });
    assert!(
        game.vars()
            .iter()
            .any(|v| v.name == "beacon_lit" && v.value == Value::Bool(true))
    );
    game.author(AuthorCommand::Time {
        time: "22:00".into(),
    });
    assert_eq!(game.scene().phase, "night");
    game.author(AuthorCommand::Give {
        thing: "oil_lamp".into(),
    });
    assert!(game.scene().inventory.iter().any(|t| t.id == "oil_lamp"));
    assert_eq!(
        game.phases().first().map(|p| p.from.as_str()),
        Some("00:00")
    );
}

#[test]
fn rules_say_why_they_matter_here() {
    let mut game = lighthouse();
    game.start();
    let rel = |game: &Game, id: &str| {
        game.rules()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
            .relevant
    };
    // On the shore at 20:30, dusk falls at 21:00.
    assert_eq!(
        rel(&game, "dusk_falls").as_deref(),
        Some("fires in 30 minutes")
    );
    assert_eq!(rel(&game, "find_iron_key"), None);
    game.input("n");
    assert_eq!(
        rel(&game, "find_iron_key").as_deref(),
        Some("uses things in reach")
    );
    game.input("open drawer");
    assert_eq!(
        rel(&game, "find_iron_key").as_deref(),
        Some("fired in the last turn")
    );
}

#[test]
fn inspect_gives_author_facts() {
    let mut game = lighthouse();
    game.start();
    let door = game.inspect_thing("lighthouse_door").unwrap();
    let fact = |i: &lantern_runtime::Inspect, k: &str| {
        i.facts.iter().find(|(l, _)| l == k).map(|(_, v)| v.clone())
    };
    assert_eq!(fact(&door, "locked").as_deref(), Some("true"));
    assert_eq!(
        fact(&door, "door of").as_deref(),
        Some("shore east, stairwell west")
    );
    let lamp = game.inspect_thing("oil_lamp").unwrap();
    assert!(
        lamp.rules.contains(&"lamp_needs_matches".to_string()),
        "{:?}",
        lamp.rules
    );
    let east = game.inspect_exit("east").unwrap();
    assert_eq!(
        fact(&east, "door").as_deref(),
        Some("lighthouse_door, locked")
    );
}
