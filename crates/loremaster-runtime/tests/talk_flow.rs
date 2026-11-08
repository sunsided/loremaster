//! The flow of a talk (#155, dramaturge R2): a node's `lines` play in order,
//! each by its speaker, and a node without a choice goes on to its `next`
//! node in the same turn, with the outcomes of each node on the way.

use loremaster_project::{Project, SourceFile, check, load};
use loremaster_runtime::authoring;
use loremaster_runtime::{BlockKind, Input, Session, Turn, World};
use std::sync::Arc;

const STORY: &str = r#"
title: Flow
start: kitchen
vars:
  - { id: late, type: bool, initial: false }
  - { id: greeted, type: bool, initial: false }
  - { id: told, type: int, initial: 0 }
"#;

const KITCHEN: &str = r#"
name: Kitchen
description: ["A warm kitchen."]
"#;

const KEEPER: &str = r#"
name: lock-keeper
aliases: [keeper]
location: { room: kitchen }
"#;

const DOG: &str = r#"
name: dog
location: { room: kitchen }
rules:
  - id: howl
    on: { verb: [push, dog] }
    instead: true
    then:
      - set: { late: true }
      - say: "The dog howls."
"#;

const DIALOGUE: &str = r#"
with: keeper
start: [hi]
nodes:
  hi:
    say: [{ says: "Evening." }]
    lines:
      - { speaker: player, say: [{ says: "Evening." }] }
      - { speaker: dog, say: [{ says: "Woof." }] }
      - { speaker: keeper, say: [{ says: "Late, then.", when: late }] }
    set: { greeted: true }
    next: story
  story:
    say: "He tells a story."
    add: { told: 1 }
    next: ask
  ask:
    say: [{ says: "Anything else?" }]
    add: { told: 1 }
    options:
      - { say: "Bye.", go: end }
"#;

/// Two nodes that go on to each other: a loop without a choice.
const LOOP: &str = r#"
  spin:
    say: "Round."
    next: spun
  spun:
    say: "And round."
    next: spin
"#;

fn game() -> Session {
    let project = project(DIALOGUE);
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    play(project)
}

/// The story with this dialogue of the keeper.
fn project(dialogue: &str) -> Project {
    let files = [
        ("story.yaml", STORY),
        ("rooms/kitchen.yaml", KITCHEN),
        ("things/keeper.yaml", KEEPER),
        ("things/dog.yaml", DOG),
        ("dialogues/keeper.yaml", dialogue),
    ]
    .iter()
    .map(|(path, text)| SourceFile {
        path: (*path).into(),
        text: (*text).into(),
    })
    .collect();
    load(files).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn play(project: Project) -> Session {
    let mut game = Session::new(Arc::new(World::compile(project.story).unwrap()));
    authoring::start(&mut game);
    game
}

fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

/// Each block of a turn: its kind, its text and the speakers in it.
fn blocks(turn: &Turn) -> Vec<(BlockKind, String, Vec<String>)> {
    turn.blocks
        .iter()
        .filter(|b| b.kind != BlockKind::Command)
        .map(|b| {
            let text = b.runs.iter().map(|r| r.text.as_str()).collect();
            let mut who: Vec<String> = b.runs.iter().filter_map(|r| r.speaker.clone()).collect();
            who.dedup();
            (b.kind, text, who)
        })
        .collect()
}

fn fact(game: &Session, name: &str) -> String {
    game.state()
        .facts
        .get(name)
        .map(ToString::to_string)
        .unwrap_or_default()
}

#[test]
fn lines_play_in_order_by_their_speakers_and_next_goes_on_in_the_same_turn() {
    let mut game = game();
    let talk = input(&mut game, "talk to keeper");
    let who = |s: &str| vec![s.to_string()];
    assert_eq!(
        blocks(&talk),
        [
            (BlockKind::Speech, "\"Evening.\"".into(), who("keeper")),
            (BlockKind::Said, "\"Evening.\"".into(), who("player")),
            (BlockKind::Speech, "\"Woof.\"".into(), who("dog")),
            // The keeper's last line has `when: late`: it does not show.
            (BlockKind::Speech, "He tells a story.".into(), vec![]),
            (
                BlockKind::Speech,
                "\"Anything else?\"".into(),
                who("keeper")
            ),
        ]
    );
    // The talk stops at the node with a choice, after the outcomes of each
    // node on the way.
    assert_eq!(game.state().talk.as_ref().unwrap().node, "ask");
    assert_eq!(fact(&game, "greeted"), "true");
    assert_eq!(fact(&game, "told"), "2");
    let choices: Vec<&str> = talk.choices.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(choices, ["Bye."]);
}

#[test]
fn a_line_with_when_shows_while_it_holds() {
    let mut game = game();
    input(&mut game, "push dog");
    let talk = input(&mut game, "talk to keeper");
    let speech: Vec<String> = blocks(&talk).into_iter().map(|b| b.1).collect();
    assert_eq!(speech[3], "\"Late, then.\"", "{speech:#?}");
}

#[test]
fn a_loop_of_next_is_a_check_error_and_stops_in_the_game() {
    let dialogue = DIALOGUE.replace("Bye.\", go: end", "Again.\", go: spin") + LOOP;
    let project = project(&dialogue);
    // dramaturge's validator finds the loop ...
    let problems: Vec<String> = check(&project).into_iter().map(|d| d.at).collect();
    assert_eq!(problems, ["nodes.spin", "nodes.spun"]);
    // ... and should it play anyway, the talk stops.
    let mut game = play(project);
    input(&mut game, "talk to keeper");
    let turn = input(&mut game, "1");
    assert!(game.state().talk.is_none(), "the loop ends the talk");
    assert!(
        turn.log
            .iter()
            .any(|e| e.kind == "author" && e.text.contains("without a choice")),
        "{:#?}",
        turn.log
    );
    // It did go round: each node of the loop showed its line.
    let text = turn.plain_text();
    assert!(
        text.contains("Round.") && text.contains("And round."),
        "{text}"
    );
}
