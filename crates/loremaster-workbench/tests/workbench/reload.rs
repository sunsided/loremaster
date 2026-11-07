//! `reload`: the running game carries over to a changed world (SE-14).

use crate::{Host, world_of};
use loremaster_model::Value;
use loremaster_runtime::{Input, Status};
use loremaster_workbench::{AuthorCommand, Workbench};

const STORY: &str = "title: Yard\nstart: hall\nvars:\n  - { id: rang, type: bool, initial: false }\n  - { id: mood, type: int, initial: 1 }\n";
const HALL: &str =
    "name: Hall\ndescription:\n  - \"A hall.\"\nexits:\n  - { direction: north, to: yard }\n";
const YARD: &str =
    "name: Yard\ndescription:\n  - \"A yard.\"\nexits:\n  - { direction: south, to: hall }\n";
const LAMP: &str = "name: lamp\nlocation: { room: hall }\nportable: true\nprops: { lit: false }\n";
const COIN: &str = "name: coin\nlocation: { room: yard }\nportable: true\nprops: { shiny: true }\n";

fn files(coin: bool, yard: &'static str) -> Vec<(&'static str, &'static str)> {
    let mut files = vec![
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", yard),
        ("things/lamp.yaml", LAMP),
    ];
    if coin {
        files.push(("things/coin.yaml", COIN));
    }
    files
}

/// In the yard, with the lamp and the coin carried, `rang` set and the lamp lit.
fn played() -> Workbench {
    let mut bench = Workbench::new(world_of(&files(true, YARD)));
    bench.start();
    bench.input("take lamp");
    bench.input("n");
    bench.input("take coin");
    bench.author(AuthorCommand::Set {
        name: "rang".into(),
        value: Value::Bool(true),
    });
    bench.author(AuthorCommand::Set {
        name: "lamp.lit".into(),
        value: Value::Bool(true),
    });
    assert_eq!(bench.state().room.as_str(), "yard");
    assert!(bench.state().is_carried("coin"));
    bench
}

#[test]
fn the_room_the_things_and_the_facts_carry_over() {
    let mut bench = played();
    let turns = bench.state().turns;
    let yard = "name: Yard\ndescription:\n  - \"A yard, newly swept.\"\nexits:\n  - { direction: south, to: hall }\n";
    bench.reload(world_of(&files(true, yard)));

    let state = bench.state();
    assert_eq!(state.room.as_str(), "yard");
    assert_eq!(state.turns, turns);
    assert!(state.is_carried("lamp") && state.is_carried("coin"));
    assert!(state.flag("rang"));
    assert!(state.flag("lamp.lit"));
    // The views read the new world.
    assert!(bench.describe().plain_text().contains("newly swept"));
    let yard_view = bench.atlas().into_iter().find(|r| r.id == "yard").unwrap();
    assert!(yard_view.visited && yard_view.current);
}

#[test]
fn a_removed_thing_is_dropped() {
    let mut bench = played();
    bench.reload(world_of(&files(false, YARD)));

    let state = bench.state();
    assert!(
        !state.locations.contains_key("coin"),
        "{:?}",
        state.locations
    );
    assert!(!state.facts.contains_key("coin.shiny"));
    assert!(bench.inspect_thing("coin").is_none());
    assert!(bench.vars().iter().all(|v| !v.name.starts_with("coin.")));
    // What is still there stays as it was.
    assert_eq!(state.room.as_str(), "yard");
    assert!(state.is_carried("lamp") && state.flag("rang"));
    assert!(
        bench
            .session()
            .scene()
            .inventory
            .iter()
            .all(|t| t.id != "coin")
    );
}

#[test]
fn the_session_plays_on_after_a_reload() {
    let mut bench = played();
    bench.reload(world_of(&files(false, YARD)));
    // The opening ran before the reload: it does not run again.
    assert!(bench.start().blocks.is_empty());
    let step = bench.session_mut().submit(Input::Text("s".into()));
    assert_eq!(step.status, Status::Ready);
    assert!(
        step.plain_text().contains("A hall."),
        "{}",
        step.plain_text()
    );
    assert_eq!(bench.session().state().room.as_str(), "hall");
}

#[test]
fn carried_to_leaves_the_old_workbench_as_it_was() {
    let bench = played();
    let next = bench.carried_to(world_of(&files(false, YARD)));
    assert!(bench.state().locations.contains_key("coin"));
    assert!(!next.state().locations.contains_key("coin"));
    assert_eq!(next.state().room.as_str(), "yard");
}

