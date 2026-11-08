//! The Salt Tower (P5-06, #93) in `examples/salt-tower`: its walkthroughs and
//! the reachability of its sections.

use loremaster_model::Value;
use loremaster_project::{Project, check, load, read_dir};
use loremaster_runtime::{Input, Session, Status, World, authoring, run_walkthrough};
use std::path::Path;
use std::sync::Arc;

fn tower() -> (Project, Arc<World>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/salt-tower");
    let project = load(read_dir(&dir).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    (project, world)
}

#[test]
fn the_walkthroughs_pass() {
    let (project, world) = tower();
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
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
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "tests/fight.yaml",
            "tests/grate.yaml",
            "tests/pay.yaml",
            "tests/spent.yaml"
        ]
    );
}

#[test]
fn all_sections_are_reachable_and_no_state_is_stuck() {
    let (project, world) = tower();
    let explored = authoring::explore(world, authoring::Budget::default());
    for n in 1..=9 {
        let id = format!("section_{n}");
        assert!(
            explored.rooms.iter().any(|r| r.id == id && r.reached),
            "{id} is not reached"
        );
    }
    let checks = &explored.ending_checks;
    assert!(checks.ran && checks.proven, "{checks:#?}");
    assert!(checks.dead_ends.is_empty(), "{:#?}", checks.dead_ends);
    let diagnostics: Vec<_> = checks.diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    // Every state goes to an ending. "Sent away" is the only one that is not
    // the win or the death at 0 STAMINA that combat needs.
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
    let mut ids: Vec<_> = project
        .story
        .endings
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, ["delivered", "fallen", "sent_away"]);
}

#[test]
fn with_host_rolls_the_exit_waits_for_the_dice() {
    let (_, world) = tower();
    let mut s = Session::new(world);
    s.set_host_rolls(true);
    s.start();
    let mut step = s.submit(Input::Text("roll your character".into()));
    for total in [4, 7, 3] {
        assert_eq!(s.state().room.as_str(), "section_1");
        let Status::Await { awaiting } = &step.status else {
            panic!("no await: {}", step.plain_text());
        };
        step = s.resume(&awaiting.id.clone(), Value::Int(total)).unwrap();
    }
    assert_eq!(s.state().room.as_str(), "section_2");
    assert_eq!(step.plain_text().matches("2. The Door").count(), 1);
}
