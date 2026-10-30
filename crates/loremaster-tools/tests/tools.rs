use assert2::assert;
use loremaster_project::{SourceFile, read_dir};
use loremaster_tools::{Workspace, tools};
use serde_json::{Value, json};
use std::path::Path;

fn files(name: &str) -> Vec<SourceFile> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    read_dir(&root).unwrap()
}

fn call(ws: &mut Workspace, name: &str, args: Value) -> Value {
    ws.call(name, args)
        .unwrap_or_else(|e| panic!("{name} failed: {e}"))
}

#[test]
fn every_tool_has_an_object_schema() {
    for t in tools() {
        assert!(t.input_schema["type"] == "object", "{}", t.name);
        assert!(!t.description.is_empty(), "{}", t.name);
    }
}

#[test]
fn overview_lists_rooms_and_the_rules_that_move_things() {
    let mut ws = Workspace::new(files("lighthouse"));
    let o = call(&mut ws, "overview", json!({}));
    assert!(o["start"] == "shore");
    assert!(o["problems"]["errors"] == 0);
    let rooms = o["rooms"].as_array().unwrap();
    assert!(
        rooms
            .iter()
            .any(|r| r["id"] == "shore" && r["exits"]["north"] == "cottage")
    );
    let things = o["things"].as_array().unwrap();
    assert!(let Some(key) = things.iter().find(|t| t["id"] == "iron_key"));
    assert!(key["location"] == "nowhere");
    assert!(key["moved_by"] == json!(["find_iron_key"]));
}

#[test]
fn neighbors_follow_exits_both_ways() {
    let mut ws = Workspace::new(files("lighthouse"));
    let n = call(&mut ws, "neighbors", json!({ "room": "shore", "hops": 2 }));
    let list = n["neighbors"].as_array().unwrap();
    assert!(let Some(cottage) = list.iter().find(|r| r["id"] == "cottage"));
    assert!(cottage["path"] == json!(["north"]));
    assert!(
        list.iter()
            .any(|r| r["id"] == "lantern_room" && r["hops"] == 2)
    );
}

#[test]
fn the_lighthouse_can_be_won_through_the_tools() {
    let mut ws = Workspace::new(files("lighthouse"));
    let mut last = Value::Null;
    for cmd in [
        "n",
        "open drawer",
        "take key",
        "take lamp",
        "take matches",
        "light lamp",
        "s",
        "unlock iron door with key",
        "e",
        "up",
        "light beacon",
    ] {
        last = call(&mut ws, "input", json!({ "text": cmd }));
    }
    assert!(last["ended"] == true, "{last:#}");
}

#[test]
fn sessions_are_separate() {
    let mut ws = Workspace::new(files("lighthouse"));
    call(&mut ws, "session_start", json!({ "session": "test" }));
    call(&mut ws, "input", json!({ "text": "n", "session": "test" }));
    assert!(call(&mut ws, "scene", json!({ "session": "test" }))["room"] == "cottage");
    assert!(call(&mut ws, "scene", json!({}))["room"] == "shore");
    assert!(let Err(e) = ws.call("scene", json!({ "session": "missing" })));
    assert!(e.0.contains("session_start"));
}

#[test]
fn a_saved_state_starts_a_new_session() {
    let mut ws = Workspace::new(files("lighthouse"));
    call(&mut ws, "input", json!({ "text": "n" }));
    let state = call(&mut ws, "save", json!({}));
    let s = call(
        &mut ws,
        "session_start",
        json!({ "session": "copy", "state": state }),
    );
    assert!(s["scene"]["room"] == "cottage");
}

#[test]
fn a_broken_file_shows_in_check_and_blocks_play() {
    let mut fs = files("lighthouse");
    let shore = fs
        .iter_mut()
        .find(|f| f.path == "rooms/shore.yaml")
        .unwrap();
    shore
        .text
        .push_str("\n  - { direction: down, to: nowhere_room }\n");
    let mut ws = Workspace::new(fs);
    let c = call(&mut ws, "check", json!({}));
    assert!(c["errors"].as_u64().unwrap() > 0);
    assert!(let Err(e) = ws.call("input", json!({ "text": "look" })));
    assert!(e.0.contains("check"));
    // The read tools still work.
    call(&mut ws, "read_file", json!({ "path": "rooms/shore.yaml" }));
}

#[test]
fn a_reload_keeps_the_session_state() {
    let mut ws = Workspace::new(files("lighthouse"));
    call(&mut ws, "input", json!({ "text": "n" }));
    let mut fs = files("lighthouse");
    let cottage = fs
        .iter_mut()
        .find(|f| f.path == "rooms/cottage.yaml")
        .unwrap();
    cottage.text = cottage.text.replace("pipe smoke", "old rope");
    assert!(ws.is_stale(&fs));
    ws.reload(fs);
    let s = call(&mut ws, "scene", json!({}));
    assert!(s["room"] == "cottage");
    let a = call(&mut ws, "assembly", json!({}));
    assert!(a.to_string().contains("old rope"));
}

#[test]
fn draft_text_is_checked() {
    let mut ws = Workspace::new(files("lighthouse"));
    let ok = call(
        &mut ws,
        "try_markup",
        json!({ "text": "{night: dark | bright} water" }),
    );
    assert!(ok["error"].is_null());
    assert!(ok["text"] == "bright water");
    let bad = call(
        &mut ws,
        "try_condition",
        json!({ "condition": "lamp.lit and" }),
    );
    assert!(!bad["error"].is_null());
}

#[test]
fn a_session_survives_a_reload_with_errors() {
    let mut ws = Workspace::new(files("lighthouse"));
    call(&mut ws, "input", json!({ "text": "n" }));
    let mut broken = files("lighthouse");
    let shore = broken
        .iter_mut()
        .find(|f| f.path == "rooms/shore.yaml")
        .unwrap();
    shore
        .text
        .push_str("\n  - { direction: down, to: nowhere_room }\n");
    ws.reload(broken);
    assert!(ws.call("scene", json!({})).is_err());
    ws.reload(files("lighthouse"));
    assert!(call(&mut ws, "scene", json!({}))["room"] == "cottage");
}
