//! The two-robot demo (P6-06, #100) in `examples/two-robots`: its walkthroughs,
//! the perceiver of a text, orders, and the ending checks. Read
//! `examples/two-robots/README.md`.

use loremaster_model::Location;
use loremaster_project::{Diagnostic, Project, Severity, SourceFile, check, load, read_dir};
use loremaster_runtime::{Command, Input, Session, World, authoring, run_walkthrough};
use std::path::Path;
use std::sync::Arc;

fn demo() -> (Project, Arc<World>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/two-robots");
    let project = load(read_dir(&dir).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    (project, world)
}

fn session(world: Arc<World>) -> Session {
    let mut session = Session::new(world);
    session.start();
    session
}

fn say(session: &mut Session, text: &str) -> String {
    session.submit(Input::Text(text.into())).plain_text()
}

#[test]
fn walkthroughs_pass() {
    let (project, world) = demo();
    let mut names = Vec::new();
    for (path, walk) in &project.walkthroughs {
        let result = run_walkthrough(world.clone(), walk, path);
        assert!(
            result.passed(),
            "{path}: {:#?}\n\n{}",
            result.failures,
            result.transcript
        );
        names.push(path.as_str());
    }
    names.sort();
    assert_eq!(
        names,
        [
            "tests/best.yaml",
            "tests/orders.yaml",
            "tests/wrong_valve.yaml"
        ]
    );
}

#[test]
fn the_text_differs_by_the_actor_who_perceives_it() {
    let (_, world) = demo();
    let mut s = session(world);
    let wick = say(&mut s, "look");
    assert!(wick.contains("shakes in the red"), "{wick}");
    assert!(!wick.contains("drums on the far side"), "{wick}");
    // Bilge stands in the same room, but hears it instead.
    let bilge = say(&mut s, "switch to bilge");
    assert!(bilge.contains("drums on the far side"), "{bilge}");
    assert!(!bilge.contains("shakes in the red"), "{bilge}");
}

#[test]
fn each_robot_keeps_its_own_things_and_room() {
    let (_, world) = demo();
    let mut s = session(world);
    say(&mut s, "take spanner");
    assert!(say(&mut s, "inventory").contains("spanner"));
    say(&mut s, "switch to bilge");
    // Bilge does not have the spanner, and neither does the room show it.
    assert!(!say(&mut s, "inventory").contains("spanner"));
    let look = say(&mut s, "look");
    assert!(look.contains("Wick waits here"), "{look}");
    assert!(!look.contains("spanner"), "{look}");
    assert!(!look.contains("Bilge waits here"), "{look}");
    let state = s.state();
    assert_eq!(
        state.location_of("spanner"),
        Some(&Location::In("wick".into()))
    );
    say(&mut s, "switch to wick");
    assert!(say(&mut s, "inventory").contains("spanner"));
}

#[test]
fn an_order_runs_one_step_per_turn_and_reports_at_the_end() {
    let (_, world) = demo();
    let mut s = session(world);
    say(&mut s, "hold lever");
    let turn = say(&mut s, "bilge, take spanner then s then s");
    assert!(turn.contains("Bilge takes the order"), "{turn}");
    assert!(!turn.contains("reports"), "{turn}");
    assert_eq!(s.state().room.as_str(), "gallery");
    // The player steered Wick: the order moved Bilge, not the player.
    let turn = say(&mut s, "wait");
    assert!(turn.contains("Pump Hall"), "{turn}");
    assert_eq!(s.state().room.as_str(), "gallery");
    assert_eq!(
        s.state().location_of("bilge"),
        Some(&Location::Room("hall".into()))
    );
    // The last step reports, and the queue is empty.
    let turn = say(&mut s, "wait");
    assert!(turn.contains("Bilge reports: done"), "{turn}");
    assert!(s.state().orders.is_empty());
    // A step the robot cannot run ends the order and says so.
    let turn = say(&mut s, "bilge, frobnicate the tide");
    assert!(
        turn.contains("Bilge reports: the order stops here"),
        "{turn}"
    );
}

#[test]
fn a_command_record_with_an_actor_is_an_order() {
    let (_, world) = demo();
    let mut s = session(world);
    let turn = s.submit(Input::Command(Command::Verb {
        verb: "take".into(),
        objects: vec!["spanner".into()],
        actor: Some("bilge".into()),
        preposition: None,
        topic: None,
    }));
    assert!(
        turn.plain_text().contains("Bilge takes the order"),
        "{}",
        turn.plain_text()
    );
    assert_eq!(
        s.state().location_of("spanner"),
        Some(&Location::In("bilge".into()))
    );
}

#[test]
fn an_order_is_refused_for_things_that_take_no_orders() {
    let (_, world) = demo();
    let mut s = session(world);
    let turn = s.submit(Input::Command(Command::Verb {
        verb: "hold".into(),
        objects: vec!["lever".into()],
        actor: Some("spanner".into()),
        preposition: None,
        topic: None,
    }));
    assert!(
        turn.plain_text().contains("acts on its own"),
        "{}",
        turn.plain_text()
    );
}

#[test]
fn undo_and_saves_keep_the_active_actor_and_the_orders() {
    let (_, world) = demo();
    let mut s = session(world.clone());
    say(&mut s, "switch to bilge");
    assert_eq!(s.state().active.as_ref().unwrap().as_str(), "bilge");
    say(&mut s, "undo");
    assert_eq!(s.state().active.as_ref().unwrap().as_str(), "wick");

    say(&mut s, "bilge, s then s");
    let (restored, _) = Session::from_save(world, &s.save()).unwrap();
    assert_eq!(restored.state().orders, s.state().orders);
    assert_eq!(restored.state().active, s.state().active);
}

#[test]
fn explorer_terminates_and_every_ending_is_reachable() {
    let (project, world) = demo();
    let explored = authoring::explore(world, authoring::Budget::default());
    assert!(!explored.truncated, "{} states", explored.states);
    let checks = &explored.ending_checks;
    assert!(checks.ran && checks.proven, "{checks:#?}");
    assert!(checks.dead_ends.is_empty(), "{:#?}", checks.dead_ends);
    assert!(checks.stale.is_empty(), "{:#?}", checks.stale);
    let diagnostics: Vec<_> = checks
        .diagnostics()
        .into_iter()
        .map(|d| project.locate(d))
        .collect();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for ending in &project.story.endings {
        assert!(
            explored
                .endings
                .iter()
                .any(|e| e.text.starts_with(&ending.title)),
            "no explored path ends in {}",
            ending.id
        );
    }
}

fn errors(files: Vec<SourceFile>) -> Vec<Diagnostic> {
    check(&load(files).unwrap())
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

#[test]
fn the_checks_accept_the_demo_and_refuse_a_bad_playable_list() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/two-robots");
    let files = read_dir(&dir).unwrap();
    assert_eq!(errors(files.clone()), Vec::new());
    for (from, to, message) in [
        ("[wick, bilge]", "[ghost, bilge]", "unknown thing `ghost`"),
        (
            "[wick, bilge]",
            "[spanner, bilge]",
            "needs `container: true`",
        ),
    ] {
        let mut changed = files.clone();
        let story = changed.iter_mut().find(|f| f.path == "story.yaml").unwrap();
        story.text = story.text.replace(from, to);
        let found = errors(changed);
        assert!(
            found.iter().any(|d| d.message.contains(message)),
            "{message}: {found:#?}"
        );
    }
}

#[test]
fn the_checks_report_every_fault_of_a_playable_actor() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/two-robots");
    let mut files = read_dir(&dir).unwrap();
    let story = files.iter_mut().find(|f| f.path == "story.yaml").unwrap();
    story.text = story
        .text
        .replace("start: gallery", "start: hall")
        .replace("[wick, bilge]", "[spanner, bilge, bilge]");
    let found = errors(files);
    for message in [
        "put `spanner` in `hall`",
        "`spanner` needs `container: true`",
        "`spanner` is portable",
        "`bilge` is playable twice",
    ] {
        assert!(
            found.iter().any(|d| d.message.contains(message)),
            "{message}: {found:#?}"
        );
    }
}

