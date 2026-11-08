//! Overrides of Rust parts (DS-11 to DS-15), on the reference example
//! `examples/rust-overrides`.
#![cfg(feature = "dsl")]

#[path = "../../../examples/rust-overrides/world.rs"]
mod world;

mod ported {
    use loremaster::prelude::*;
    include!("../../../examples/rust-overrides/ported.rs");
}

use loremaster::dsl::project::{
    Origin, OverrideState, Pack, Severity, SourceFile, load, model_hash, read_dir, world_snippet,
};
use loremaster::prelude::*;
use std::path::Path;

fn override_files() -> Vec<SourceFile> {
    read_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rust-overrides/override"))
        .expect("the example")
}

fn with_override(pack: Pack) -> Loremaster {
    Loremaster::new().pack(pack).yaml_files(override_files())
}

fn hash_of(pack: &Pack, key: &str) -> String {
    let mut pack = pack.clone();
    pack.stamp_hashes();
    match &pack.origins[key] {
        Origin::Rust { hash, .. } => hash.clone(),
        other => panic!("{other:?}"),
    }
}

/// The world with the shore text changed in Rust.
fn changed_in_rust() -> Pack {
    let mut pack = world::pack();
    pack.rooms[0].description = vec![Fragment::text("Sand, now.")];
    pack
}

#[test]
fn the_hash_follows_the_model_not_the_source() {
    // Another place in the source, the same model value.
    let mut moved = world::pack();
    if let Some(Origin::Rust { line, .. }) = moved.origins.get_mut("room:shore") {
        *line += 40;
    }
    assert_eq!(
        hash_of(&world::pack(), "room:shore"),
        hash_of(&moved, "room:shore")
    );
    assert_ne!(
        hash_of(&world::pack(), "room:shore"),
        hash_of(&changed_in_rust(), "room:shore")
    );
}

#[test]
fn an_unchanged_base_is_a_warning() {
    let (project, found) = with_override(world::pack()).check().expect("no error");
    let d = found
        .iter()
        .find(|d| d.message.starts_with("override active, port it to the DSL"))
        .expect("the warning");
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.file, "rooms/shore.yaml");
    assert!(!found.iter().any(|d| d.message.contains("replaces the one")));
    // The override takes the place of the Rust room.
    assert_eq!(
        project
            .story
            .rooms
            .iter()
            .find(|r| r.id.as_str() == "shore")
            .unwrap()
            .description[0]
            .variants[0],
        "Shingle crunches underfoot. The tide is out."
    );
    let part = &project.rust_parts["room:shore"];
    assert_eq!(
        part.overridden_by,
        Some(("rooms/shore.yaml".to_string(), OverrideState::Active))
    );
}

#[test]
fn a_changed_base_is_an_error_with_a_three_way_diff() {
    let found = with_override(changed_in_rust()).check().unwrap_err();
    let d = found
        .iter()
        .find(|d| d.message.starts_with("both sides changed"))
        .expect("the error");
    assert_eq!(d.severity, Severity::Error);
    assert!(
        d.message
            .contains("base:     [\"Shingle crunches underfoot.\"]"),
        "{}",
        d.message
    );
    assert!(
        d.message.contains("Rust now: [\"Sand, now.\"]"),
        "{}",
        d.message
    );
    assert!(
        d.message
            .contains("override: [\"Shingle crunches underfoot. The tide is out.\"]"),
        "{}",
        d.message
    );
}

#[test]
fn rust_equal_to_the_override_is_an_info() {
    let mut pack = world::pack();
    pack.rooms[0].description = vec![Fragment::text(
        "Shingle crunches underfoot. The tide is out.",
    )];
    let (project, found) = with_override(pack).check().expect("no error");
    let d = found
        .iter()
        .find(|d| d.message.starts_with("override no longer needed"))
        .expect("the info");
    assert_eq!(d.severity, Severity::Info);
    assert_eq!(
        project.rust_parts["room:shore"].overridden_by,
        Some((
            "rooms/shore.yaml".to_string(),
            OverrideState::NoLongerNeeded
        ))
    );
}

#[test]
fn a_rust_part_that_is_gone_is_an_error() {
    let mut pack = world::pack();
    pack.origins.remove("room:shore");
    pack.rooms.remove(0);
    let found = with_override(pack).check().unwrap_err();
    assert!(
        found
            .iter()
            .any(|d| d.message.contains("no Rust part has this id"))
    );
}

