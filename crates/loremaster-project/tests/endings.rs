//! Endings are entities in `endings/`, and the `end` effect names one.

use loremaster_project::{Severity, SourceFile, check, load, read_dir};
use std::path::Path;

/// The lighthouse with one file replaced or added.
fn lighthouse_with(path: &str, text: &str) -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&dir).unwrap();
    files.retain(|f| f.path != path);
    files.push(SourceFile {
        path: path.into(),
        text: text.into(),
    });
    files
}

fn messages(files: Vec<SourceFile>) -> Vec<(Severity, String, String)> {
    let project = load(files).unwrap();
    check(&project)
        .into_iter()
        .map(|d| (d.severity, d.file, d.message))
        .collect()
}

#[test]
fn an_ending_loads_with_its_kind_and_rank() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&dir).unwrap()).unwrap();
    let ending = project.story.ending("beacon_lit").unwrap();
    assert_eq!(ending.kind.as_str(), "win");
    assert_eq!(ending.rank, Some(1));
    assert_eq!(ending.title, "The beacon is lit");
    assert_eq!(
        project.origins.get("ending:beacon_lit").map(|o| o.file()),
        Some("endings/beacon_lit.yaml")
    );
}

#[test]
fn an_unknown_kind_or_field_does_not_load() {
    let bad_kind = "kind: victory\ntitle: X\n";
    assert!(load(lighthouse_with("endings/x.yaml", bad_kind)).is_err());
    let bad_field = "kind: win\ntitle: X\nscore: 10\n";
    assert!(load(lighthouse_with("endings/x.yaml", bad_field)).is_err());
}

#[test]
fn end_must_name_an_ending() {
    let rule = "- id: give_up\n  on: { verb: [wait] }\n  then:\n    - end: gave_up\n";
    let found = messages(lighthouse_with("rules/give_up.yaml", rule));
    assert!(
        found.iter().any(|(s, f, m)| *s == Severity::Error
            && f == "rules/give_up.yaml"
            && m.contains("unknown ending `gave_up`")),
        "{found:#?}"
    );
}

#[test]
fn an_ending_that_no_end_names_is_a_warning() {
    let ending = "kind: loss\nrank: 2\ntitle: The ship is lost\n";
    let found = messages(lighthouse_with("endings/ship_lost.yaml", ending));
    assert!(
        found.iter().any(|(s, f, m)| *s == Severity::Warning
            && f == "endings/ship_lost.yaml"
            && m.contains("no `end` effect names ending `ship_lost`")),
        "{found:#?}"
    );
}

#[test]
fn rank_zero_and_bad_markup_are_errors() {
    let ending = "kind: win\nrank: 0\ntitle: The beacon is lit\ntext: \"{nosuchvar ? a | b}\"\n";
    let found = messages(lighthouse_with("endings/beacon_lit.yaml", ending));
    let errors: Vec<_> = found
        .iter()
        .filter(|(s, f, _)| *s == Severity::Error && f == "endings/beacon_lit.yaml")
        .map(|(_, _, m)| m.as_str())
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("the best rank is 1")),
        "{found:#?}"
    );
    assert_eq!(errors.len(), 2, "{found:#?}");
}
