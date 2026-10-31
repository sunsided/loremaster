//! The walkthroughs in the `tests/` folders of the examples pass, and a
//! failing walkthrough says what failed.

use loremaster_project::{load, read_dir};
use loremaster_runtime::{World, run_walkthrough};
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
            assert!(
                result.passed(),
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
