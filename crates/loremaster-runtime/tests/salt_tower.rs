//! The Salt Tower (P5-06, #93) in `examples/salt-tower`: its walkthroughs and
//! the reachability of its sections.

use loremaster_project::{Project, check, load, read_dir};
use loremaster_runtime::{World, authoring, run_walkthrough};
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
    assert_eq!(project.story.endings.len(), 3);
}
