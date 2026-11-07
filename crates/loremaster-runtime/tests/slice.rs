//! The golden slice of the roadmap (phase 0): a part of the Lock-Keeper demo
//! in `examples/lock-keeper-slice`, written before the engine can run it.
//!
//! `slice_baseline` passes today. Each other test is pending: it names the
//! roadmap phase and the requirement or issue that make it pass. Run them with
//! `task test:pending`. Read `examples/lock-keeper-slice/PENDING.md`.

use loremaster_project::{Project, Severity, SourceFile, check, load, model_hash, read_dir};
use loremaster_runtime::{Game, WalkResult, World, run_walkthrough};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
        .filter(|(_, w)| w.pending.is_none())
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
    let mut game = Game::new(world.clone());
    game.start();
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
        game.input(command);
    }
    let save = serde_json::to_value(game.save()).unwrap();
    assert_eq!(
        save["format"], "loremaster-save",
        "no save envelope: {save:#}"
    );
    assert!(save["version"].is_u64(), "no save version: {save:#}");

    let restored = Game::load_save(world, save).unwrap();
    assert!(!restored.story_changed);
    let mut game = restored.game;
    game.input("w");
    let text = game.input("accuse keeper").plain_text();
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

#[test]
#[ignore = "pending: phase 3, DS-4, P3-03 #66 (world! core, DSL step L3) and P4-08 #85 (L4)"]
fn slice_equivalence() {
    // The three forms must assemble to one canonical model (DS-11..DS-15).
    // This compares the part in today's syntax; the parts with new syntax join
    // when they load. The `world!` files of dsl/ and mixed/ need the macro, so
    // today's loader cannot read those forms.
    let hash = |name: &str| model_hash(&project(with(name, &[])).story);
    let expected = expected_hash();
    assert_eq!(hash("yaml"), expected, "yaml");
    assert_eq!(hash("mixed"), expected, "mixed");
    assert_eq!(hash("dsl"), expected, "dsl");
}

#[test]
#[ignore = "pending: phase 2, SE-8, SE-9, P2-03 #54"]
fn slice_undo_after_death() {
    assert_passes("slice_undo_after_death", &[]);
}

#[test]
#[ignore = "pending: phase 4, D6, P4-02 #79, P3-15 #111"]
fn slice_talk_pauses_agenda() {
    assert_passes("slice_talk_pauses_agenda", &[AGENDA, DIALOGUE, TEXTS]);
}

#[test]
#[ignore = "pending: phase 4, TX, D2, P4-01 #78"]
fn slice_stove_text() {
    assert_passes("slice_stove_text", &[STOVE, AGENDA]);
}

#[test]
#[ignore = "pending: phase 3, D3, P3-01 #64, P3-15 #111"]
fn slice_keeper_dialogue() {
    assert_passes("slice_keeper_dialogue", &[DIALOGUE, TEXTS]);
}

#[test]
#[ignore = "pending: phase 4, D6, P4-02 #79"]
fn slice_keeper_agenda() {
    assert_passes("slice_keeper_agenda", &[AGENDA]);
}

#[test]
#[ignore = "pending: phase 4, CK, P4-04 #81"]
fn slice_endings_reachable() {
    // Two checks on the endings. A dead end (a state from which no ending can
    // be reached) is an error. A point of no return (a step after which the
    // best reachable ending gets worse) is a warning, unless the step names
    // the endings it closes. The keeper's agenda step closes `committed`; the
    // ashes ending stays reachable, so the slice has no dead end.
    let marked = check(&project(with("yaml", NEW_SYNTAX)));
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
    let unmarked = check(&project(files));
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
