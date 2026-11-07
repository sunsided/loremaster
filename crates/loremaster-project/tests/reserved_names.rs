//! A variable, machine, room or thing must not take a built-in name of the
//! condition language: conditions would read the built-in instead.

use loremaster_lang::RESERVED_NAMES;
use loremaster_project::{Diagnostic, Severity, SourceFile, check, load, read_dir};
use std::path::Path;

fn lighthouse() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

fn errors(files: Vec<SourceFile>) -> Vec<Diagnostic> {
    check(&load(files).unwrap())
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn with(mut files: Vec<SourceFile>, path: &str, text: &str) -> Vec<SourceFile> {
    files.retain(|f| f.path != path);
    files.push(SourceFile {
        path: path.into(),
        text: text.into(),
    });
    files
}

fn assert_reported(found: &[Diagnostic], file: &str, at: &str, kind_and_id: &str) {
    assert!(
        found.iter().any(|d| d.file == file
            && d.at == at
            && d.message.starts_with(&format!(
                "the {kind_and_id} has a name that conditions use for a built-in"
            ))),
        "{found:#?}"
    );
}

#[test]
fn a_room_or_thing_with_a_reserved_name_is_an_error_at_its_file() {
    let files = with(lighthouse(), "rooms/here.yaml", "name: Here\n");
    assert_reported(&errors(files), "rooms/here.yaml", "id", "room `here`");

    let files = with(
        lighthouse(),
        "things/chance.yaml",
        "name: chance\nlocation: nowhere\n",
    );
    assert_reported(&errors(files), "things/chance.yaml", "id", "thing `chance`");

    // An `id:` in the file counts, not the file name.
    let files = with(
        lighthouse(),
        "things/glance.yaml",
        "id: seen_by\nname: glance\nlocation: nowhere\n",
    );
    assert_reported(
        &errors(files),
        "things/glance.yaml",
        "id",
        "thing `seen_by`",
    );
}

#[test]
fn a_variable_or_machine_with_a_reserved_name_is_an_error_at_its_declaration() {
    let mut files = lighthouse();
    let story = files.iter_mut().find(|f| f.path == "story.yaml").unwrap();
    story.text = story.text.replacen(
        "vars:\n",
        "vars:\n  - { id: turns, type: int, initial: 0 }\n",
        1,
    );
    assert!(
        story.text.contains("id: turns"),
        "the lighthouse has no `vars:`"
    );
    assert_reported(
        &errors(files),
        "story.yaml",
        "vars.turns",
        "variable `turns`",
    );

    let files = with(
        lighthouse(),
        "machines/level.yaml",
        "start: low\nsteps:\n  - { from: low, on: turn, to: high }\n",
    );
    assert_reported(
        &errors(files),
        "machines/level.yaml",
        "machines.level",
        "machine `level`",
    );
}

#[test]
fn a_variable_or_machine_with_a_phase_name_is_an_error() {
    // A bare `dusk` tests the phase only while no variable is named `dusk`.
    let mut files = lighthouse();
    let story = files.iter_mut().find(|f| f.path == "story.yaml").unwrap();
    story.text = story.text.replacen(
        "vars:\n",
        "vars:\n  - { id: dusk, type: bool, initial: false }\n",
        1,
    );
    let found = errors(files);
    assert!(
        found.iter().any(|d| d.file == "story.yaml"
            && d.at == "vars.dusk"
            && d.message
                .starts_with("the variable `dusk` has the name of a phase")),
        "{found:#?}"
    );

    let files = with(
        lighthouse(),
        "machines/dawn.yaml",
        "start: low\nsteps:\n  - { from: low, on: turn, to: high }\n",
    );
    let found = errors(files);
    assert!(
        found.iter().any(|d| d.file == "machines/dawn.yaml"
            && d.message
                .starts_with("the machine `dawn` has the name of a phase")),
        "{found:#?}"
    );

    // A room may have a phase name: a bare name reads the phase, and a room
    // is never a flag.
    let files = with(lighthouse(), "rooms/day.yaml", "name: Day\n");
    assert!(
        !errors(files).iter().any(|d| d.message.contains("phase")),
        "a room with a phase name is reported"
    );
}

#[test]
fn the_list_holds_the_derived_values_and_the_functions() {
    for name in [
        "here",
        "turns",
        "level",
        "first_visit",
        "time",
        "action",
        "timer",
        "true",
        "false",
        "in",
        "carried",
        "present",
        "visited",
        "chance",
        "remembers",
        "seen_by",
    ] {
        assert!(RESERVED_NAMES.contains(&name), "{name}");
    }
}

#[test]
fn the_examples_take_no_reserved_name() {
    for example in ["lighthouse", "cantina", "tower", "zork1"] {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(example);
        let found: Vec<_> = errors(read_dir(&dir).unwrap())
            .into_iter()
            .filter(|d| d.message.contains("built-in"))
            .collect();
        assert!(found.is_empty(), "{example}: {found:#?}");
    }
}
