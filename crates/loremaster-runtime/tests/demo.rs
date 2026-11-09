//! The Lock-Keeper demo (P4-07, #84) in `examples/lock-keeper-slice/demo`:
//! the three walkthroughs of the design, their transcripts, the ending
//! checks over the whole night, and the hints and the chart that the
//! walkthroughs give. Read `examples/lock-keeper-slice/README.md`.

use loremaster_model::step_chart;
use loremaster_project::{Project, load, read_dir};
use loremaster_runtime::{HintBook, Input, Session, World, authoring, run_walkthrough};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn slice(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/lock-keeper-slice")
        .join(name)
}

fn demo() -> (Project, Arc<World>) {
    let project = load(read_dir(&slice("demo")).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    (project, world)
}

/// The transcript equals `expected/<name>.transcript`. With `BLESS=1` in
/// the environment, the test writes the file instead.
fn assert_transcript(name: &str, transcript: &str) {
    let path = slice("expected").join(format!("{name}.transcript"));
    if std::env::var("BLESS").is_ok_and(|v| v == "1") {
        std::fs::write(&path, transcript).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}; run the test with BLESS=1 to write it",
            path.display()
        )
    });
    assert!(
        expected == transcript,
        "the transcript of {name} changed; if that is right, run the test with BLESS=1 to write {}\n\n{transcript}",
        path.display()
    );
}

#[test]
fn demo_walkthroughs_pass() {
    let (project, world) = demo();
    let mut names = Vec::new();
    for (path, walk) in &project.walkthroughs {
        let name = path.trim_start_matches("tests/").trim_end_matches(".yaml");
        let result = run_walkthrough(world.clone(), walk, name);
        assert!(
            result.passed(),
            "{path}: {:#?}\n\n{}",
            result.failures,
            result.transcript
        );
        assert_transcript(&format!("demo_{name}"), &result.transcript);
        names.push(name);
    }
    assert_eq!(names, ["ashes", "best", "wrong_man"]);
}

/// Play commands from the start; the text of all of them.
fn play(world: Arc<World>, commands: &[&str]) -> String {
    let mut session = Session::new(world);
    session.start();
    let mut text = String::new();
    for c in commands {
        text.push_str(&session.submit(Input::Text((*c).into())).plain_text());
    }
    text
}

#[test]
fn the_pages_count_only_while_the_player_holds_them() {
    // The best path to the pages and the weapon; the coroner comes at three.
    // With the pages in hand, the case is whole. Dropped in the chamber,
    // they are no proof, and the case is weak, as with the ash.
    // The route follows `examples/lock-keeper-slice/demo/tests/best.yaml`
    // through `search chest`. The three waits after it are this test's own:
    // they land on the coroner's step at 03:00 with the clock of
    // `examples/lock-keeper-slice/demo/story.yaml` (15-minute turns from
    // 22:00), so don't add best.yaml's accusation here.
    let (_, world) = demo();
    let to_pages = [
        "w",
        "examine boots",
        "e",
        "n",
        "e",
        "talk to crane",
        "1",
        "w",
        "open lower paddles",
        "wait",
        "wait",
        "down",
        "take pages",
    ];
    let rest = ["up", "s", "s", "search chest", "wait", "wait", "wait"];
    let held = play(world.clone(), &[&to_pages[..], &rest].concat());
    assert!(held.contains("Committed for trial"), "{held}");
    let dropped = play(world, &[&to_pages[..], &["drop pages"], &rest].concat());
    assert!(dropped.contains("Charged, case weak"), "{dropped}");
}

#[test]
fn demo_endings_are_reachable() {
    // The ending checks walk every state of the night. The demo has about
    // 39,000 states, more than the default budget of 2000; the default
    // depth (40 commands) is enough, since the coroner ends the night
    // after 20 turns.
    let (project, world) = demo();
    let budget = authoring::Budget {
        max_states: 70_000,
        ..Default::default()
    };
    let explored = authoring::explore(world, budget);
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

    // Every point of no return is accepted by a `closes` with a reason, and
    // the steps of Marsh (the pages) and Bert (the witness) are named.
    assert!(checks.no_return.iter().all(|p| p.accepted()));
    for closer in ["agenda:marsh/steps[2]", "agenda:bert/steps[2]"] {
        assert!(
            checks
                .no_return
                .iter()
                .any(|p| p.accepted_by.iter().any(|a| a == closer)),
            "no point of no return names {closer}: {:#?}",
            checks.no_return
        );
    }
    assert!(
        checks
            .no_return
            .iter()
            .flat_map(|p| &p.reasons)
            .all(|r| !r.is_empty())
    );
    // All five endings are reached.
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

#[test]
fn demo_hints_and_chart_come_from_the_walkthroughs() {
    let (project, world) = demo();
    let (key, best) = project
        .walkthroughs
        .iter()
        .find(|(p, _)| p == "tests/best.yaml")
        .unwrap();
    let chart = step_chart(key, best).expect("best has step ids");
    let ids: Vec<_> = chart.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        ["boots", "crane", "drain", "pages", "weapon", "accuse"]
    );
    let accuse = chart
        .steps
        .iter()
        .find(|s| s.id.as_str() == "accuse")
        .unwrap();
    assert_eq!(accuse.depth, 3, "{chart:#?}");

    // In the kitchen at the start, the ladder offers the boots.
    let mut session = Session::new(world.clone());
    session.set_hints(HintBook::new(world, &project.walkthroughs));
    session.start();
    session.submit(Input::Text("w".into()));
    let open: Vec<_> = session.hints().into_iter().map(|h| h.step).collect();
    assert_eq!(open, ["boots"]);
    let text = session.submit(Input::Text("hint".into())).plain_text();
    assert!(text.contains("Look at what he wore."), "{text}");
}

#[test]
fn slice_best_transcript() {
    // The slice's best walkthrough, as P4-01 left it passing.
    let files: Vec<_> = read_dir(&slice("yaml")).unwrap();
    let project = load(files).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let (_, walk) = project
        .walkthroughs
        .iter()
        .find(|(p, _)| p == "tests/slice_best.yaml")
        .unwrap();
    let result = run_walkthrough(world, walk, "slice_best");
    assert!(result.passed(), "{:#?}", result.failures);
    assert_transcript("slice_best", &result.transcript);
}
