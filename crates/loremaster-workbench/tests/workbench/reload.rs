//! `reload`: the running game carries over to a changed world (SE-14).

use crate::{Host, root, world_of};
use loremaster_model::{Location, Value};
use loremaster_project::{load, read_dir};
use loremaster_runtime::authoring::carry_over;
use loremaster_runtime::{Input, ReloadError, Session, Status, World};
use loremaster_workbench::{AuthorCommand, Workbench};
use std::sync::Arc;

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
    bench.reload(world_of(&files(true, yard))).unwrap();

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
    bench.reload(world_of(&files(false, YARD))).unwrap();

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
    bench.reload(world_of(&files(false, YARD))).unwrap();
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
    bench.reload(world_of(&changed)).unwrap();
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
    bench.reload(world_of(&changed)).unwrap();
    assert_eq!(bench.state().room.as_str(), "hall");
    assert!(!bench.state().visits.contains_key("yard"));
    assert!(bench.state().is_carried("lamp"));
}

#[test]
fn the_undo_history_does_not_carry_over() {
    let mut bench = played();
    assert!(bench.session().undo_steps() > 0);
    bench.reload(world_of(&files(true, YARD))).unwrap();
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
    bench.reload(world_of(&same)).unwrap();
    assert!(bench.state().awaiting.is_some());
    // An edit of the rule drops the wait: its effects may have moved.
    let mut bench = waiting();
    let edited = "- id: pull_lever\n  on: { verb: [pull, lever] }\n  instead: true\n  then:\n    - await: { choice: [left, right] }\n    - say: \"You go.\"\n";
    bench.reload(world_of(&lever_files(edited))).unwrap();
    assert!(bench.state().awaiting.is_none());
}

// ---- Session::reload: the host API ---------------------------------------

const RING: &str = "- id: ring\n  once: true\n  on: { verb: [pull, lever] }\n  instead: true\n  then:\n    - say: \"A bell rings.\"\n";

/// A session in the yard, with the coin carried and the once rule `ring` fired.
fn host_played() -> Session {
    let mut session = Session::new(world_of(&lever_files(RING)));
    session.start();
    for text in ["pull lever", "n", "take coin"] {
        session.submit(Input::Text(text.into()));
    }
    assert!(session.state().fired.iter().any(|r| r.as_str() == "ring"));
    assert_eq!(session.state().room.as_str(), "yard");
    session
}

#[test]
fn session_reload_lists_what_it_dropped() {
    let mut session = host_played();
    // No coin, no yard, no rule `ring`.
    let changed = vec![
        ("story.yaml", STORY),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription:\n  - \"A hall.\"\n",
        ),
        ("things/lamp.yaml", LAMP),
        ("things/lever.yaml", LEVER_THING),
        ("verbs/pull.yaml", PULL),
    ];
    let reloaded = session.reload(world_of(&changed), &[]).unwrap();
    assert_eq!(
        reloaded.dropped,
        ["fact:coin.shiny", "room:yard", "rule:ring", "thing:coin"]
    );
    // The player was in the yard: they go to the start room.
    assert_eq!(session.state().room.as_str(), "hall");
    assert!(session.state().fired.is_empty());
}

#[test]
fn session_reload_of_the_same_story_drops_nothing() {
    let mut session = host_played();
    let reloaded = session.reload(world_of(&lever_files(RING)), &[]).unwrap();
    assert!(reloaded.dropped.is_empty(), "{:?}", reloaded.dropped);
    assert!(session.state().is_carried("coin"));
}

#[test]
fn a_thing_in_what_is_no_longer_a_container_goes_back_to_its_start() {
    let with_chest = |chest: &'static str| {
        let mut files = files(true, YARD);
        files.push(("things/chest.yaml", chest));
        world_of(&files)
    };
    let box_ = "name: chest\nlocation: { room: hall }\ncontainer: true\n";
    let mut session = Session::new(with_chest(box_));
    session.start();
    session.submit(Input::Text("take lamp".into()));
    let step = session.submit(Input::Text("put lamp in chest".into()));
    assert_eq!(
        session.state().locations["lamp"],
        Location::In("chest".into()),
        "{}",
        step.plain_text()
    );
    session
        .reload(with_chest("name: chest\nlocation: { room: hall }\n"), &[])
        .unwrap();
    assert_eq!(
        session.state().locations["lamp"],
        Location::Room("hall".into())
    );
}

#[test]
fn hints_work_after_a_session_reload() {
    let project = load(read_dir(&root("lighthouse")).unwrap()).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut session = Session::new(world.clone());
    session.set_hints_from(&project.walkthroughs);
    session.start();
    session.submit(Input::Text("n".into()));
    session.hint(None);
    session
        .reload(Arc::new((*world).clone()), &project.walkthroughs)
        .unwrap();
    assert!(session.has_hints());
    assert_eq!(session.hints()[0].shown.len(), 1);
    let step = session.hint(None);
    assert!(
        step.plain_text().starts_with("Hint 2 of 3:"),
        "{}",
        step.plain_text()
    );
}

#[test]
fn the_unchecked_carry_over_validates_the_host_interface() {
    let world = |story: &'static str| {
        let mut files = files(true, YARD);
        files[0] = ("story.yaml", story);
        world_of(&files)
    };
    let changed = "title: Yard\nstart: hall\ninterface:\n  events:\n    - { id: gone, args: { boss: text } }\n";
    let before = world(CONTRACT);
    let built_for = before.story.interface.clone().unwrap();
    let session = Session::for_host(before, &built_for).unwrap();

    // The same interface: the session stays bound to the host.
    let mut same = carry_over(&session, world(CONTRACT));
    assert!(matches!(
        same.reload(world(changed), &[]),
        Err(ReloadError::InterfaceChanged(_))
    ));

    // Another interface: the editor's path unbinds it, and a later reload is
    // not refused against the stale interface.
    let mut unbound = carry_over(&session, world(changed));
    assert!(unbound.reload(world(changed), &[]).is_ok());
    assert!(unbound.reload(world(CONTRACT), &[]).is_ok());
}

const CONTRACT: &str =
    "title: Yard\nstart: hall\ninterface:\n  events:\n    - { id: killed, args: { boss: text } }\n";

#[test]
fn a_host_session_refuses_a_changed_interface() {
    let world = |story: &'static str| {
        let mut files = files(true, YARD);
        files[0] = ("story.yaml", story);
        world_of(&files)
    };
    let before = world(CONTRACT);
    let built_for = before.story.interface.clone().unwrap();
    let mut session = Session::for_host(before, &built_for).unwrap();
    session.start();
    session.submit(Input::Text("n".into()));
    let changed = "title: Yard\nstart: hall\ninterface:\n  events:\n    - { id: killed, args: { boss: text, level: int } }\n";
    let Err(ReloadError::InterfaceChanged(mismatch)) = session.reload(world(changed), &[]) else {
        panic!("the reload should be refused");
    };
    assert_eq!(mismatch.built, built_for.hash());
    // The session is unchanged: same world, same room.
    assert_eq!(session.interface(), built_for);
    assert_eq!(session.state().room.as_str(), "yard");
    // The same interface reloads.
    assert!(session.reload(world(CONTRACT), &[]).is_ok());
    // A session not made for a host never gets the error.
    let mut plain = Session::new(world(CONTRACT));
    assert!(plain.reload(world(changed), &[]).is_ok());
}