#[test]
fn the_first_actor_carries_what_it_holds_at_the_start() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/two-robots");
    let mut files = read_dir(&dir).unwrap();
    let spanner = (files.iter_mut())
        .find(|f| f.path == "things/spanner.yaml")
        .unwrap();
    spanner.text = spanner.text.replace("{ room: gallery }", "{ in: wick }");
    let project = load(files).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut s = session(world);
    assert_eq!(s.state().location_of("spanner"), Some(&Location::Carried));
    assert!(say(&mut s, "inventory").contains("spanner"));
}

#[test]
fn an_order_text_needs_a_command_and_a_refused_order_takes_no_turn() {
    let (_, world) = demo();
    let mut s = session(world);
    // `bilge,` has no step, and `robot` names both robots.
    for text in ["bilge,", "robot, look"] {
        let turn = say(&mut s, text);
        assert!(!turn.contains("takes the order"), "{text}: {turn}");
        assert!(s.state().orders.is_empty(), "{text}");
    }
    let turns = s.state().turns;
    let turn = say(&mut s, "wick, look");
    assert!(turn.contains("is you now"), "{turn}");
    assert_eq!(s.state().turns, turns);
}

#[test]
fn an_order_keeps_the_players_way_back() {
    let (_, world) = demo();
    let mut s = session(world);
    say(&mut s, "switch to bilge");
    say(&mut s, "hold lever");
    say(&mut s, "switch to wick");
    say(&mut s, "s");
    assert_eq!(s.state().previous.as_ref().unwrap().as_str(), "gallery");
    let turn = say(&mut s, "bilge, look");
    assert!(turn.contains("Bilge reports: done"), "{turn}");
    assert_eq!(s.state().room.as_str(), "hall");
    assert_eq!(s.state().previous.as_ref().unwrap().as_str(), "gallery");
}

