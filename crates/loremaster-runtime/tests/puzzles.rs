//! Puzzle logic: action phases, rule scope and priority, timers, chance and events.

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::{Game, World};
use std::sync::Arc;

const STORY: &str = r#"
title: Puzzles
start: hall
seed: 3
vars:
  - { id: rang, type: int, initial: 0 }
  - { id: alarm, type: bool, initial: false }
timers:
  - id: lamp_battery
    turns: 4
    running: true
    runs_while: lamp.lit
    at:
      2: [{ say: "The lamp appears a bit dimmer." }]
      0: [{ set: { lamp.lit: false, lamp.dead: true } }, { say: "The lamp has died." }]
  - id: fuse
    turns: 3
"#;

const HALL: &str = r#"
name: Hall
description: ["A bare hall."]
exits:
  - { direction: north, to: yard }
rules:
  - id: hall_bell
    on: { verb: [push, button] }
    then:
      - say: "Somewhere in the hall, a bell rings."
"#;

const YARD: &str = r#"
name: Yard
description: ["A yard."]
exits:
  - { direction: south, to: hall }
"#;

const LAMP: &str = r#"
name: lamp
location: { room: hall }
portable: true
props: { lit: false, dead: false }
rules:
  - id: dead_lamp
    on: { check: [turn_on] }
    when: lamp.dead
    then:
      - say: "A burned-out lamp won't light."
      - stop
  - id: light_lamp
    on: { verb: [turn_on] }
    instead: true
    then:
      - set: { lamp.lit: true }
      - say: "The lamp is now on."
"#;

const BUTTON: &str = r#"
name: button
location: { room: hall }
rules:
  - id: button_low
    on: { verb: [push, button] }
    then:
      - say: "Low."
  - id: button_high
    on: { verb: [push, button] }
    priority: 5
    then:
      - say: "High."
      - add: { rang: 1 }
"#;

const RULES: &str = r#"
- id: story_bell
  on: { verb: [push, button] }
  then:
    - say: "Story rule."
- id: no_leaving
  on: { check: [go, north] }
  when: not alarm
  then:
    - say: "Not yet."
    - stop
- id: light_fuse
  on: { verb: [pull, button] }
  then:
    - start: fuse
    - say: "The fuse hisses."
- id: boom
  on: { timer: fuse }
  then:
    - emit: blast
- id: after_blast
  on: { event: blast }
  then:
    - set: { alarm: true }
    - say: "BOOM."
"#;

fn game() -> Game {
    let files = [
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", YARD),
        ("things/lamp.yaml", LAMP),
        ("things/button.yaml", BUTTON),
        ("rules/puzzles.yaml", RULES),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    let mut game = Game::new(Arc::new(World::compile(project.story).unwrap()));
    game.start();
    game
}

fn say(game: &mut Game, cmd: &str) -> String {
    game.input(cmd).plain_text()
}

#[test]
fn thing_rules_run_first_then_room_then_story_and_priority_orders_one_scope() {
    let mut g = game();
    let text = say(&mut g, "push button");
    let order = ["High.", "Low.", "Somewhere in the hall", "Story rule."].map(|s| {
        text.find(s)
            .unwrap_or_else(|| panic!("{s} missing in {text}"))
    });
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");
}

#[test]
fn a_room_rule_fires_only_in_its_room() {
    let mut g = game();
    say(&mut g, "pull button");
    for _ in 0..3 {
        say(&mut g, "wait");
    }
    say(&mut g, "n");
    let text = say(&mut g, "s");
    assert!(text.contains("A bare hall."), "{text}");
    assert!(!text.contains("bell rings"));
}

#[test]
fn a_check_rule_with_stop_refuses_a_move() {
    let mut g = game();
    let text = say(&mut g, "n");
    assert!(text.contains("Not yet."), "{text}");
    assert_eq!(g.state().room.as_str(), "hall");
}

#[test]
fn a_timer_runs_out_and_its_event_chain_fires() {
    let mut g = game();
    let text = say(&mut g, "pull button");
    assert!(text.contains("The fuse hisses."));
    assert_eq!(g.state().timers.get("fuse"), Some(&2));
    let mut log = String::new();
    for _ in 0..2 {
        log.push_str(&say(&mut g, "wait"));
    }
    assert!(log.contains("BOOM."), "{log}");
    assert!(g.state().flag("alarm"));
    assert!(!g.state().timers.contains_key("fuse"));
    // The check rule no longer holds.
    let text = say(&mut g, "n");
    assert_eq!(g.state().room.as_str(), "yard", "{text}");
}

#[test]
fn a_timer_counts_only_while_its_condition_holds() {
    let mut g = game();
    say(&mut g, "take lamp");
    say(&mut g, "wait");
    assert_eq!(g.state().timers.get("lamp_battery"), Some(&4));
    let mut log = say(&mut g, "turn on lamp");
    assert!(log.contains("The lamp is now on."), "{log}");
    for _ in 0..3 {
        log.push_str(&say(&mut g, "wait"));
    }
    assert!(log.contains("The lamp appears a bit dimmer."), "{log}");
    assert!(log.contains("The lamp has died."), "{log}");
    let text = say(&mut g, "turn on lamp");
    assert!(text.contains("A burned-out lamp won't light."), "{text}");
    assert!(!text.contains("The lamp is now on."), "{text}");
}

#[test]
fn chance_is_the_same_in_the_same_state() {
    let story = r#"
title: Dice
start: hall
vars:
  - { id: heads, type: int, initial: 0 }
"#;
    let rules = r#"
- id: coin
  on: { turn }
  when: chance(50)
  then:
    - add: { heads: 1 }
"#;
    let files = vec![
        SourceFile {
            path: "story.yaml".into(),
            text: story.into(),
        },
        SourceFile {
            path: "rooms/hall.yaml".into(),
            text: HALL
                .replace("  - { direction: north, to: yard }\n", "")
                .replace("exits:\n", "exits: []\n"),
        },
        SourceFile {
            path: "rules/coin.yaml".into(),
            text: rules.into(),
        },
    ];
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let world = Arc::new(World::compile(project.story).unwrap());
    let run = || {
        let mut g = Game::new(world.clone());
        g.start();
        for _ in 0..100 {
            g.input("wait");
        }
        g.state().facts["heads"].as_int().unwrap()
    };
    let heads = run();
    assert_eq!(heads, run());
    assert!((25..=75).contains(&heads), "{heads} of 100");
}

#[test]
fn world_rules_hear_when_a_dialogue_reaches_a_node() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let mut files = loremaster_project::read_dir(&root).unwrap();
    files.push(SourceFile {
        path: "rules/heard.yaml".into(),
        text: r#"
- id: heard_the_name
  on: { talk: [blackbeard, names] }
  once: true
  then:
    - say: "The barkeep looks up at the name."
"#
        .into(),
    });
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    let mut g = Game::new(Arc::new(World::compile(project.story).unwrap()));
    g.start();
    say(&mut g, "in");
    say(&mut g, "talk to shadow");
    let text = say(&mut g, "1");
    assert!(text.contains("They call me Blackbeard"), "{text}");
    assert!(text.contains("The barkeep looks up at the name."), "{text}");
}
