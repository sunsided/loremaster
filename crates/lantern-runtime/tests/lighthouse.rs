use lantern_project::{check, load, read_dir};
use lantern_runtime::{Game, World};
use std::path::Path;
use std::sync::Arc;

fn lighthouse() -> Game {
    example("lighthouse")
}

fn example(name: &str) -> Game {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
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

#[test]
fn look_around_teases_on_repeats_and_only_when_asked() {
    let mut game = lighthouse();
    let opening = game.start().plain_text();
    assert!(
        !opening.contains("You look around"),
        "the automatic description is not framed"
    );
    let first = game.input("look").plain_text();
    assert!(
        first.contains("You look around.") && first.contains("The Shore"),
        "{first}"
    );
    assert!(
        game.input("look")
            .plain_text()
            .contains("You look around, again.")
    );
    let frame = game.last_frame().expect("the look frame was used");
    assert_eq!(
        (frame.verb.as_str(), frame.scope.as_str(), frame.repeat),
        ("look", "story", 2)
    );
    let shown: Vec<usize> = frame.parts[0]
        .fragments
        .iter()
        .filter(|f| f.shown)
        .map(|f| f.index)
        .collect();
    assert_eq!(shown, vec![1]);
    assert!(
        game.input("l")
            .plain_text()
            .contains("You keep looking around")
    );
    game.input("wait");
    assert!(
        game.input("look").plain_text().contains("You look around."),
        "a different command resets the repeat"
    );
}

#[test]
fn names_change_with_what_the_player_knows() {
    let mut game = example("cantina");
    game.start();
    let inside = game.input("in").plain_text();
    assert!(inside.contains("A shadow looms in the corner"), "{inside}");
    assert!(
        game.input("talk to blackbeard")
            .plain_text()
            .contains("You see no blackbeard here.")
    );
    assert!(
        game.input("talk to shadow")
            .plain_text()
            .contains("grim-looking")
    );
    assert!(
        game.input("look")
            .plain_text()
            .contains("The grim-looking man is still staring")
    );
    assert!(
        game.input("talk to man")
            .plain_text()
            .contains("They call me Blackbeard")
    );
    game.input("out");
    let back = game.input("in").plain_text();
    assert!(
        back.contains("Blackbeard is still sitting in the corner"),
        "{back}"
    );
    assert!(!back.contains("The Blackbeard"));
    assert!(
        game.input("x shadow")
            .plain_text()
            .contains("weathered face"),
        "earlier names still work"
    );
    let save = serde_json::to_string(game.state()).unwrap();
    assert!(
        save.contains("\"talked\""),
        "counters are in the save state"
    );
}

#[test]
fn the_explorer_reaches_the_ending_and_reports_coverage() {
    use lantern_runtime::{Budget, explore};
    let game = lighthouse();
    let result = explore(&game, Budget::default());
    assert!(!result.truncated, "{} states", result.states);
    assert!(
        result.rooms.iter().all(|r| r.reached),
        "every room is reachable"
    );
    assert_eq!(result.endings.len(), 1, "{:#?}", result.endings);
    let path = &result.endings[0].path;
    assert!(
        path.iter().any(|c| c.starts_with("turn_on lens")),
        "{path:?}"
    );
    assert_eq!(result.stuck, 0, "no soft-lock: {:?}", result.stuck_path);
    let shore = result.rooms.iter().find(|r| r.id == "shore").unwrap();
    assert!(shore.variants.len() >= 2, "dusk and night read differently");
    println!(
        "{} states, never shown: {:?}, never fired: {:?}",
        result.states, result.never_shown, result.never_fired
    );
}

#[test]
fn the_room_matrix_renders_every_combination_of_the_room() {
    use lantern_runtime::{AuthorCommand, room_matrix};
    let mut game = lighthouse();
    game.start();
    let matrix = room_matrix(&game, "shore").unwrap();
    let names: Vec<&str> = matrix.dimensions.iter().map(|d| d.name.as_str()).collect();
    println!("{names:?} -> {} combinations", matrix.combinations);
    assert!(names.contains(&"time.phase"), "{names:?}");
    assert!(names.contains(&"first_visit"), "{names:?}");
    assert_eq!(
        matrix.cells.len(),
        matrix.combinations.min(lantern_runtime::MAX_CELLS)
    );
    let texts: std::collections::BTreeSet<&str> =
        matrix.cells.iter().map(|c| c.text.as_str()).collect();
    assert!(texts.len() >= 2, "the text changes with the state");
    assert!(
        game.state().room.as_str() == "shore",
        "the game itself does not change"
    );

    // "Try this" puts the game into one combination.
    let cell = &matrix.cells[matrix.cells.len() - 1];
    let values: Vec<(String, String)> = matrix
        .dimensions
        .iter()
        .zip(&cell.values)
        .map(|(d, v)| (d.name.clone(), v.clone()))
        .collect();
    let turn = game.author(AuthorCommand::Situation {
        room: "shore".into(),
        values,
    });
    let prose = turn.plain_text();
    let first_line = cell.text.lines().next().unwrap();
    assert!(prose.contains(first_line), "{prose}\n---\n{}", cell.text);
}

#[test]
fn the_room_matrix_follows_name_stages_and_characters() {
    use lantern_runtime::room_matrix;
    let mut game = example("cantina");
    game.start();
    let matrix = room_matrix(&game, "cantina").unwrap();
    let names: Vec<&str> = matrix.dimensions.iter().map(|d| d.name.as_str()).collect();
    println!("{names:?} -> {} combinations", matrix.combinations);
    assert!(names.contains(&"blackbeard.talked"), "{names:?}");
    assert!(names.contains(&"knows_blackbeard_name"), "{names:?}");
    assert!(
        matrix
            .cells
            .iter()
            .any(|c| c.text.contains("A shadow looms"))
    );
    assert!(
        matrix
            .cells
            .iter()
            .any(|c| c.text.contains("Blackbeard is still"))
    );
    // The player knows the name but never talked to him: both presence texts show.
    let odd = matrix
        .cells
        .iter()
        .find(|c| c.values == ["true", "true", "0"])
        .unwrap();
    assert_eq!(odd.warnings, vec!["2 presence texts of blackbeard show"]);
    // The whole-game explorer never sees that combination: the player learns the name only by talking.
    let explored = lantern_runtime::explore(&game, lantern_runtime::Budget::default());
    let room = explored.rooms.iter().find(|r| r.id == "cantina").unwrap();
    assert!(!room.seen.contains(&odd.text), "{:#?}", room.seen);
    assert!(matrix.cells.iter().any(|c| room.seen.contains(&c.text)));
}

#[test]
fn an_edited_story_keeps_the_game_state() {
    let mut game = lighthouse();
    game.start();
    game.input("n");
    game.input("take lamp");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&root).unwrap();
    for f in &mut files {
        if f.path == "rooms/cottage.yaml" {
            f.text = f
                .text
                .replace("description:\n", "description:\n  - \"A NEW LINE.\"\n");
        }
    }
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let next = game.carry_over(world);
    assert_eq!(next.state().room.as_str(), "cottage");
    assert!(next.state().is_carried("oil_lamp"));
    assert!(next.describe_preview().plain_text().contains("A NEW LINE."));
}

