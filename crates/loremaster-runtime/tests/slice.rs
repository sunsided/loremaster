//! The golden slice of the roadmap (phase 0): a part of the Lock-Keeper demo
//! in `examples/lock-keeper-slice`, written before the engine can run it.
//!
//! `slice_baseline` passes today. Each other test is pending: it names the
//! roadmap phase and the requirement or issue that make it pass. Run them with
//! `task test:pending`. Read `examples/lock-keeper-slice/PENDING.md`.

use loremaster_project::{
    Diagnostic, Project, Severity, SourceFile, check, load, model_hash, read_dir,
};
use loremaster_runtime::authoring;
use loremaster_runtime::{Input, Session, StepOutcome, Turn, WalkResult, World, run_walkthrough};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One typed command as a whole turn.
fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

/// The parts of the slice that use syntax the model does not know yet. Today's
/// loader rejects them, or skips their folder with a warning. Each test loads
/// today's files plus only the parts it needs, so that each feature turns its
/// own test green.
const AGENDA: &str = "agendas/";
const DIALOGUE: &str = "dialogues/";
const TEXTS: &str = "texts/";
const STOVE: &str = "things/stove.yaml";
const KEEPER_RULE: &str = "rules/keeper.yaml";
const NEW_SYNTAX: &[&str] = &[AGENDA, DIALOGUE, TEXTS, STOVE, KEEPER_RULE];

/// Walkthroughs without a pending mark that need a part, and run in their
/// own test with it. The agenda loads since D6 (P4-02 #79), but it stays a
/// part: `slice_model_hash` and `slice_equivalence` compare today's syntax,
/// and world! has no agendas yet.
const WITH_PARTS: &[&str] = &["tests/slice_keeper_agenda.yaml"];

fn form(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/lock-keeper-slice")
        .join(name)
}

fn files(form_name: &str) -> Vec<SourceFile> {
    read_dir(&form(form_name)).unwrap_or_else(|e| panic!("{form_name}: {e}"))
}

/// The files of a form in today's syntax, plus the given new parts.
fn with(form_name: &str, parts: &[&str]) -> Vec<SourceFile> {
    let mut files = files(form_name);
    files.retain(|f| {
        let new = |p: &&str| f.path.starts_with(*p);
        !NEW_SYNTAX.iter().any(new) || parts.iter().any(new)
    });
    files
}

fn project(files: Vec<SourceFile>) -> Project {
    load(files).unwrap_or_else(|e| panic!("the slice does not load: {e:#?}"))
}

/// Load today's YAML files plus the given parts, and play one walkthrough.
fn play(test: &str, parts: &[&str]) -> WalkResult {
    let project = project(with("yaml", parts));
    let path = format!("tests/{test}.yaml");
    let walk = project
        .walkthroughs
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, w)| w.clone())
        .unwrap_or_else(|| panic!("no walkthrough {path}"));
    let world = Arc::new(World::compile(project.story).unwrap());
    run_walkthrough(world, &walk, test)
}

/// The walkthrough passes. Its `pending:` mark does not count here: the
/// pending job expects this test to fail until the mark goes.
fn assert_passes(test: &str, parts: &[&str]) {
    let result = play(test, parts);
    assert!(
        result.passed(),
        "{test}: {:#?}\n\n{}",
        result.failures,
        result.transcript
    );
}

#[test]
fn slice_baseline() {
    let project = project(with("yaml", &[]));
    let errors: Vec<_> = check(&project)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");

    // Every walkthrough without a `pending:` mark passes on today's files.
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut ran = Vec::new();
    for (path, walk) in project
        .walkthroughs
        .iter()
        .filter(|(p, w)| w.pending.is_none() && !WITH_PARTS.contains(&p.as_str()))
    {
        let result = run_walkthrough(world.clone(), walk, path);
        assert!(
            result.passed(),
            "{path}: {:#?}\n\n{}",
            result.failures,
            result.transcript
        );
        ran.push(path.as_str());
    }
    ran.sort();
    assert_eq!(
        ran,
        [
            "tests/slice_baseline.yaml",
            "tests/slice_drowning.yaml",
            "tests/slice_endings.yaml",
            "tests/slice_undo_after_death.yaml",
        ]
    );
}

