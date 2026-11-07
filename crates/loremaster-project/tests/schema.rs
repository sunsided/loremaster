//! The schema checks as one library function (XP-5): the YAML loader and the
//! `world!` macro run `check_packs` on their packs.

use loremaster_model::{Room, Rule, Thing};
use loremaster_project::{
    Origin, Pack, Severity, SourceFile, StoryHeader, check, check_files, check_packs, load,
    read_dir,
};
use serde_json::json;
use std::path::Path;

fn lighthouse() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

fn rust(line: u32) -> Origin {
    Origin::Rust {
        file: "src/world.rs".into(),
        line,
        column: 5,
        pack: "world".into(),
    }
}

/// A pack as the `world!` macro would make it: model values with origins in
/// Rust, and no files.
fn rust_pack() -> Pack {
    let mut pack = Pack::new("world");
    pack.header = Some(
        serde_json::from_value::<StoryHeader>(json!({
            "title": "In Rust",
            "start": "hall",
            "vars": [{ "id": "turns", "type": "int", "initial": 0 }],
        }))
        .unwrap(),
    );
    pack.origins.insert("story".into(), rust(1));
    pack.rooms.push(
        serde_json::from_value::<Room>(json!({
            "id": "hall",
            "name": "Hall",
            "exits": [{ "direction": "north", "to": "garden" }],
        }))
        .unwrap(),
    );
    pack.origins.insert("room:hall".into(), rust(3));
    pack.things.push(
        serde_json::from_value::<Thing>(json!({
            "id": "lamp",
            "name": "lamp",
            "location": { "room": "hall" },
            "props": { "lit": false },
        }))
        .unwrap(),
    );
    pack.origins.insert("thing:lamp".into(), rust(9));
    pack.rules.push(
        serde_json::from_value::<Rule>(json!({
            "id": "glow",
            "on": "turn",
            "when": "lamp.lt",
            "then": [{ "say": "It glows." }],
        }))
        .unwrap(),
    );
    pack.origins.insert("rule:glow".into(), rust(14));
    pack
}

#[test]
fn a_pack_from_rust_gets_the_checks_without_files() {
    let checked = check_packs(vec![rust_pack()]);
    assert!(checked.project.is_some());
    assert!(checked.has_errors());
    let found: Vec<(&str, &str, Option<u32>, &str)> = checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.file.as_str(), d.at.as_str(), d.line, d.message.as_str()))
        .collect();
    // Each diagnostic has the Rust file and line of its entity's origin.
    let has = |at: &str, line: u32, text: &str| {
        found.iter().any(|(f, a, l, m)| {
            *f == "src/world.rs" && *a == at && *l == Some(line) && m.contains(text)
        })
    };
    // A reference, a condition, and a reserved name; the variable is declared
    // in the header, which comes from Rust too.
    assert!(has("exits[0].to", 3, "unknown room `garden`"), "{found:#?}");
    assert!(
        has(
            "when",
            14,
            "`lamp` has no property `lt`; did you mean `lamp.lit`?"
        ),
        "{found:#?}"
    );
    assert!(has("vars.turns", 1, "the variable `turns`"), "{found:#?}");
}

#[test]
fn packs_that_do_not_join_give_no_project() {
    let checked = check_packs(vec![rust_pack(), rust_pack()]);
    assert!(checked.project.is_none());
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|d| d.message.starts_with("two packs give the story header")),
        "{:#?}",
        checked.diagnostics
    );
}

#[test]
fn the_loader_runs_the_same_checks() {
    let checked = check_files(lighthouse());
    let project = load(lighthouse()).unwrap();
    assert_eq!(checked.diagnostics, check(&project));
    assert!(!checked.has_errors());
    assert_eq!(
        checked.project.map(|p| p.story),
        Some(project.story),
        "check_files loads the same story"
    );

    // A file that does not read is an error of the same function.
    let mut files = lighthouse();
    files.push(SourceFile {
        path: "room/attic.yaml".into(),
        text: "name: Attic\n".into(),
    });
    let checked = check_files(files);
    assert!(checked.project.is_none());
    assert!(checked.has_errors());
}
