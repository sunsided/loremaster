//! The builders one by one: effects, walkthrough steps, the paths of YAML
//! sources and the places of diagnostics.

use loremaster_dsl::prelude::*;
use loremaster_model::{Expect, StartTimers, WalkStep};
use loremaster_project::{SourceFile, read_dir};
use std::path::Path;

fn lighthouse_files() -> Vec<SourceFile> {
    read_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse")).unwrap()
}

#[test]
fn effects_join_into_one_map_only_for_new_keys() {
    let mut p = Pack::new("effects");
    p.rule("r", on::turn())
        .set("a", true)
        .set("b", 1)
        .add("gold", -5)
        .add("gold", 5)
        .move_to("lamp", "cellar")
        .move_to("lamp", "carried")
        .start_timer_for("fuse", 3)
        .start_timer_for("fuse", 4);
    let then = &p.rules[0].then;
    assert_eq!(
        then,
        &vec![
            Effect::Set([("a".into(), Value::Bool(true)), ("b".into(), Value::Int(1))].into()),
            // Two YAML entries: the runtime adds both, so the net is 0.
            Effect::Add([("gold".into(), -5)].into()),
            Effect::Add([("gold".into(), 5)].into()),
            Effect::Move([("lamp".into(), "cellar".into())].into()),
            Effect::Move([("lamp".into(), "carried".into())].into()),
            Effect::Start(StartTimers::Many([("fuse".into(), 3)].into())),
            Effect::Start(StartTimers::Many([("fuse".into(), 4)].into())),
        ]
    );
}

#[test]
fn step_checks_go_to_the_step_and_expect_checks_to_the_end() {
    let mut p = Pack::new("walk");
    p.test("Drawer")
        .step("open drawer")
        .sees("an iron key")
        .fact("drawer_searched", true)
        .in_room("cottage")
        .ended(false)
        .step("light beacon")
        .ending("beacon_lit")
        .expect_ended(true);
    let walk = &p.walkthroughs[0].1;
    let WalkStep::Checked {
        command, expect, ..
    } = &walk.steps[0]
    else {
        panic!("not a checked step");
    };
    assert_eq!(command, "open drawer");
    assert_eq!(expect.see, ["an iron key"]);
    assert_eq!(
        expect.facts.get("drawer_searched"),
        Some(&Value::Bool(true))
    );
    assert_eq!(expect.room.as_ref().map(|r| r.as_str()), Some("cottage"));
    assert_eq!(expect.ended, Some(false));
    let WalkStep::Checked { expect, .. } = &walk.steps[1] else {
        panic!("not a checked step");
    };
    assert_eq!(
        expect.ending.as_ref().map(|e| e.as_str()),
        Some("beacon_lit")
    );
    assert_eq!(
        walk.expect,
        Expect {
            ended: Some(true),
            ..Expect::default()
        }
    );
}

#[rustfmt::skip]
fn two_on_one_line(p: &mut Pack) {
    p.test("b").step("look"); p.test("c").step("look");
}

#[test]
fn each_test_has_its_own_key() {
    let mut p = Pack::new("walk");
    two_on_one_line(&mut p);
    for name in ["d", "e"] {
        p.test(name).step("look");
    }
    let keys: Vec<&str> = p.walkthroughs.iter().map(|(k, _)| k.as_str()).collect();
    assert_ne!(keys[0], keys[1]);
    assert_eq!(keys[3], format!("{}#2", keys[2]));
}

#[test]
fn two_yaml_sources_keep_their_paths_apart() {
    // One YAML source: the paths stay relative to its folder.
    let one = Loremaster::new()
        .yaml_files(lighthouse_files())
        .assemble()
        .unwrap();
    assert_eq!(one.origins["room:shore"], Origin::yaml("rooms/shore.yaml"));
    // Two: each path starts with its source.
    let errors = Loremaster::new()
        .yaml_files(lighthouse_files())
        .pack(Pack::new("between"))
        .yaml_files(lighthouse_files())
        .assemble()
        .unwrap_err();
    assert!(
        errors.iter().any(|d| d.message
            == "two packs give the story header: yaml1/story.yaml and yaml2/story.yaml"),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|d| d.message
            == "`room:shore` is defined twice: in yaml1/rooms/shore.yaml and in yaml2/rooms/shore.yaml"),
        "{errors:?}"
    );
}

#[test]
fn a_diagnostic_about_a_rust_entity_has_its_line_and_column() {
    let mut p = Pack::new("broken");
    p.story("Broken", "cellar");
    p.room("cellar", "Cellar").text("Dark.");
    let line = line!();
    p.rule("bad", on::turn()).when("no_such_var").say("Never.");
    let errors = Loremaster::new().pack(p).check().unwrap_err();
    let bad = errors
        .iter()
        .find(|d| d.message == "unknown name `no_such_var`")
        .unwrap_or_else(|| panic!("no diagnostic for the rule: {errors:?}"));
    assert_eq!(bad.file, file!());
    assert_eq!((bad.line, bad.column), (Some(line + 1), Some(7)));
}

#[test]
fn a_diagnostic_about_a_rust_story_header_has_its_line() {
    let mut p = Pack::new("broken");
    let line = line!();
    p.story("Broken", "nowhere_room");
    p.room("cellar", "Cellar").text("Dark.");
    let errors = Loremaster::new().pack(p).check().unwrap_err();
    let start = errors
        .iter()
        .find(|d| d.at == "start")
        .unwrap_or_else(|| panic!("no diagnostic for the start room: {errors:?}"));
    assert_eq!(start.file, file!());
    assert_eq!((start.line, start.column), (Some(line + 1), Some(7)));
}