#[test]
fn the_views_work_through_the_workbench() {
    let mut bench = played();
    assert_eq!(bench.try_condition("rang and lamp.lit").holds, Some(true));
    assert_eq!(bench.try_markup("{rang ? Rung. | Quiet.}").text, "Rung.");
    assert!(bench.try_markup("{nothing.here}").error.is_some());
    assert!(bench.symbols().iter().any(|s| s.name == "mood"));
    assert_eq!(bench.map().rooms.len(), 2);
    assert!(bench.inspect_exit("south").is_some());
    assert!(bench.room_matrix("yard").is_some());
    assert!(!bench.assembly().is_empty());
    let turn = bench.author(AuthorCommand::Goto {
        room: "hall".into(),
    });
    assert!(turn.plain_text().contains("A hall."));
    assert_eq!(bench.session().scene().room, "hall");
    let explored = bench.explore(loremaster_workbench::Budget::default());
    assert!(explored.rooms.iter().all(|r| r.reached));
}

#[test]
fn a_value_still_at_its_start_takes_the_new_start_value() {
    let mut bench = played();
    let story = "title: Yard\nstart: hall\nvars:\n  - { id: rang, type: bool, initial: false }\n  - { id: mood, type: int, initial: 5 }\n";
    let mut changed = files(true, YARD);
    changed[0] = ("story.yaml", story);
    bench.reload(world_of(&changed));
    // `mood` was never changed: it takes the new start. `rang` was: it stays.
    assert_eq!(bench.state().facts["mood"], Value::Int(5));
    assert!(bench.state().flag("rang"));
}

#[test]
fn a_room_that_is_gone_becomes_the_start_room() {
    let mut bench = played();
    let hall = "name: Hall\ndescription:\n  - \"A hall.\"\n";
    let changed = vec![
        ("story.yaml", STORY),
        ("rooms/hall.yaml", hall),
        ("things/lamp.yaml", LAMP),
    ];
    bench.reload(world_of(&changed));
    assert_eq!(bench.state().room.as_str(), "hall");
    assert!(!bench.state().visits.contains_key("yard"));
    assert!(bench.state().is_carried("lamp"));
}

#[test]
fn the_undo_history_does_not_carry_over() {
    let mut bench = played();
    assert!(bench.session().undo_steps() > 0);
    bench.reload(world_of(&files(true, YARD)));
    assert_eq!(bench.session().undo_steps(), 0);
    let step = bench.session_mut().undo();
    assert!(
        step.plain_text().contains("nothing to undo"),
        "{}",
        step.plain_text()
    );
    assert_eq!(bench.state().room.as_str(), "yard");
}

const LEVER: &str = "- id: pull_lever\n  on: { verb: [pull, lever] }\n  instead: true\n  then:\n    - say: \"Left or right?\"\n    - await: { choice: [left, right] }\n    - say: \"You go.\"\n";
const LEVER_THING: &str = "name: lever\nlocation: { room: hall }\n";
const PULL: &str = "label: Pull\nwords: [pull]\n";

fn lever_files(rule: &'static str) -> Vec<(&'static str, &'static str)> {
    let mut files = files(true, YARD);
    files.extend([
        ("things/lever.yaml", LEVER_THING),
        ("verbs/pull.yaml", PULL),
        ("rules/lever.yaml", rule),
    ]);
    files
}

#[test]
fn a_waiting_rule_waits_on_only_while_the_rule_is_the_same() {
    let waiting = || {
        let mut bench = Workbench::new(world_of(&lever_files(LEVER)));
        bench.start();
        bench.input("pull lever");
        assert!(bench.state().awaiting.is_some());
        bench
    };
    // An edit elsewhere keeps the rule waiting.
    let mut bench = waiting();
    let yard = "name: Yard\ndescription:\n  - \"A yard, swept.\"\nexits:\n  - { direction: south, to: hall }\n";
    let mut same = lever_files(LEVER);
    same[2] = ("rooms/yard.yaml", yard);
    bench.reload(world_of(&same));
    assert!(bench.state().awaiting.is_some());
    // An edit of the rule drops the wait: its effects may have moved.
    let mut bench = waiting();
    let edited = "- id: pull_lever\n  on: { verb: [pull, lever] }\n  instead: true\n  then:\n    - await: { choice: [left, right] }\n    - say: \"You go.\"\n";
    bench.reload(world_of(&lever_files(edited)));
    assert!(bench.state().awaiting.is_none());
}
