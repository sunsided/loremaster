//! The `world!` forms of the golden slice (roadmap phase 0, P0-02 and
//! P0-03), with the cargo feature `dsl`. The parts in today's syntax compile
//! since the world! core (DSL step L3, P3-03 #66); dsl.rs and mixed.rs keep
//! the other parts out of the build until world! has their syntax.
//!
//! `slice_equivalence` lives here and not in loremaster-runtime's slice.rs,
//! because the DSL and mixed forms need the macro.
#![cfg(feature = "dsl")]

#[path = "../../../examples/lock-keeper-slice/dsl.rs"]
mod dsl;

#[path = "../../../examples/lock-keeper-slice/mixed.rs"]
mod mixed;

use loremaster::dsl::project::{Origin, SourceFile, load, model_hash, read_dir};
use loremaster_runtime::{World, run_walkthrough};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The parts of the slice that use syntax the model does not know yet, as in
/// loremaster-runtime's tests/slice.rs.
const NEW_SYNTAX: &[&str] = &[
    "agendas/",
    "dialogues/",
    "texts/",
    "things/stove.yaml",
    "rules/keeper.yaml",
];

fn form(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/lock-keeper-slice")
        .join(name)
}

/// The files of a YAML folder in today's syntax.
fn today(files: Vec<SourceFile>) -> Vec<SourceFile> {
    let mut files = files;
    files.retain(|f| !NEW_SYNTAX.iter().any(|p| f.path.starts_with(*p)));
    files
}

fn expected_hash() -> String {
    std::fs::read_to_string(form("expected/model.hash"))
        .expect("expected/model.hash")
        .trim()
        .to_string()
}

/// The lists of a story whose order means nothing, sorted by id, as
/// `model_hash` sorts them: a failing comparison shows the difference.
fn sorted(story: &loremaster::dsl::model::Story) -> loremaster::dsl::model::Story {
    let mut s = story.clone();
    s.rooms.sort_by(|a, b| a.id.cmp(&b.id));
    s.things.sort_by(|a, b| a.id.cmp(&b.id));
    s.endings.sort_by(|a, b| a.id.cmp(&b.id));
    s.timers.sort_by(|a, b| a.id.cmp(&b.id));
    s.machines.sort_by(|a, b| a.id.cmp(&b.id));
    s.vars.sort_by(|a, b| a.id.cmp(&b.id));
    s.threads.sort_by(|a, b| a.id.cmp(&b.id));
    s.dialogues.sort_by(|a, b| a.id.cmp(&b.id));
    s.relations.sort();
    s
}

#[test]
fn slice_equivalence() {
    // The three forms must assemble to one canonical model (DS-11..DS-15).
    // This compares the part in today's syntax, with the endings; the parts
    // with new syntax join when they load and when world! has their syntax.
    let expected = expected_hash();

    let yaml = load(today(read_dir(&form("yaml")).unwrap())).unwrap();
    assert_eq!(model_hash(&yaml.story), expected, "yaml");

    let mixed = mixed::today(today(mixed::yaml_files()))
        .assemble()
        .unwrap_or_else(|e| panic!("mixed: {e:#?}"));
    assert_eq!(model_hash(&mixed.story), expected, "mixed");

    let dsl = dsl::with(&[dsl::Part::Endings])
        .assemble()
        .unwrap_or_else(|e| panic!("dsl: {e:#?}"));
    assert_eq!(sorted(&dsl.story), sorted(&yaml.story), "dsl");
    assert_eq!(model_hash(&dsl.story), expected, "dsl");
}

#[test]
fn the_dsl_slice_checks_and_has_rust_origins() {
    let (project, warnings) = dsl::with(&[dsl::Part::Endings])
        .check()
        .unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(warnings.is_empty(), "{warnings:#?}");
    // Each entity points at its id in the world! block.
    let Origin::Rust {
        file, line, column, ..
    } = &project.origins["room:kitchen"]
    else {
        panic!("not from Rust: {:?}", project.origins["room:kitchen"]);
    };
    assert!(file.ends_with("lock-keeper-slice/dsl/today.rs"), "{file}");
    let source = std::fs::read_to_string(form("dsl/today.rs")).unwrap();
    let at = source.lines().nth(*line as usize - 1).unwrap();
    assert_eq!(&at[*column as usize - 1..], "kitchen \"Kitchen\" {", "{at}");
}

/// The walkthroughs of dsl/tests.rs without a pending mark pass on the DSL
/// form, as the YAML ones do in `slice_baseline`.
#[test]
fn slice_dsl_walkthroughs() {
    let project = dsl::with(&[dsl::Part::Endings])
        .pack(dsl::tests::pack())
        .assemble()
        .unwrap_or_else(|e| panic!("{e:#?}"));
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut ran = Vec::new();
    // These pass on the YAML form (D6 P4-02 #79, P4-01 #78); they need parts
    // that world! does not have yet: the agenda (dsl/agenda.rs), the text
    // pool of the dialogue and the stove's `one_of` (dsl/texts.rs, dsl/stove.rs).
    let needs_parts = |id: &str| {
        [
            "slice_keeper_agenda",
            "slice_keeper_dialogue",
            "slice_stove_text",
            "slice_talk_pauses_agenda",
            "slice_best",
            "slice_ashes",
        ]
        .contains(&id)
    };
    for (id, walk) in project
        .walkthroughs
        .iter()
        .filter(|(id, w)| w.pending.is_none() && !needs_parts(id))
    {
        let result = run_walkthrough(world.clone(), walk, id);
        assert!(
            result.passed(),
            "{id}: {:#?}\n\n{}",
            result.failures,
            result.transcript
        );
        ran.push(id.as_str());
    }
    ran.sort();
    assert_eq!(
        ran,
        [
            "slice_baseline",
            "slice_drowning",
            "slice_endings",
            "slice_undo_after_death",
        ]
    );
}

