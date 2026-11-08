//! Packs and assemble (DS-1, DS-2): each source makes a pack, and assemble
//! joins packs in their order.

use loremaster_model::{Machine, Room};
use loremaster_project::{Origin, Pack, SourceFile, assemble, load, read_dir};
use std::path::Path;

fn lighthouse() -> Vec<SourceFile> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    read_dir(&dir).unwrap()
}

fn rust_origin(line: u32) -> Origin {
    Origin::Rust {
        file: "src/world.rs".into(),
        line,
        column: 5,
        pack: "extra".into(),
        hash: String::new(),
    }
}

fn messages(
    result: Result<loremaster_project::Project, Vec<loremaster_project::Diagnostic>>,
) -> Vec<String> {
    match result {
        Ok(_) => panic!("the packs assembled"),
        Err(d) => d.into_iter().map(|d| d.message).collect(),
    }
}

#[test]
fn load_is_one_yaml_pack_assembled() {
    let pack = Pack::from_files(lighthouse()).unwrap();
    assert_eq!(pack.name, "yaml");
    assert_eq!(pack.origins.get("story"), Some(&Origin::yaml("story.yaml")));
    assert_eq!(
        pack.origins.get("room:shore").map(Origin::file),
        Some("rooms/shore.yaml")
    );
    assert_eq!(
        assemble(vec![pack]).unwrap().story,
        load(lighthouse()).unwrap().story
    );
}

#[test]
fn two_packs_give_the_same_story_as_one() {
    // The oil lamp in a second YAML pack, without a story header.
    let (lamp, rest): (Vec<_>, Vec<_>) = lighthouse()
        .into_iter()
        .partition(|f| f.path == "things/oil_lamp.yaml");
    let first = Pack::from_files(rest).unwrap();
    let second = Pack::from_files(lamp).unwrap();
    assert!(second.header.is_none());
    let joined = assemble(vec![first, second]).unwrap();
    assert_eq!(joined.story, load(lighthouse()).unwrap().story);
    assert_eq!(
        joined.origins.get("thing:oil_lamp").map(Origin::file),
        Some("things/oil_lamp.yaml")
    );
}

#[test]
fn an_id_in_two_packs_names_both_origins() {
    let mut extra = Pack::new("extra");
    let shore: Room = serde_saphyr::from_str("id: shore\nname: Shore\n").unwrap();
    extra.rooms.push(shore);
    extra.origins.insert("room:shore".into(), rust_origin(3));
    let found = messages(assemble(vec![
        Pack::from_files(lighthouse()).unwrap(),
        extra,
    ]));
    assert_eq!(
        found,
        ["`room:shore` is defined twice: in rooms/shore.yaml and in src/world.rs:3:5 (pack extra)"]
    );
}

#[test]
fn exactly_one_pack_gives_the_story_header() {
    let yaml = || Pack::from_files(lighthouse()).unwrap();
    let mut second = yaml();
    second.rooms.clear();
    second.things.clear();
    second.rules.clear();
    second.timers.clear();
    second.endings.clear();
    second.walkthroughs.clear();
    second.origins.retain(|k, _| k == "story");
    second.origins.insert("story".into(), rust_origin(1));
    let found = messages(assemble(vec![yaml(), second]));
    assert_eq!(
        found,
        ["two packs give the story header: story.yaml and src/world.rs:1:5 (pack extra)"]
    );

    let mut headless = yaml();
    headless.header = None;
    headless.origins.remove("story");
    assert_eq!(
        messages(assemble(vec![headless])),
        ["the project has no story.yaml"]
    );
}

#[test]
fn a_machine_from_a_second_pack_gets_its_rules_and_variable() {
    let machine: Machine = serde_saphyr::from_str(
        "id: tide\nstart: low\nsteps:\n  - { from: low, on: { verb: [wait] }, to: high }\n",
    )
    .unwrap();
    let mut extra = Pack::new("extra");
    extra.machines.push(machine);
    extra.origins.insert("machine:tide".into(), rust_origin(7));
    let project = assemble(vec![Pack::from_files(lighthouse()).unwrap(), extra]).unwrap();
    assert!(project.story.var("tide").is_some());
    assert!(
        project
            .story
            .rules
            .iter()
            .any(|r| r.id.as_str() == "tide#1")
    );
    // The step rule has the place of its machine, with the machine's hash.
    assert_eq!(
        project.origins["rule:tide#1"],
        project.origins["machine:tide"]
    );
    assert_eq!(project.origins["rule:tide#1"].file(), "src/world.rs");
}

#[test]
fn the_header_origin_names_the_file_that_gave_it() {
    let mut files = lighthouse();
    for f in &mut files {
        if f.path == "story.yaml" {
            f.path = "story.yml".into();
        }
    }
    let project = load(files).unwrap();
    assert_eq!(
        project.origins.get("story"),
        Some(&Origin::yaml("story.yml"))
    );
}

/// A second YAML pack with one file.
fn second(path: &str, text: &str) -> Pack {
    Pack::from_files(vec![SourceFile {
        path: path.into(),
        text: text.into(),
    }])
    .unwrap()
}

