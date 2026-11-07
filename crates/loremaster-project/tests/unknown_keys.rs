//! The loader rejects unknown fields in every entity file and unknown folders
//! of a project, with the nearest known name when there is one.

use loremaster_project::{Diagnostic, Pack, Severity, SourceFile, assemble, check, load, read_dir};
use std::path::Path;

fn lighthouse() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

/// The lighthouse with one file replaced or added.
fn lighthouse_with(path: &str, text: &str) -> Vec<SourceFile> {
    let mut files = lighthouse();
    files.retain(|f| f.path != path);
    files.push(SourceFile {
        path: path.into(),
        text: text.into(),
    });
    files
}

/// The errors of loading the lighthouse with one file replaced or added.
fn errors(path: &str, text: &str) -> Vec<Diagnostic> {
    match load(lighthouse_with(path, text)) {
        Ok(_) => panic!("{path} loads, but it should not:\n{text}"),
        Err(errors) => errors,
    }
}

/// The one error names the file and the unknown field, and suggests `near`.
fn assert_unknown_field(path: &str, text: &str, field: &str, near: &str) {
    let found = errors(path, text);
    assert!(
        found.iter().any(|d| d.file == path
            && d.line.is_some()
            && d.message
                .contains(&format!("unknown field `{field}`: did you mean `{near}`?"))),
        "{found:#?}"
    );
}

#[test]
fn a_misspelled_field_of_an_entity_is_an_error_with_a_suggestion() {
    assert_unknown_field(
        "rooms/attic.yaml",
        "name: Attic\ndescripton: [\"Dust.\"]\n",
        "descripton",
        "description",
    );
    assert_unknown_field(
        "rooms/attic.yaml",
        "name: Attic\nexits:\n  - { direction: down, to: shore, visibel: \"true\" }\n",
        "visibel",
        "visible",
    );
    assert_unknown_field(
        "things/rope.yaml",
        "name: rope\nlocation: { room: shore }\nportible: true\n",
        "portible",
        "portable",
    );
    assert_unknown_field(
        "things/rope.yaml",
        "name: rope\nlocation: { room: shore }\nnames:\n  - { text: rope, whn: \"true\" }\n",
        "whn",
        "when",
    );
    assert_unknown_field(
        "rules/wave.yaml",
        "on: { verb: [wait] }\nthen: [{ say: Waves. }]\nonce: true\nprority: 2\n",
        "prority",
        "priority",
    );
    assert_unknown_field(
        "timers/tide.yaml",
        "turns: 3\nrunning: true\nrepat: true\n",
        "repat",
        "repeat",
    );
    assert_unknown_field(
        "verbs/wave.yaml",
        "label: Wave\nwords: [wave]\nslot: none\n",
        "slot",
        "slots",
    );
    assert_unknown_field(
        "machines/tide.yaml",
        "start: low\nsteps:\n  - { from: low, on: turn, to: high, sya: Rising. }\n",
        "sya",
        "say",
    );
    assert_unknown_field(
        "machines/tide.yaml",
        "start: low\nstates:\n  - low\n  - { name: high, descripton: Wet. }\n",
        "descripton",
        "description",
    );
}

#[test]
fn a_misspelled_field_of_a_fragment_is_an_error_not_a_variant_mismatch() {
    // Fragments are a text or a map. An untagged enum would only say that
    // the data matches no variant.
    assert_unknown_field(
        "rooms/attic.yaml",
        "name: Attic\ndescription:\n  - Dust.\n  - { text: Bats., whne: \"true\" }\n",
        "whne",
        "when",
    );
}

#[test]
fn a_misspelled_field_in_story_yaml_is_an_error() {
    let story = lighthouse()
        .into_iter()
        .find(|f| f.path == "story.yaml")
        .unwrap()
        .text;
    let bad = story.replacen("minutes_per_turn", "minute_per_turn", 1);
    assert_ne!(bad, story, "the lighthouse has no `minutes_per_turn`");
    assert_unknown_field("story.yaml", &bad, "minute_per_turn", "minutes_per_turn");
}

#[test]
fn a_misspelled_field_of_a_dialogue_node_or_option_is_an_error() {
    // The outcomes of a node and of an option are flattened in the model;
    // serde does not reject unknown fields of a flattened struct.
    let dialogue = |node: &str, option: &str| {
        format!(
            "with: keeper_ghost\nstart: [meet]\nnodes:\n  meet:\n    say: Hello.\n    {node}\n    options:\n      - {{ say: Bye., go: end, {option} }}\n"
        )
    };
    let ghost = "name: ghost\nlocation: { room: shore }\n";
    for (path, text, field, near) in [
        (
            "dialogues/ghost.yaml",
            dialogue("remeber: [met]", "set: {}"),
            "remeber",
            "remember",
        ),
        (
            "dialogues/ghost.yaml",
            dialogue("remember: [met]", "relaton: { trust: 1 }"),
            "relaton",
            "relation",
        ),
    ] {
        let mut files = lighthouse_with(path, &text);
        files.push(SourceFile {
            path: "things/keeper_ghost.yaml".into(),
            text: ghost.into(),
        });
        let found = load(files).expect_err("loads");
        assert!(
            found.iter().any(|d| d.file == path
                && d.message
                    .contains(&format!("unknown field `{field}`: did you mean `{near}`?"))),
            "{found:#?}"
        );
    }
}