#[test]
fn a_save_from_before_playable_gets_an_active_actor() {
    let (_, world) = demo();
    let mut s = session(world.clone());
    say(&mut s, "take spanner");
    let mut save = serde_json::to_value(s.save()).unwrap();
    // As a save of the story before it had `playable`.
    save["story"]["hash"] = "older".into();
    let state = save["state"].as_object_mut().unwrap();
    state.remove("active");
    let facts = state["facts"].as_object_mut().unwrap();
    facts.remove("wick.active");
    facts.remove("bilge.active");
    let (restored, _) = Session::load(world, save).unwrap();
    let state = restored.state();
    assert_eq!(state.active.as_ref().unwrap().as_str(), "wick");
    assert_eq!(
        state.facts.get("wick.active"),
        Some(&loremaster_model::Value::Bool(true))
    );
    assert_eq!(
        state.location_of("wick"),
        Some(&Location::Room(state.room.clone()))
    );
    assert_eq!(state.location_of("spanner"), Some(&Location::Carried));
}

#[test]
fn a_dropped_actor_fact_stays_dropped() {
    let (_, world) = demo();
    let s = session(world.clone());
    let mut save = serde_json::to_value(s.save()).unwrap();
    // As a save of a story whose active actor is gone.
    save["story"]["hash"] = "older".into();
    let state = save["state"].as_object_mut().unwrap();
    state.insert("active".into(), "ghost".into());
    state["facts"]
        .as_object_mut()
        .unwrap()
        .insert("ghost.active".into(), true.into());
    let (restored, loaded) = Session::load(world, save).unwrap();
    assert!(!restored.state().facts.contains_key("ghost.active"));
    assert!(
        loaded.dropped.iter().any(|d| d == "fact:ghost.active"),
        "{:?}",
        loaded.dropped
    );
    assert_eq!(restored.state().active.as_ref().unwrap().as_str(), "wick");
}

#[test]
fn an_order_step_is_the_players_action() {
    // Bilge acts for the player (WO-5): the rules for the player fire, with
    // `bilge.active`, and the action counts as the player's.
    let (_, world) = demo();
    let mut s = session(world);
    say(&mut s, "bilge, hold lever");
    assert_eq!(
        s.state().facts.get("bilge_holds"),
        Some(&loremaster_model::Value::Bool(true))
    );
    assert_eq!(s.state().action_counts.get("hold"), Some(&1));
    // A rule that names Bilge as its actor never fires: the check says how
    // to write it.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/two-robots");
    let mut files = read_dir(&dir).unwrap();
    files.push(SourceFile {
        path: "rules/probe.yaml".into(),
        text: "- id: probe\n  on: { verb: hold, actor: bilge }\n  then: [stop]\n".into(),
    });
    let found = errors(files);
    assert!(
        found.iter().any(|d| d.message.contains("`bilge.active`")),
        "{found:#?}"
    );
}
