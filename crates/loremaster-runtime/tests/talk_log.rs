//! The turn log of a talk: each change that a dialogue makes gets one line,
//! with its new value, in the order of the changes.

use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::authoring;
use loremaster_runtime::{Input, Session, Turn, World};
use std::path::Path;
use std::sync::Arc;

/// One typed command as a whole turn.
fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

fn cantina(dialogue: impl Fn(String) -> String) -> Session {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let mut files: Vec<SourceFile> = read_dir(&dir).unwrap();
    for f in &mut files {
        if f.path == "dialogues/blackbeard.yaml" {
            f.text = dialogue(std::mem::take(&mut f.text));
        }
    }
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    input(&mut game, "in");
    input(&mut game, "talk to figure");
    game
}

/// The log lines of a turn, without the rule lines.
fn changes(game: &mut Session, text: &str) -> Vec<String> {
    input(game, text)
        .log
        .iter()
        .filter(|e| e.kind != "rule")
        .map(|e| format!("{} {}", e.kind, e.text))
        .collect()
}

#[test]
fn a_dialogue_logs_each_change_with_its_new_value() {
    let mut game = cantina(|t| t);
    assert_eq!(
        changes(&mut game, "1"),
        [
            "set knows_blackbeard_name = true",
            "relation blackbeard.trust +1 = 1",
        ]
    );
}

#[test]
fn a_zero_delta_changes_nothing_and_logs_nothing() {
    let mut game = cantina(|t| t.replace("relation: { trust: 1 }", "relation: { trust: 0 }"));
    let lines = changes(&mut game, "1");
    assert_eq!(lines, ["set knows_blackbeard_name = true"], "{lines:?}");
    assert!(!game.state().relations.contains_key("blackbeard.trust"));
}

#[test]
fn go_back_shows_the_node_again_without_its_outcomes() {
    let mut game = cantina(|t| {
        t.replace(
            "relation: { trust: -1 }\n        go: end",
            "relation: { trust: -1 }\n        go: back",
        )
    });
    // meet -> marigold (remembers asked_marigold) -> names.
    assert_eq!(
        changes(&mut game, "2"),
        ["memory blackbeard remembers asked_marigold"]
    );
    input(&mut game, "who");
    let talk = game.state().talk.clone().expect("in a talk");
    assert_eq!((talk.node.as_str(), talk.back.len()), ("names", 2));

    // Back to marigold: its line shows again, its `remember` does not apply again.
    let turn = input(&mut game, "just");
    let lines: Vec<String> = turn
        .log
        .iter()
        .filter(|e| e.kind != "rule")
        .map(|e| format!("{} {}", e.kind, e.text))
        .collect();
    assert_eq!(lines, ["relation blackbeard.trust -1 = 0"]);
    let text = format!("{:?}", turn.blocks);
    assert!(text.contains("Ships come and go"), "{text}");
    let talk = game.state().talk.clone().expect("still in the talk");
    assert_eq!(
        (talk.node.as_str(), talk.back.clone()),
        ("marigold", vec!["meet".to_string()])
    );
}

#[test]
fn a_go_to_a_node_whose_when_does_not_hold_ends_the_talk() {
    let mut game = cantina(|t| {
        t.replace(
            "  marigold:\n",
            "  marigold:\n    when: knows_blackbeard_name\n",
        )
    });
    let turn = input(&mut game, "2");
    assert!(game.state().talk.is_none());
    // The node is not entered: no line, no `remember`.
    let text = format!("{:?}", turn.blocks);
    assert!(!text.contains("Ships come and go"), "{text}");
    assert!(
        !turn.log.iter().any(|e| e.kind == "memory"),
        "{:?}",
        turn.log
    );
}