#[test]
fn replace_takes_the_place_of_the_earlier_entity_with_a_warning() {
    let extra = second(
        "rooms/shore.yaml",
        "replace: true\nname: The Beach\ndescription: [\"Sand.\"]\nexits:\n  - { direction: north, to: cottage }\n",
    );
    assert!(extra.replaces.contains("room:shore"));
    let project = assemble(vec![Pack::from_files(lighthouse()).unwrap(), extra]).unwrap();
    let shore = project.story.room("shore").unwrap();
    assert_eq!(shore.name, "The Beach");
    assert_eq!(
        project
            .story
            .rooms
            .iter()
            .filter(|r| r.id.as_str() == "shore")
            .count(),
        1
    );
    let warnings: Vec<_> = loremaster_project::check(&project)
        .into_iter()
        .filter(|d| d.message.contains("replaces"))
        .map(|d| d.message)
        .collect();
    assert_eq!(
        warnings,
        ["`room:shore` replaces the one in rooms/shore.yaml"]
    );
}

#[test]
fn extend_adds_fragments_and_exits_to_the_earlier_entity() {
    let extra = second(
        "rooms/cottage.yaml",
        "extend: true\ndescription: [\"A gull sits on the sill.\"]\n",
    );
    assert_eq!(extra.extensions.len(), 1);
    let base = load(lighthouse()).unwrap();
    let project = assemble(vec![Pack::from_files(lighthouse()).unwrap(), extra]).unwrap();
    let before = base.story.room("cottage").unwrap().description.len();
    let after = &project.story.room("cottage").unwrap().description;
    assert_eq!(after.len(), before + 1);
    assert_eq!(
        project.story.room("cottage").unwrap().name,
        base.story.room("cottage").unwrap().name
    );
}

#[test]
fn extend_and_replace_report_conflicts_and_missing_entities() {
    let yaml = || Pack::from_files(lighthouse()).unwrap();
    let conflict = second("rooms/cottage.yaml", "extend: true\nname: The Hut\n");
    assert_eq!(
        messages(assemble(vec![yaml(), conflict])),
        ["`room:cottage` cannot be extended: both packs set `name`"]
    );
    let nowhere = second(
        "rooms/attic.yaml",
        "extend: true\ndescription: [\"Dust.\"]\n",
    );
    assert_eq!(
        messages(assemble(vec![yaml(), nowhere])),
        ["`room:attic` extends an entity that no earlier pack defines"]
    );
    let nothing = second("rooms/attic.yaml", "replace: true\nname: Attic\n");
    assert_eq!(
        messages(assemble(vec![yaml(), nothing])),
        ["`room:attic` replaces an entity that no earlier pack defines"]
    );
}

#[test]
fn the_id_of_an_extension_comes_from_the_parsed_file() {
    // Flow style: the id is not on a line of its own.
    let extra = second(
        "rooms/other_name.yaml",
        "extend: true\n{ id: cottage, description: [\"A gull sits on the sill.\"] }\n",
    );
    assert_eq!(extra.extensions[0].key, "room:cottage");
}

#[test]
fn replace_drops_the_nested_rules_of_the_replaced_file() {
    // rooms/cellar.yaml with a nested rule, then a replacement without it.
    let mut files = lighthouse();
    files.push(SourceFile {
        path: "rooms/cellar.yaml".into(),
        text: "name: Cellar\nrules:\n  - id: cellar_drip\n    on: { enter: cellar }\n    then: [{ say: \"Drip.\" }]\n".into(),
    });
    let first = Pack::from_files(files).unwrap();
    assert!(first.rules.iter().any(|r| r.id.as_str() == "cellar_drip"));
    let extra = second("rooms/cellar.yaml", "replace: true\nname: Dry Cellar\n");
    let project = assemble(vec![first, extra]).unwrap();
    assert!(
        !project
            .story
            .rules
            .iter()
            .any(|r| r.id.as_str() == "cellar_drip")
    );
    assert!(!project.origins.contains_key("rule:cellar_drip"));
    assert_eq!(project.story.room("cellar").unwrap().name, "Dry Cellar");
}

#[test]
fn the_model_hash_does_not_depend_on_the_order_of_files_or_packs() {
    use loremaster_project::model_hash;
    let one = load(lighthouse()).unwrap();
    // The oil lamp first, in a pack of its own: the things come in another order.
    let (lamp, rest): (Vec<_>, Vec<_>) = lighthouse()
        .into_iter()
        .partition(|f| f.path == "things/oil_lamp.yaml");
    let mut lamp_first = Pack::from_files(lamp).unwrap();
    let rest = Pack::from_files(rest).unwrap();
    lamp_first.header = None;
    let two = assemble(vec![lamp_first, rest]).unwrap();
    assert_ne!(one.story.things, two.story.things, "the order differs");
    assert_eq!(model_hash(&one.story), model_hash(&two.story));
}

#[test]
fn a_json_file_that_is_not_an_export_is_an_error() {
    for text in ["{}", r#"{ "name": "w", "roomz": [] }"#] {
        let files = vec![SourceFile {
            path: "packs/w.json".into(),
            text: text.into(),
        }];
        let found = messages(load(files));
        assert!(found[0].starts_with("not an exported pack"), "{found:?}");
    }
}