#[test]
fn a_misspelled_field_of_a_walkthrough_step_or_expect_is_an_error() {
    // A step with checks flattens `expect` into the step in the model.
    assert_unknown_field(
        "tests/short.yaml",
        "steps:\n  - { do: look, sees: Shore }\n",
        "sees",
        "see",
    );
    assert_unknown_field(
        "tests/short.yaml",
        "steps: [look]\nexpect:\n  rooom: shore\n",
        "rooom",
        "room",
    );
    // A field that is far from every known one keeps serde's list.
    let found = errors("tests/short.yaml", "steps:\n  - { do: look, banana: 1 }\n");
    assert!(
        found.iter().any(|d| d
            .message
            .contains("unknown field `banana`, expected one of")),
        "{found:#?}"
    );
}

#[test]
fn an_extend_file_with_a_misspelled_field_is_an_error() {
    let extra = Pack::from_files(vec![SourceFile {
        path: "rooms/shore.yaml".into(),
        text: "extend: true\ntagz: [wet]\n".into(),
    }])
    .unwrap();
    let found = assemble(vec![Pack::from_files(lighthouse()).unwrap(), extra]).unwrap_err();
    assert!(
        found.iter().any(|d| d
            .message
            .contains("unknown field `tagz`: did you mean `tags`?")),
        "{found:#?}"
    );
}

#[test]
fn an_unknown_folder_is_an_error_with_a_suggestion() {
    let found = errors("ending/lost.yaml", "kind: loss\ntitle: Lost\n");
    assert!(
        found.iter().any(|d| d.file == "ending/lost.yaml"
            && d.message == "unknown folder `ending/`: did you mean `endings/`?"),
        "{found:#?}"
    );
    let found = errors("scenes/one.yaml", "name: One\n");
    assert!(
        found.iter().any(|d| d.message.starts_with(
            "unknown folder `scenes/`: a project has story.yaml and the folders rooms/"
        )),
        "{found:#?}"
    );
    // A folder inside a known folder is not read either.
    let found = errors("rooms/old/attic.yaml", "name: Attic\n");
    assert!(
        found
            .iter()
            .any(|d| d.message.starts_with("unknown folder `rooms/old/`")),
        "{found:#?}"
    );
}

#[test]
fn an_unknown_file_at_the_top_is_an_error() {
    let found = errors("stroy.yaml", "title: X\n");
    assert!(
        found
            .iter()
            .any(|d| d.message == "unknown file `stroy.yaml`: did you mean `story.yaml`?"),
        "{found:#?}"
    );
}

#[test]
fn the_folders_of_planned_parts_are_skipped_with_a_warning() {
    // The golden slice keeps drafts in `agendas/` and `texts/`.
    let project = load(lighthouse_with(
        "agendas/keeper.yaml",
        "actor: keeper\nsteps: []\n",
    ))
    .unwrap();
    let found = check(&project);
    assert!(
        found.iter().any(|d| d.severity == Severity::Warning
            && d.file == "agendas/keeper.yaml"
            && d.message.contains("`agendas/` is not read yet")),
        "{found:#?}"
    );
    assert!(!found.iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn other_files_than_yaml_are_not_read() {
    // `behaviors/` may hold any text; a Rust file next to the YAML is not read.
    let mut files = lighthouse();
    files.push(SourceFile {
        path: "agendas/keeper.rs".into(),
        text: "world! {}".into(),
    });
    files.push(SourceFile {
        path: "README.md".into(),
        text: "# The Lighthouse".into(),
    });
    assert!(load(files).is_ok());
}

#[test]
fn the_warnings_stay_when_the_project_has_errors() {
    let mut files = lighthouse_with("agendas/keeper.yaml", "actor: keeper\nsteps: []\n");
    files.push(SourceFile {
        path: "ending/lost.yaml".into(),
        text: "kind: loss\ntitle: Lost\n".into(),
    });
    let checked = loremaster_project::check_files(files);
    assert!(checked.project.is_none());
    let has = |severity, text: &str| {
        checked
            .diagnostics
            .iter()
            .any(|d| d.severity == severity && d.message.contains(text))
    };
    assert!(
        has(Severity::Error, "unknown folder `ending/`"),
        "{checked:#?}"
    );
    assert!(
        has(Severity::Warning, "`agendas/` is not read yet"),
        "{checked:#?}"
    );

    // `assemble` keeps the warnings of joining too, here a replaced room.
    let extra = Pack::from_files(vec![
        SourceFile {
            path: "rooms/shore.yaml".into(),
            text: "replace: true\nname: The Beach\n".into(),
        },
        SourceFile {
            path: "rooms/cottage.yaml".into(),
            text: "extend: true\nname: Twice\n".into(),
        },
    ])
    .unwrap();
    let found = assemble(vec![Pack::from_files(lighthouse()).unwrap(), extra]).unwrap_err();
    assert!(
        found.iter().any(|d| d.severity == Severity::Warning
            && d.message.contains("`room:shore` replaces")),
        "{found:#?}"
    );
    assert!(
        found.iter().any(|d| d.severity == Severity::Error),
        "{found:#?}"
    );
}
