//! The Zork I fit test: puzzles of the original, ported by hand with timers,
//! containers, the score, machines and check rules. The tests jump to the
//! puzzle with author commands, as the editor's state switches do.

use loremaster_model::Value;
use loremaster_project::{check, load, read_dir};
use loremaster_runtime::{AuthorCommand, Game, World};
use std::path::Path;
use std::sync::Arc;

fn zork() -> Game {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/zork1");
    let project = load(read_dir(&root).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    let mut game = Game::new(Arc::new(World::compile(project.story).unwrap()));
    game.start();
    game
}

fn goto(g: &mut Game, room: &str) {
    g.author(AuthorCommand::Goto { room: room.into() });
}

fn give(g: &mut Game, things: &[&str]) {
    for t in things {
        g.author(AuthorCommand::Give {
            thing: t.to_string(),
        });
    }
}

fn say(g: &mut Game, cmd: &str) -> String {
    g.input(cmd).plain_text()
}

fn flag(g: &Game, name: &str) -> bool {
    g.state().flag(name)
}

#[test]
fn the_lamp_battery_runs_down_while_the_lamp_is_lit() {
    let mut g = zork();
    give(&mut g, &["lamp"]);
    say(&mut g, "wait");
    assert_eq!(
        g.state().timers["lamp_battery"],
        385,
        "an unlit lamp keeps its battery"
    );
    say(&mut g, "turn on lamp");
    g.author(AuthorCommand::Set {
        name: "timer.lamp_battery".into(),
        value: Value::Int(16),
    });
    let text = say(&mut g, "wait");
    assert!(text.contains("The lamp is nearly out."), "{text}");
    g.author(AuthorCommand::Set {
        name: "timer.lamp_battery".into(),
        value: Value::Int(1),
    });
    let text = say(&mut g, "wait");
    assert!(
        text.contains("more light than from the brass lantern"),
        "{text}"
    );
    assert!(!flag(&g, "lamp.lit"));
    let text = say(&mut g, "turn on lamp");
    assert!(text.contains("A burned-out lamp won't light."), "{text}");
}

#[test]
fn treasures_score_when_taken_and_in_the_trophy_case() {
    let mut g = zork();
    goto(&mut g, "gallery");
    give(&mut g, &["lamp"]);
    say(&mut g, "turn on lamp");
    say(&mut g, "take painting");
    let after_take = g.state().facts["score"].as_int().unwrap();
    assert!(after_take > 0, "taking a treasure scores");
    goto(&mut g, "living_room");
    say(&mut g, "open case");
    let text = say(&mut g, "put painting in case");
    assert!(text.contains("Done."), "{text}");
    let score = g.state().facts["score"].as_int().unwrap();
    assert!(score > after_take, "the trophy case scores");
    let text = say(&mut g, "score");
    assert!(
        text.contains(&format!("Your score is {score} (total of 350 points)")),
        "{text}"
    );
    assert!(text.contains("Beginner"), "{text}");
    let text = say(&mut g, "look");
    assert!(
        text.contains("The trophy case contains a painting."),
        "{text}"
    );
}

#[test]
fn bell_candles_and_book_drive_the_spirits_away() {
    let mut g = zork();
    goto(&mut g, "entrance_to_hades");
    give(&mut g, &["lamp", "bell", "candles", "book"]);
    say(&mut g, "turn on lamp");
    let text = say(&mut g, "s");
    assert!(text.contains("invisible force"), "{text}");
    let text = say(&mut g, "ring bell");
    assert!(text.contains("The bell suddenly becomes red hot"), "{text}");
    assert!(text.contains("the candles drop to the ground"), "{text}");
    say(&mut g, "take candles");
    let text = say(&mut g, "turn on candles");
    assert!(text.contains("The spirits cower"), "{text}");
    let text = say(&mut g, "read book");
    assert!(text.contains("Begone, fiends!"), "{text}");
    assert!(flag(&g, "lld_flag"));
    say(&mut g, "s");
    assert_eq!(g.state().room.as_str(), "land_of_living_dead");
}

#[test]
fn a_slow_exorcism_fails_and_can_start_again() {
    let mut g = zork();
    goto(&mut g, "entrance_to_hades");
    give(&mut g, &["lamp", "bell", "candles", "book"]);
    say(&mut g, "turn on lamp");
    say(&mut g, "ring bell");
    let mut log = String::new();
    for _ in 0..6 {
        log.push_str(&say(&mut g, "wait"));
    }
    assert!(log.contains("resume their hideous jeering"), "{log}");
    assert_eq!(
        g.machines()
            .iter()
            .find(|m| m.id == "exorcism")
            .unwrap()
            .state,
        "idle"
    );
    say(&mut g, "take bell");
    let text = say(&mut g, "ring bell");
    assert!(text.contains("red hot"), "{text}");
}

#[test]
fn the_dam_drains_the_reservoir() {
    let mut g = zork();
    goto(&mut g, "dam_room");
    give(&mut g, &["lamp", "wrench"]);
    say(&mut g, "turn on lamp");
    let text = say(&mut g, "turn bolt with wrench");
    assert!(text.contains("The bolt won't turn"), "{text}");
    goto(&mut g, "maintenance_room");
    say(&mut g, "push yellow button");
    goto(&mut g, "dam_room");
    let text = say(&mut g, "turn bolt with wrench");
    assert!(text.contains("The sluice gates open"), "{text}");
    assert!(flag(&g, "gates_open"));
    for _ in 0..8 {
        say(&mut g, "wait");
    }
    assert!(flag(&g, "low_tide"));
    goto(&mut g, "reservoir_north");
    say(&mut g, "s");
    assert_eq!(
        g.state().room.as_str(),
        "reservoir",
        "the reservoir can be crossed"
    );
}