/// The test blocks give the same walkthroughs as the YAML files, step by
/// step: dsl/tests.rs `test <id>` is yaml/tests/<id>.yaml.
#[test]
fn the_dsl_tests_equal_the_yaml_tests() {
    let yaml = load(today(read_dir(&form("yaml")).unwrap())).unwrap();
    let dsl = dsl::with(&[dsl::Part::Endings])
        .pack(dsl::tests::pack())
        .assemble()
        .unwrap();
    assert_eq!(dsl.walkthroughs.len(), yaml.walkthroughs.len());
    for (id, walk) in &dsl.walkthroughs {
        let path = format!("tests/{id}.yaml");
        let (_, yaml_walk) = yaml
            .walkthroughs
            .iter()
            .find(|(p, _)| *p == path)
            .unwrap_or_else(|| panic!("no {path}"));
        assert_eq!(walk, yaml_walk, "{id}");
    }
}

/// dsl/dialogue.rs with an inline line in place of the text pool
/// keeper_remark (R9), which world! does not have yet. Remove it when
/// dsl/dialogue.rs compiles.
mod keeper_dialogue_inline {
    use loremaster::prelude::*;

    world! {
        extern {
            thing keeper;
            var ash_is_fresh;
        }

        dialogue with keeper start [stove] {
            stove: "Mind your step." {
                says "Who fed it?" when ash_is_fresh => who_fed_it;
                says "What do you burn at night?" => what_burns;
                says "Good night." => end;
            }
            what_burns: says "Coal, when the company sends it. Driftwood, when it does not." {
                says "And paper?" => who_fed_it;
                says "Good night." => end;
            }
            who_fed_it: "He turns the poker in his hands." says "Nobody. The stove was cold all night." {
                says "Good night." => end;
            }
        }
    }
}

/// keeper_dialogue_inline with the node who_fed_it in the screenplay form
/// of dsl/dialogue.rs (spike, #112).
mod keeper_dialogue_screenplay {
    use loremaster::prelude::*;

    world! {
        extern {
            thing keeper;
            var ash_is_fresh;
        }

        dialogue with keeper start [stove] {
            stove: "Mind your step." {
                says "Who fed it?" when ash_is_fresh => who_fed_it;
                says "What do you burn at night?" => what_burns;
                says "Good night." => end;
            }
            what_burns: says "Coal, when the company sends it. Driftwood, when it does not." {
                says "And paper?" => who_fed_it;
                says "Good night." => end;
            }
            who_fed_it: (He turns the poker in his hands.) "Nobody. The stove was cold all night." {
                says "Good night." => end;
            }
        }
    }
}

/// The keeper's dialogue in world! is the one of dialogues/keeper.yaml, with
/// the same inline line in place of the text pool in both forms.
#[test]
fn the_dsl_dialogue_equals_the_yaml_dialogue() {
    let path = "dialogues/keeper.yaml";
    let mut file = read_dir(&form("yaml"))
        .unwrap()
        .into_iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path}"));
    let pool = "say: { text: keeper_remark }";
    assert!(file.text.contains(pool), "{}", file.text);
    file.text = file.text.replace(pool, "say: \"Mind your step.\"");
    let yaml = loremaster::dsl::Pack::from_files(vec![file]).unwrap_or_else(|e| panic!("{e:#?}"));
    let dsl = keeper_dialogue_inline::pack();
    assert_eq!(dsl.dialogues, yaml.dialogues);
    assert_eq!(keeper_dialogue_screenplay::pack().dialogues, yaml.dialogues);
}

#[test]
#[ignore = "pending: phase 4, world! texts and one_of after P4-01 #78 (dsl/texts.rs, dsl/stove.rs, the pool of dsl/dialogue.rs), agendas in world! after P4-02 #79"]
fn slice_equivalence_all_parts() {
    // The whole slice, with the parts in new syntax: the YAML form loads only
    // when the model knows that syntax, and the world! forms compile only
    // when the macro does.
    let yaml = load(read_dir(&form("yaml")).unwrap()).unwrap_or_else(|e| panic!("yaml: {e:#?}"));
    let dsl = dsl::world()
        .assemble()
        .unwrap_or_else(|e| panic!("dsl: {e:#?}"));
    let mixed = mixed::world()
        .assemble()
        .unwrap_or_else(|e| panic!("mixed: {e:#?}"));
    assert_eq!(sorted(&dsl.story), sorted(&yaml.story), "dsl");
    assert_eq!(model_hash(&mixed.story), model_hash(&yaml.story), "mixed");
}