#[test]
fn slice_endings() {
    // WO-1: endings are entities with an id, a kind, a rank and a text.
    let story = project(with("yaml", &[])).story;
    let kinds: Vec<_> = story
        .endings
        .iter()
        .map(|e| (e.id.as_str(), e.kind.as_str(), e.rank))
        .collect();
    assert_eq!(
        kinds,
        [
            ("ashes", "loss", Some(3)),
            ("committed", "win", Some(1)),
            ("drowned", "death", None),
        ]
    );
    assert_passes("slice_endings", &[]);
}

#[test]
fn slice_save_envelope() {
    // A save from the middle of slice_best: the pages are taken.
    let project = project(with("yaml", &[]));
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world.clone());
    authoring::start(&mut game);
    for command in [
        "up",
        "take windlass",
        "down",
        "e",
        "open lower paddles",
        "wait",
        "wait",
        "open gear housing",
        "take pages",
    ] {
        input(&mut game, command);
    }
    let save = serde_json::to_value(game.save()).unwrap();
    assert_eq!(
        save["format"], "loremaster-save",
        "no save envelope: {save:#}"
    );
    assert!(save["version"].is_u64(), "no save version: {save:#}");

    let (mut game, loaded) = Session::load(world, save).unwrap();
    assert!(!loaded.story_changed);
    input(&mut game, "w");
    let text = input(&mut game, "accuse keeper").plain_text();
    assert!(text.contains("Committed for trial"), "{text}");
}

/// The model hash of the YAML form, in today's syntax. A change of the slice
/// or of the model changes it: then update `expected/model.hash` on purpose.
fn expected_hash() -> String {
    std::fs::read_to_string(form("expected/model.hash"))
        .expect("expected/model.hash")
        .trim()
        .to_string()
}

#[test]
fn slice_model_hash() {
    let hash = model_hash(&project(with("yaml", &[])).story);
    assert_eq!(
        hash,
        expected_hash(),
        "the model of the slice changed; if that is right, write {hash} into examples/lock-keeper-slice/expected/model.hash"
    );
}

// `slice_equivalence` (YAML, DSL and mixed give one model hash) is in
// crates/loremaster-cli/tests/slice_dsl.rs: the DSL and mixed forms need the
// world! macro, with the cargo feature `dsl` of the crate `loremaster`.

#[test]
fn slice_undo_after_death() {
    assert_passes("slice_undo_after_death", &[]);
}

#[test]
#[ignore = "pending: phase 4, R9 (text pools), P4-01 #78"]
fn slice_talk_pauses_agenda() {
    assert_passes("slice_talk_pauses_agenda", &[AGENDA, DIALOGUE, TEXTS]);
}

/// `slice_talk_pauses_agenda` without the text pool (R9): the first node of
/// the dialogue says the line of the pool that shows in the kitchen. The
/// rest is the slice as it is: the talk holds the agenda step back, and the
/// step runs late, in the turn that ends the talk.
#[test]
fn slice_talk_pauses_agenda_without_the_text_pool() {
    let mut files = with("yaml", &[AGENDA, DIALOGUE]);
    let dialogue = files
        .iter_mut()
        .find(|f| f.path == "dialogues/keeper.yaml")
        .unwrap();
    let pool = "say: { text: keeper_remark }";
    assert!(dialogue.text.contains(pool), "{}", dialogue.text);
    dialogue.text = dialogue.text.replace(
        pool,
        r#"say: [{ says: "Do not touch the stove," }, { narrate: "the lock-keeper says." }]"#,
    );
    let project = project(files);
    let (_, walk) = project
        .walkthroughs
        .iter()
        .find(|(p, _)| p == "tests/slice_talk_pauses_agenda.yaml")
        .unwrap();
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    let result = run_walkthrough(world, walk, "slice_talk_pauses_agenda");
    assert!(
        result.passed(),
        "{:#?}\n\n{}",
        result.failures,
        result.transcript
    );
}

