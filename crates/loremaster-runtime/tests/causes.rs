//! What graphical hosts need from a step (#178): why a block appeared, the
//! name of a thing without a stage that holds, the verb `go` as a command
//! record, and a thing outside the player's room.

use loremaster_model::{Id, Location};
use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Block, Cause, Command, Effect, Input, Session, Step, World};
use std::sync::Arc;

fn kitchen() -> Arc<World> {
    let files = [
        ("story.yaml", "title: Kitchen\nstart: kitchen\n"),
        (
            "rooms/kitchen.yaml",
            "name: Kitchen\ndescription: [\"A kitchen.\"]\nexits:\n  - { direction: east, to: yard }\n",
        ),
        (
            "rooms/yard.yaml",
            "name: Yard\ndescription: [\"A yard.\"]\nexits:\n  - { direction: west, to: kitchen }\n",
        ),
        (
            "things/kettle.yaml",
            "name: empty kettle\nlocation: { room: kitchen }\nprops: { full: false }\nnames:\n  - { when: kettle.full, text: filled kettle }\n",
        ),
        ("things/bucket.yaml", "name: bucket\nlocation: { room: yard }\n"),
        (
            "rules/clock.yaml",
            "- id: clock\n  on: turn\n  then:\n    - say: \"The clock ticks.\"\n- id: shout\n  on: { verb: [examine, kettle] }\n  then:\n    - say: \"You hum.\"\n- id: step\n  on: { verb: [go, kettle] }\n  then:\n    - say: \"You step to the kettle.\"\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    Arc::new(World::compile(project.story).unwrap())
}

fn blocks(step: &Step) -> Vec<&Block> {
    (step.effects.iter())
        .filter_map(|e| match e {
            Effect::Text { block } => Some(block),
            _ => None,
        })
        .collect()
}

fn cause_of(step: &Step, text: &str) -> Cause {
    let block = blocks(step)
        .into_iter()
        .find(|b| b.plain_text().contains(text))
        .unwrap_or_else(|| panic!("no block with {text:?}: {}", step.plain_text()));
    block.cause
}

#[test]
fn a_block_says_whether_the_command_or_the_world_made_it() {
    let mut s = Session::new(kitchen());
    let opening = s.start();
    assert!(!blocks(&opening).is_empty());
    assert!(blocks(&opening).iter().all(|b| b.cause == Cause::World));

    let step = s.submit(Input::Text("examine kettle".into()));
    assert_eq!(cause_of(&step, "examine kettle"), Cause::Command);
    assert_eq!(cause_of(&step, "You hum."), Cause::Command);
    assert_eq!(cause_of(&step, "The clock ticks."), Cause::World);

    let tick = s.submit(Input::Tick);
    assert_eq!(cause_of(&tick, "The clock ticks."), Cause::World);

    // The JSON of a block names its cause.
    let clock = (blocks(&tick).into_iter())
        .find(|b| b.plain_text().contains("The clock ticks."))
        .unwrap();
    let json = serde_json::to_value(clock).unwrap();
    assert_eq!(json["cause"], "world");
}

#[test]
fn a_thing_whose_name_stages_do_not_hold_has_its_name() {
    let mut s = Session::new(kitchen());
    s.start();
    let step = s.submit(Input::Text("examine kettle".into()));
    assert!(
        !step.plain_text().contains("filled"),
        "{}",
        step.plain_text()
    );
    assert_eq!(s.thing("kettle").unwrap().name, "empty kettle");
}

#[test]
fn the_verb_go_with_a_direction_moves_the_player() {
    let mut s = Session::new(kitchen());
    s.start();
    let step = s.submit(Input::Command(Command::verb("go", vec![Id::new("east")])));
    assert_eq!(s.scene().room, "yard", "{}", step.plain_text());
    // The echo is the same as for `Command::Go`.
    assert!(
        step.plain_text().starts_with("go east"),
        "{}",
        step.plain_text()
    );
}

#[test]
fn the_verb_go_with_a_thing_stays_a_verb_and_runs_its_rules() {
    let mut s = Session::new(kitchen());
    s.start();
    let step = s.submit(Input::Command(Command::verb("go", vec![Id::new("kettle")])));
    assert!(
        step.plain_text().contains("You step to the kettle."),
        "{}",
        step.plain_text()
    );
    assert_eq!(s.scene().room, "kitchen");
}

#[test]
fn a_host_reads_a_thing_in_another_room() {
    let mut s = Session::new(kitchen());
    s.start();
    let bucket = s.thing("bucket").unwrap();
    assert_eq!(bucket.name, "bucket");
    assert_eq!(bucket.location, Location::Room(Id::new("yard")));
    assert!(s.thing("nothing").is_none());
}