#[test]
fn a_yaml_only_load_keeps_the_override_with_a_warning() {
    let mut files = override_files();
    files.push(SourceFile {
        path: "story.yaml".into(),
        text: "title: T\nstart: cottage\n".into(),
    });
    files.push(SourceFile {
        path: "rooms/cottage.yaml".into(),
        text: "name: C\nexits:\n  - direction: south\n    to: shore\n".into(),
    });
    let project = load(files).expect("loads");
    assert!(
        project
            .assembly
            .iter()
            .any(|d| d.message.contains("not compared"))
    );
}

#[test]
fn the_exported_pack_gives_the_same_model() {
    let mut pack = world::pack();
    let hash = hash_of(&pack, "room:shore");
    let exported = pack.to_json();
    let files = vec![SourceFile {
        path: "packs/world.json".into(),
        text: exported,
    }];
    let project = load(files).expect("loads");
    let direct = Loremaster::new()
        .pack(world::pack())
        .assemble()
        .expect("assembles");
    assert_eq!(model_hash(&project.story), model_hash(&direct.story));
    assert!(matches!(&project.origins["room:shore"], Origin::Rust { hash: h, .. } if *h == hash));
}

#[test]
fn an_override_prints_as_a_snippet_that_compiles() {
    let pack = Pack::from_files(override_files()).expect("reads");
    let snippet = world_snippet(&pack, "room:shore").expect("a room");
    assert_eq!(
        snippet,
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rust-overrides/ported.rs")
        )
        .unwrap()
    );
    // The file compiled above (`mod ported`), and it says what the override says.
    assert_eq!(ported::pack().rooms[0], pack.rooms[0]);
}

#[test]
#[should_panic(expected = "both sides changed")]
fn the_whole_world_test_fails_on_drift() {
    with_override(changed_in_rust()).assert_world();
}

#[test]
#[should_panic(expected = "strict")]
fn the_whole_world_test_fails_on_warnings_only_when_strict() {
    with_override(world::pack()).assert_world();
    with_override(world::pack()).strict().assert_world();
}

// The generated test, on the world without an override.
loremaster::dsl::world_test!(Loremaster::new().pack(world::pack()));

#[test]
fn an_override_whose_body_has_another_id_is_an_error() {
    let mut files = override_files();
    let shore = files
        .iter_mut()
        .find(|f| f.path == "rooms/shore.yaml")
        .unwrap();
    shore.text = shore.text.replace("\nid: shore\n", "\nid: beach\n");
    let found = Loremaster::new()
        .pack(world::pack())
        .yaml_files(files)
        .check()
        .unwrap_err();
    assert!(
        found.iter().any(|d| d
            .message
            .contains("overrides `room:shore`, but does not define it")),
        "{found:?}"
    );
}

#[test]
fn an_export_removes_stale_packs() {
    /// Removes the folder also when an assert fails.
    struct Gone(std::path::PathBuf);
    impl Drop for Gone {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let dir = Gone(std::env::temp_dir().join(format!("loremaster-export-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&dir.0);
    std::fs::create_dir_all(dir.0.join("packs")).unwrap();
    std::fs::write(dir.0.join("packs/old.json"), "{}").unwrap();
    let paths = Loremaster::new()
        .pack(world::pack())
        .export_packs(&dir.0)
        .unwrap();
    let left: Vec<_> = std::fs::read_dir(dir.0.join("packs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(left, paths);
}

#[test]
fn a_second_file_of_the_overridden_entity_does_not_blame_the_override() {
    let mut files = override_files();
    let shore = files.iter().find(|f| f.path == "rooms/shore.yaml").unwrap();
    // Sorted after the override, so its origin is the last one.
    files.push(SourceFile {
        path: "rooms/shore.yml".into(),
        text: shore
            .text
            .lines()
            .skip_while(|l| !l.starts_with("id:"))
            .collect::<Vec<_>>()
            .join("\n"),
    });
    let found = Loremaster::new()
        .pack(world::pack())
        .yaml_files(files)
        .check()
        .err()
        .unwrap_or_default();
    assert!(
        !found
            .iter()
            .any(|d| d.message.contains("but does not define it")),
        "{found:?}"
    );
}

#[test]
fn a_nested_json_file_in_packs_is_not_read() {
    let files = vec![
        SourceFile {
            path: "packs/world.json".into(),
            text: world::pack().to_json(),
        },
        SourceFile {
            path: "packs/backup/old.json".into(),
            text: "{}".into(),
        },
    ];
    load(files).expect("loads");
}