#[test]
fn the_writer_gets_symbols_checks_and_other_values() {
    use lantern_runtime::text_matrix;
    let mut game = lighthouse();
    game.start();
    let symbols = game.symbols();
    let lit = symbols.iter().find(|s| s.name == "oil_lamp.lit").unwrap();
    assert_eq!(lit.value.as_deref(), Some("false"));
    let phase = symbols.iter().find(|s| s.name == "time.phase").unwrap();
    assert_eq!(phase.value.as_deref(), Some("dusk"));
    assert!(phase.values.contains(&"night".to_string()));
    let check = game.try_markup("A {oil_lamp.lot ? x : y}");
    assert!(check.error.unwrap().message.contains("did you mean"));
    let m = text_matrix(
        &game,
        "The sun {night: sets | rises}.",
        Some("not beacon_lit"),
    )
    .unwrap();
    let texts: std::collections::BTreeSet<&str> = m.cells.iter().map(|c| c.text.as_str()).collect();
    assert!(
        texts.contains("The sun sets.") && texts.contains("The sun rises.") && texts.contains(""),
        "{texts:?}"
    );
}

#[test]
fn the_spiral_tower_has_levels_and_an_ending() {
    use lantern_runtime::{Budget, explore};
    let mut game = example("tower");
    game.start();
    let map = game.map();
    let level = |id: &str| map.rooms.iter().find(|r| r.id == id).unwrap().level;
    assert_eq!(
        (level("north_1"), level("west_2"), level("bell_chamber")),
        (1, 2, 4)
    );
    game.input("in");
    let text = game.input("sw").plain_text();
    assert!(text.contains("Steps curl up into the dark."), "{text}");
    game.input("up");
    assert!(
        game.describe_preview().plain_text().contains("Pigeons"),
        "level 2 text"
    );
    let result = explore(&game, Budget::default());
    assert_eq!(result.endings.len(), 1, "{:?}", result.endings);
    assert_eq!(result.stuck, 0);
}

#[test]
fn levels_come_from_up_and_down_exits_when_rooms_do_not_say() {
    let game = lighthouse();
    let map = game.map();
    let level = |id: &str| map.rooms.iter().find(|r| r.id == id).unwrap().level;
    assert_eq!(
        (level("shore"), level("stairwell"), level("lantern_room")),
        (0, 0, 1)
    );
}
