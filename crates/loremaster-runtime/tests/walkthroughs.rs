//! The walkthroughs in the `tests/` folders of the examples pass, and a
//! failing walkthrough says what failed. A walkthrough marked `pending:` must
//! fail; when it passes, the mark must go.

use loremaster_project::{load, read_dir};
use loremaster_runtime::{Verdict, World, run_walkthrough};
use std::path::Path;
use std::sync::Arc;

fn examples() -> Vec<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("story.yaml").exists())
        .collect();
    dirs.sort();
    dirs
}

#[test]
fn every_example_walkthrough_passes() {
    let mut ran = 0;
    for dir in examples() {
        let project = load(read_dir(&dir).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
        let world = Arc::new(World::compile(project.story).unwrap());
        for (path, walk) in &project.walkthroughs {
            let result = run_walkthrough(world.clone(), walk, path);
            assert_ne!(
                result.verdict(),
                Verdict::UnexpectedPass,
                "{}/{path} passes: remove its `pending:` mark",
                dir.display()
            );
            assert!(
                result.ok(),
                "{}/{path}: {:#?}\n\n{}",
                dir.display(),
                result.failures,
                result.transcript
            );
            ran += 1;
        }
    }
    assert!(ran > 0, "no walkthroughs found");
}

#[test]
fn a_failing_walkthrough_names_the_step_and_the_fact() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&dir).unwrap();
    files.retain(|f| !f.path.starts_with("tests/"));
    files.push(loremaster_project::SourceFile {
        path: "tests/wrong.yaml".into(),
        text: "steps:\n  - { do: n, see: A dragon }\nexpect:\n  facts: { beacon_lit: true }\n  room: shore\n".into(),
    });
    let project = load(files).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let (path, walk) = &project.walkthroughs[0];
    let result = run_walkthrough(world, walk, path);
    assert_eq!(result.failures.len(), 3, "{:#?}", result.failures);
    assert!(result.failures[0].starts_with("step 1 (`n`): does not show \"A dragon\""));
    assert!(result.failures[1].contains("`beacon_lit` is false, not true"));
    assert!(result.failures[2].contains("not `shore`"));
}

/// The lighthouse without its own walkthroughs, plus one walkthrough file.
fn lighthouse_with(test: &str) -> (Arc<World>, loremaster_model::Walkthrough) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&dir).unwrap();
    files.retain(|f| !f.path.starts_with("tests/"));
    files.push(loremaster_project::SourceFile {
        path: "tests/pending.yaml".into(),
        text: test.into(),
    });
    let project = load(files).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    (world, project.walkthroughs[0].1.clone())
}

#[test]
fn a_pending_walkthrough_that_fails_is_pending() {
    let (world, walk) = lighthouse_with(
        "pending: { phase: 2, needs: SE-4 }\nsteps: [n]\nexpect: { room: shore }\n",
    );
    let mark = walk.pending.as_ref().unwrap();
    assert_eq!((mark.phase, mark.needs.as_str()), (2, "SE-4"));
    assert_eq!(mark.to_string(), "pending: phase 2, needs SE-4");
    let result = run_walkthrough(world, &walk, "pending");
    assert_eq!(result.verdict(), Verdict::Pending);
    assert!(result.ok());
}

#[test]
fn a_pending_walkthrough_that_passes_is_an_error() {
    let (world, walk) = lighthouse_with(
        "pending: { phase: 2, needs: SE-4 }\nsteps: [n]\nexpect: { room: cottage }\n",
    );
    let result = run_walkthrough(world, &walk, "pending");
    assert_eq!(result.verdict(), Verdict::UnexpectedPass);
    assert!(!result.ok());
}

#[test]
fn a_pending_mark_needs_a_phase_and_a_requirement() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&dir).unwrap();
    files.retain(|f| !f.path.starts_with("tests/"));
    files.push(loremaster_project::SourceFile {
        path: "tests/pending.yaml".into(),
        text: "pending: { phase: 2 }\nsteps: [n]\n".into(),
    });
    assert!(load(files).is_err());
}

#[test]
fn a_step_at_another_room_fails() {
    let (world, walk) = lighthouse_with("steps:\n  - { do: open drawer, at: cottage }\n");
    let result = run_walkthrough(world, &walk, "t");
    assert_eq!(
        result.failures,
        ["step 1 (`open drawer`): the player is in `shore`, not at `cottage`"]
    );
}
