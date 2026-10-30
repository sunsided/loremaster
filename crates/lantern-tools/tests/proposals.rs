use assert2::assert;
use lantern_project::{SourceFile, read_dir};
use lantern_tools::Workspace;
use serde_json::{Value, json};
use std::path::Path;

fn files() -> Vec<SourceFile> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let mut files = read_dir(&root).unwrap();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

fn text(path: &str) -> String {
    files().into_iter().find(|f| f.path == path).unwrap().text
}

fn call(ws: &mut Workspace, name: &str, args: Value) -> Value {
    ws.call(name, args)
        .unwrap_or_else(|e| panic!("{name} failed: {e}"))
}

const PIER: &str = "name: The Pier\nposition: [0, 1]\ndescription:\n  - \"Tarred planks run out over the water.\"\nexits:\n  - { direction: up, to: shore }\n";

/// The Shore with an exit down to the pier.
fn shore_with_pier() -> String {
    text("rooms/shore.yaml").replace("exits:\n", "exits:\n  - { direction: down, to: pier }\n")
}

#[test]
fn a_new_room_is_checked_and_previewed() {
    let mut ws = Workspace::new(files());
    let v = call(
        &mut ws,
        "propose",
        json!({
            "summary": "Down from the shore to a pier",
            "files": [
                { "path": "rooms/pier.yaml", "text": PIER },
                { "path": "rooms/shore.yaml", "text": shore_with_pier() }
            ]
        }),
    );
    assert!(v["ok"] == true, "{v:#}");
    assert!(v["id"] == "p1");
    let changes = v["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["kind"] == "added"
        && c["entity"] == "room:pier"
        && c["file"] == "rooms/pier.yaml"));
    assert!(
        changes
            .iter()
            .any(|c| c["kind"] == "field" && c["entity"] == "room:shore" && c["field"] == "exits")
    );
    let rooms = v["rooms"].as_array().unwrap();
    assert!(rooms.iter().any(|r| {
        r["room"] == "pier"
            && r["texts"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Tarred planks")
    }));
    // The project itself did not change.
    assert!(
        call(&mut ws, "overview", json!({}))["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["id"] != "pier")
    );
}

#[test]
fn a_proposal_can_be_played_before_it_is_accepted() {
    let mut ws = Workspace::new(files());
    call(
        &mut ws,
        "propose",
        json!({
            "summary": "pier",
            "files": [
                { "path": "rooms/pier.yaml", "text": PIER },
                { "path": "rooms/shore.yaml", "text": shore_with_pier() }
            ]
        }),
    );
    call(
        &mut ws,
        "session_start",
        json!({ "session": "draft", "proposal": "p1" }),
    );
    let turn = call(
        &mut ws,
        "input",
        json!({ "text": "down", "session": "draft" }),
    );
    assert!(turn["scene"]["room"] == "pier", "{turn:#}");
    let main = call(&mut ws, "input", json!({ "text": "down" }));
    assert!(main["text"].as_str().unwrap().contains("can't go"));
}

#[test]
fn a_changed_fragment_is_one_change() {
    let mut ws = Workspace::new(files());
    let shore = text("rooms/shore.yaml").replace(
        "Shingle crunches underfoot.",
        "Wet shingle crunches underfoot.",
    );
    let v = call(
        &mut ws,
        "propose",
        json!({ "summary": "wet", "files": [{ "path": "rooms/shore.yaml", "text": shore }] }),
    );
    let changes = v["changes"].as_array().unwrap();
    assert!(changes.len() == 1, "{changes:#?}");
    let c = &changes[0];
    assert!(c["kind"] == "fragment");
    assert!(c["field"] == json!(["description"]));
    assert!(c["owner"] == "room:shore");
    assert!(
        c["after_fragment"]["variants"][0]
            .as_str()
            .unwrap()
            .starts_with("Wet shingle")
    );
    assert!(c["after_fragment"]["layer"] == "base");
    assert!(c["before_index"] == 0 && c["after_index"] == 0);
    assert!(c["after"].to_string().contains("Wet shingle"));
}

#[test]
fn an_added_fragment_has_no_before() {
    let mut ws = Workspace::new(files());
    let shore =
        text("rooms/shore.yaml").replace("exits:\n", "  - \"Gulls argue over a crab.\"\nexits:\n");
    let v = call(
        &mut ws,
        "propose",
        json!({ "summary": "gulls", "files": [{ "path": "rooms/shore.yaml", "text": shore }] }),
    );
    let changes = v["changes"].as_array().unwrap();
    assert!(changes.len() == 1, "{changes:#?}");
    assert!(changes[0]["before"].is_null());
    assert!(changes[0]["after_index"] == 4);
}

#[test]
fn a_broken_proposal_reports_the_problems_it_adds() {
    let mut ws = Workspace::new(files());
    let v = call(
        &mut ws,
        "propose",
        json!({ "summary": "dangling exit", "files": [{ "path": "rooms/shore.yaml", "text": shore_with_pier() }] }),
    );
    assert!(v["ok"] == false);
    assert!(!v["new_problems"].as_array().unwrap().is_empty());
    assert!(
        ws.call("session_start", json!({ "session": "x", "proposal": "p1" }))
            .is_err()
    );
}

#[test]
fn only_project_files_can_be_proposed() {
    let mut ws = Workspace::new(files());
    for path in [
        "../evil.yaml",
        "rooms/../story.yaml",
        "src/main.rs",
        "rooms/a/b.yaml",
        "/etc/x.yaml",
    ] {
        let r = ws.call(
            "propose",
            json!({ "summary": "x", "files": [{ "path": path, "text": "" }] }),
        );
        assert!(r.is_err(), "{path}");
    }
    assert!(ws.call("propose", json!({ "summary": "nothing" })).is_err());
}

#[test]
fn proposals_are_listed_and_withdrawn() {
    let mut ws = Workspace::new(files());
    let shore = text("rooms/shore.yaml").replace("Shingle", "Grey shingle");
    call(
        &mut ws,
        "propose",
        json!({ "summary": "a", "files": [{ "path": "rooms/shore.yaml", "text": shore.clone() }] }),
    );
    call(
        &mut ws,
        "propose",
        json!({ "summary": "b", "files": [{ "path": "rooms/shore.yaml", "text": shore }] }),
    );
    let list = call(&mut ws, "list_proposals", json!({}));
    assert!(list.as_array().unwrap().len() == 2);
    call(&mut ws, "withdraw_proposal", json!({ "id": "p1" }));
    let list = call(&mut ws, "list_proposals", json!({}));
    assert!(list[0]["id"] == "p2");
    assert!(ws.call("get_proposal", json!({ "id": "p1" })).is_err());
}

#[test]
fn applying_gives_the_new_files() {
    let mut ws = Workspace::new(files());
    let id = ws
        .add_proposal(
            "pier".into(),
            vec![SourceFile {
                path: "rooms/pier.yaml".into(),
                text: PIER.into(),
            }],
            vec![],
        )
        .unwrap();
    let applied = ws.apply_proposal(&id).unwrap();
    assert!(applied.iter().any(|f| f.path == "rooms/pier.yaml"));
    assert!(applied.len() == files().len() + 1);
}

#[test]
fn frame_fragments_have_the_runtime_owner() {
    let mut ws = Workspace::new(files());
    let story = text("story.yaml").replace(
        "text: \"You look around.\"",
        "text: \"You take in the view.\"",
    );
    let v = call(
        &mut ws,
        "propose",
        json!({ "summary": "frame", "files": [{ "path": "story.yaml", "text": story }] }),
    );
    let changes = v["changes"].as_array().unwrap();
    assert!(changes.len() == 1, "{changes:#?}");
    assert!(changes[0]["owner"] == "story/frames/look/before");
    assert!(changes[0]["field"] == json!(["frames", "look", "before"]));
    assert!(changes[0]["after_fragment"]["when"] == "action.repeat == 1");
}

#[test]
fn files_carry_both_texts_and_a_base_hash() {
    let mut ws = Workspace::new(files());
    let old = text("rooms/shore.yaml");
    let new = old.replace("Shingle", "Grey shingle");
    let v = call(
        &mut ws,
        "propose",
        json!({
            "summary": "grey",
            "files": [{ "path": "rooms/shore.yaml", "text": new }, { "path": "rooms/pier.yaml", "text": PIER }],
            "delete": ["rooms/pier.yaml"]
        }),
    );
    let f = &v["files"][0];
    assert!(f["path"] == "rooms/shore.yaml");
    assert!(f["before"] == old.as_str());
    assert!(f["after"] == new.as_str());
    assert!(f["base_sha"] == lantern_tools::sha256(&old).as_str());
    assert!(v["files"][1]["base_sha"].is_null());
    assert!(v["files"][2]["after"].is_null());
    assert!(v["stale"] == false);
}

#[test]
fn a_proposal_goes_stale_when_its_file_changes() {
    let mut ws = Workspace::new(files());
    let new = text("rooms/shore.yaml").replace("Shingle", "Grey shingle");
    call(
        &mut ws,
        "propose",
        json!({ "summary": "grey", "files": [{ "path": "rooms/shore.yaml", "text": new }] }),
    );
    let mut changed = files();
    let shore = changed
        .iter_mut()
        .find(|f| f.path == "rooms/shore.yaml")
        .unwrap();
    shore.text = shore.text.replace("crunches", "grinds");
    ws.reload(changed);
    assert!(call(&mut ws, "get_proposal", json!({ "id": "p1" }))["stale"] == true);
}