#[test]
#[ignore = "pending: phase 4, TX, D2, P4-01 #78"]
fn slice_stove_text() {
    assert_passes("slice_stove_text", &[STOVE, AGENDA]);
}

#[test]
#[ignore = "pending: phase 4, R9 (text pools), P4-01 #78"]
fn slice_keeper_dialogue() {
    assert_passes("slice_keeper_dialogue", &[DIALOGUE, TEXTS]);
}

#[test]
fn slice_keeper_agenda() {
    assert_passes("slice_keeper_agenda", &[AGENDA]);

    // The step is over in the state, and its `closes` mark is in the model
    // for the ending checks (P4-04).
    let project = project(with("yaml", &[AGENDA]));
    let agenda = project
        .story
        .agenda_of("keeper")
        .expect("the keeper's agenda");
    assert_eq!(agenda.steps[0].closes, ["committed".into()]);
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    for _ in 0..17 {
        input(&mut game, "wait");
    }
    assert_eq!(game.state().step_outcome("keeper", "1"), None);
    let turn = input(&mut game, "wait");
    assert_eq!(
        game.state().step_outcome("keeper", "1"),
        Some(StepOutcome::Ran),
        "{}",
        turn.plain_text()
    );
}

/// The schema checks plus the ending checks of the explorer, with files.
fn ending_checks(project: &Project) -> Vec<Diagnostic> {
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    let explored = authoring::explore(world, authoring::Budget::default());
    let mut out = check(project);
    out.extend(
        explored
            .ending_checks
            .diagnostics()
            .into_iter()
            .map(|d| project.locate(d)),
    );
    out
}

#[test]
#[ignore = "pending: phase 4, R9 text pools P4-01 #78 (the dialogue, texts and stove of NEW_SYNTAX; the agenda and the ending checks work)"]
fn slice_endings_reachable() {
    // Two checks on the endings. A dead end (a state from which no ending can
    // be reached) is an error. A point of no return (a step after which the
    // best reachable ending gets worse) is a warning, unless the step names
    // the endings it closes. The keeper's agenda step closes `committed`; the
    // ashes ending stays reachable, so the slice has no dead end.
    let marked = ending_checks(&project(with("yaml", NEW_SYNTAX)));
    assert!(
        !marked.iter().any(|d| d.severity == Severity::Error),
        "{marked:#?}"
    );
    assert!(
        !marked.iter().any(|d| d.file == "agendas/keeper.yaml"),
        "the `closes` mark does not accept the rank drop: {marked:#?}"
    );

    // Without the mark, the check reports the rank drop and names the ending.
    let mut files = with("yaml", NEW_SYNTAX);
    let agenda = files
        .iter_mut()
        .find(|f| f.path == "agendas/keeper.yaml")
        .unwrap();
    agenda.text = agenda
        .text
        .lines()
        .filter(|l| {
            !l.trim_start().starts_with("closes:") && !l.trim_start().starts_with("reason:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let unmarked = ending_checks(&project(files));
    assert!(
        unmarked.iter().any(|d| d.file == "agendas/keeper.yaml"
            && d.severity == Severity::Warning
            && d.message.contains("committed")),
        "no warning for the rank drop at the agenda step: {unmarked:#?}"
    );
}

#[test]
#[ignore = "pending: phase 4, P4-07 #84"]
fn slice_best() {
    assert_passes("slice_best", NEW_SYNTAX);
}

#[test]
#[ignore = "pending: phase 4, P4-07 #84"]
fn slice_ashes() {
    assert_passes("slice_ashes", NEW_SYNTAX);
}
