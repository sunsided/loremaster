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
            && d.message.contains("no_such_var")),
        "{found:#?}"
    );
}
