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
