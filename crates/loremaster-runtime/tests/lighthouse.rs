use loremaster_project::{check, load, read_dir};
use loremaster_runtime::{BlockKind, Game, World};
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
    let meet = game.input("talk to shadow");
    assert!(meet.plain_text().contains("grim-looking"));
    assert_eq!(meet.talk.as_ref().unwrap().node, "meet");
    assert_eq!(meet.choices.len(), 3);
    let names = game.input("1");
    assert_eq!(names.blocks[0].kind, BlockKind::Said);
    assert!(names.plain_text().contains("They call me Blackbeard"));
    game.input("leave");
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

/// The cantina, with the player at Blackbeard's table.
fn at_the_table() -> Game {
    let mut game = example("cantina");
    game.start();
    game.input("in");
    game
}

#[test]
fn a_talk_stays_open_until_the_player_leaves() {
    let mut game = at_the_table();
    game.input("talk to shadow");
    let turns = game.state().turns;
    let refused = game.input("look");
    assert!(
        refused
            .plain_text()
            .contains("You are talking to the grim-looking man. Choose an answer, or leave."),
        "{}",
        refused.plain_text()
    );
    assert_eq!(refused.choices.len(), 3, "the choices show again");
    assert_eq!(game.state().turns, turns, "a refused command takes no time");
    assert!(game.input("north").talk.is_some());
    game.input("who");
    assert_eq!(game.state().turns, turns + 1, "a choice takes one turn");
    assert!(game.state().flag("knows_blackbeard_name"));
    let left = game.input("bye");
    assert!(left.talk.is_none());
    assert!(left.choices.is_empty());
    assert!(game.input("look").plain_text().contains("Low beams"));
}

#[test]
fn trust_decides_what_blackbeard_tells() {
    // Straight to the ship: he does not trust a stranger.
    let mut game = at_the_table();
    game.input("talk to shadow");
    let wary = game.input("2").plain_text();
    assert!(wary.contains("Why should I tell a stranger?"), "{wary}");
    assert!(game.choices().iter().all(|c| c.text != "\"Thank you.\""));
    // He names himself, which raises trust; then he tells.
    game.input("1");
    assert_eq!(game.state().relations["blackbeard.trust"], 1);
    let told = game.input("1").plain_text();
    assert!(told.contains("South, past the breakwater"), "{told}");
    // "Thank you" moves the player out through a world effect.
    let thanks = game
        .choices()
        .iter()
        .find(|c| c.text == "\"Thank you.\"")
        .unwrap()
        .index;
    let out = game.act(loremaster_runtime::Command::Choose { index: thanks });
    assert!(
        out.plain_text().contains("Harbour Street"),
        "{}",
        out.plain_text()
    );
    assert_eq!(game.state().room.as_str(), "street");
    assert!(
        out.talk.is_none(),
        "the talk ends when the player is taken away"
    );
    assert_eq!(game.state().relations["blackbeard.trust"], 2);
}

#[test]
fn blackbeard_remembers_the_question() {
    let mut game = at_the_table();
    game.input("talk to shadow");
    game.input("2");
    assert!(
        game.state().memories["blackbeard"].contains("asked_marigold"),
        "{:?}",
        game.state().memories
    );
    game.input("leave");
    let again = game.input("talk to man");
    assert_eq!(again.talk.as_ref().unwrap().node, "again");
    assert!(
        again.plain_text().contains("Still after that ship?"),
        "{}",
        again.plain_text()
    );
}

#[test]
fn a_saved_talk_goes_on_after_a_restore() {
    let mut game = at_the_table();
    game.input("talk to shadow");
    game.input("1");
    let save = serde_json::to_string(game.state()).unwrap();
    let state: loremaster_runtime::State = serde_json::from_str(&save).unwrap();
    let world = game.world().clone();
    let mut restored = Game::restore(std::sync::Arc::new(world), state);
    assert_eq!(restored.talk().unwrap().node, "names");
    assert_eq!(restored.choices(), game.choices());
    let told = restored.input("1").plain_text();
    assert!(
        told.contains("South, past the breakwater"),
        "trust survives: {told}"
    );
}

#[test]
fn back_returns_to_the_node_before() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let mut files = read_dir(&root).unwrap();
    for f in &mut files {
        if f.path == "dialogues/blackbeard.yaml" {
            f.text = f.text.replace(
                "      - say: '\"Never mind.\"'\n        go: end",
                "      - say: '\"Never mind.\"'\n        go: back",
            );
        }
    }
    let world = std::sync::Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut game = Game::new(world);
    game.start();
    game.input("in");
    game.input("talk to shadow");
    game.input("2");
    let back = game.input("never");
    assert_eq!(back.talk.as_ref().unwrap().node, "meet");
    assert!(back.plain_text().contains("Sit, if you like."));
}

#[test]
fn an_exit_needs_a_room_or_a_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&root).unwrap();
    let shore = files
        .iter_mut()
        .find(|f| f.path == "rooms/shore.yaml")
        .unwrap();
    shore.text.push_str(
        "  - { direction: down }\n  - { direction: up, blocked: \"No.\", door: cottage_door }\n",
    );
    let problems = check(&load(files).unwrap());
    let messages: Vec<(&str, &str)> = problems
        .iter()
        .map(|d| (d.at.as_str(), d.message.as_str()))
        .collect();
    assert_eq!(problems.len(), 2, "{messages:#?}");
    assert!(messages[0].0.ends_with("exits[3]") && messages[0].1.contains("needs `to`"));
    assert!(messages[1].0.ends_with("exits[4].door"));
}

#[test]
fn back_goes_the_way_the_player_came() {
    let mut game = lighthouse();
    game.start();
    let turn = game.input("back");
    assert!(
        turn.plain_text().contains("can't go back"),
        "{}",
        turn.plain_text()
    );
    game.input("n");
    assert_eq!(game.scene().room, "cottage");
    game.input("go back");
    assert_eq!(game.scene().room, "shore");
    game.input("n");
    game.input("return");
    assert_eq!(game.scene().room, "shore");
}
