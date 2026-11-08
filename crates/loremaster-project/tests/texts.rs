//! Text pools in the checks (P4-01): dramaturge's `validate_pools` issues as
//! loremaster diagnostics, at the file of the pool.

use loremaster_project::{Diagnostic, Severity, SourceFile, check, load, read_dir};
use std::path::Path;

fn lighthouse_with(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut all = read_dir(&dir).unwrap();
    for (path, text) in files {
        all.push(SourceFile {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    check(&load(all).unwrap())
}

#[test]
fn pool_issues_are_diagnostics_of_the_pool_file() {
    let found = lighthouse_with(&[(
        "texts/remark.yaml",
        r#"
pick: best
one_of:
  - when: beacon_lit
    text: "A."
  - when: drawer_searched
    text: "B."
  - mode: cycle
    variants: ["C.", "D."]
  - text: { text: nowhere }
"#,
    )]);
    let in_file = |kind: Severity, what: &str| {
        found.iter().any(|d| {
            d.file == "texts/remark.yaml" && d.severity == kind && d.message.contains(what)
        })
    };
    assert!(in_file(Severity::Warning, "same score"), "{found:#?}");
    assert!(in_file(Severity::Warning, "has no key"), "{found:#?}");
    assert!(in_file(Severity::Error, "`nowhere`"), "{found:#?}");
}

#[test]
fn a_group_learns_only_bool_variables_and_says_known_pools() {
    let found = lighthouse_with(&[
        (
            "rules/remark.yaml",
            "id: remark\non: { verb: [jump] }\nthen:\n  - say: { text: missing }\n",
        ),
        (
            "things/note.yaml",
            "name: note\nlocation: { room: shore }\ndescription:\n  - pick: first\n    one_of:\n      - text: \"A note.\"\n        learn: [no_such_var]\n",
        ),
    ]);
    assert!(
        found.iter().any(|d| d.file == "rules/remark.yaml"
            && d.severity == Severity::Error
            && d.message.contains("unknown text pool `missing`")),
        "{found:#?}"
    );
    assert!(
        found.iter().any(|d| d.file == "things/note.yaml"
            && d.severity == Severity::Error
            && d.message.contains("`no_such_var` is not one")),
        "{found:#?}"
    );
}

#[test]
fn texts_that_cannot_show_where_they_are_written_are_errors() {
    let found = lighthouse_with(&[
        (
            "texts/remark.yaml",
            "id: keeper_remark\none_of: [\"Hm.\"]\n",
        ),
        (
            "rules/remark.yaml",
            "id: remark\non: { verb: [jump] }\nwhen: seen_by(player)\nthen:\n  - say: { text: keeper_remark }\n",
        ),
        (
            "things/note.yaml",
            "name: note\nlocation: { room: shore }\ndescription:\n  - { id: a, text: \"One.\" }\n  - { id: a, text: \"Two.\" }\n  - one_of:\n      - { key: k, text: \"Three.\" }\n      - { key: k, text: \"Four.\" }\n",
        ),
    ]);
    let error = |file: &str, what: &str| {
        found
            .iter()
            .any(|d| d.file == file && d.severity == Severity::Error && d.message.contains(what))
    };
    // The pool's `id` wins over the file name, so the rule finds it.
    assert!(
        !error("rules/remark.yaml", "unknown text pool"),
        "{found:#?}"
    );
    assert!(
        error("rules/remark.yaml", "`seen_by` works only"),
        "{found:#?}"
    );
    assert!(error("things/note.yaml", "the key `a`"), "{found:#?}");
    assert!(error("things/note.yaml", "the key `k`"), "{found:#?}");
}

#[test]
fn an_option_cannot_say_a_pool() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let mut files = read_dir(&dir).unwrap();
    let d = files
        .iter_mut()
        .find(|f| f.path == "dialogues/blackbeard.yaml")
        .unwrap();
    let at = d
        .text
        .find("    options:\n      - say: ")
        .expect("an option with say");
    let start = at + "    options:\n      - say: ".len();
    let end = start + d.text[start..].find('\n').unwrap();
    d.text.replace_range(start..end, "{ text: remark }");
    files.push(SourceFile {
        path: "texts/remark.yaml".into(),
        text: "one_of: [\"Hm.\"]\n".into(),
    });
    let found = check(&load(files).unwrap());
    assert!(
        found.iter().any(|d| d.file == "dialogues/blackbeard.yaml"
            && d.severity == Severity::Error
            && d.message.contains("not here")),
        "{found:#?}"
    );
}

#[test]
fn two_pools_with_one_id_are_an_error() {
    let pool = "id: remark\none_of:\n  - \"A.\"\n";
    let found = lighthouse_with(&[
        ("texts/a.yaml", pool),
        ("texts/b.yaml", pool),
        ("texts/c.yaml", pool),
    ]);
    let twice: Vec<_> = found
        .iter()
        .filter(|d| {
            d.severity == Severity::Error
                && d.message.contains("text pool `remark` is defined twice")
        })
        .collect();
    // One error, at the file that the id's origin names.
    assert_eq!(twice.len(), 1, "{found:#?}");
    assert_eq!(twice[0].file, "texts/c.yaml");
}

#[test]
fn an_id_that_is_the_index_of_another_fragment_is_an_error() {
    // A fragment or candidate without an id is keyed by its index.
    let found = lighthouse_with(&[(
        "things/note.yaml",
        "name: note\nlocation: { room: shore }\ndescription:\n  - { text: \"A note.\", mode: once }\n  - { id: \"0\", text: \"B.\", mode: once }\n  - one_of:\n      - { mode: once, text: \"C.\" }\n      - { key: \"0\", mode: once, text: \"D.\" }\n",
    )]);
    let error = |what: &str| {
        found.iter().any(|d| {
            d.file == "things/note.yaml"
                && d.severity == Severity::Error
                && d.message.contains(what)
        })
    };
    assert!(
        error("two fragments of this text have the key `0`"),
        "{found:#?}"
    );
    assert!(
        error("two candidates of this `one_of` have the key `0`"),
        "{found:#?}"
    );
}
