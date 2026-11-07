//! The Session API (ADR 0003): input down, effects as data up.

use loremaster_model::{EndingKind, Id};
use loremaster_project::{load, read_dir};
use loremaster_runtime::{Command, Effect, Input, Session, Status, Step, World};
use std::path::Path;
use std::sync::Arc;

fn world(example: &str) -> Arc<World> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(example);
    Arc::new(World::compile(load(read_dir(&dir).unwrap()).unwrap().story).unwrap())
}

fn text(s: &str) -> Input {
    Input::Text(s.into())
}

const LIGHTHOUSE_WIN: &[&str] = &[
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
];

#[test]
fn a_session_plays_the_lighthouse_to_its_ending() {
    let mut s = Session::new(world("lighthouse"));
    let opening = s.start();
    assert_eq!(opening.status, Status::Ready);
    assert!(
        opening.plain_text().contains("ferry"),
        "{}",
        opening.plain_text()
    );
    for command in LIGHTHOUSE_WIN {
        let step = s.submit(text(command));
        assert_eq!(
            step.status,
            Status::Ready,
            "{command}: {}",
            step.plain_text()
        );
    }
    // A command record does the same as typed text.
    let last = s.submit(Input::Command(Command::Verb {
        verb: Id::new("turn_on"),
        objects: vec![Id::new("lens")],
    }));
    let Status::Ended {
        ending: Some(ending),
    } = &last.status
    else {
        panic!("not ended: {last:?}");
    };
    assert_eq!(ending.id.as_str(), "beacon_lit");
    assert_eq!(ending.kind, EndingKind::Win);
    assert_eq!(ending.rank, Some(1));
    assert!(matches!(last.effects.last(), Some(Effect::Ending { .. })));
    assert!(last.plain_text().contains("The beacon is lit"));
}

#[test]
fn state_changes_come_as_notices_and_rule_lines_do_not() {
    let mut s = Session::new(world("lighthouse"));
    s.start();
    s.submit(text("n"));
    let step = s.submit(text("open drawer"));
    let notices: Vec<_> = step
        .effects
        .iter()
        .filter_map(|e| match e {
            Effect::Notice { entry } => Some(format!("{} {}", entry.kind, entry.text)),
            _ => None,
        })
        .collect();
    assert!(
        notices.iter().any(|n| n.contains("drawer_searched")),
        "{notices:?}"
    );
    assert!(
        !notices.iter().any(|n| n.starts_with("rule ")),
        "{notices:?}"
    );
}

#[test]
fn a_talk_offers_its_choices_as_an_effect() {
    let mut s = Session::new(world("cantina"));
    s.start();
    s.submit(text("in"));
    let step = s.submit(text("talk to figure"));
    let Some(Effect::Choices { talk, choices }) = step
        .effects
        .iter()
        .find(|e| matches!(e, Effect::Choices { .. }))
    else {
        panic!("no choices: {step:?}");
    };
    assert_eq!(talk.with, "blackbeard");
    assert!(choices.len() >= 2, "{choices:?}");
    let answer = s.choose(0);
    assert!(
        answer.plain_text().contains("Blackbeard"),
        "{}",
        answer.plain_text()
    );
}

#[test]
fn the_same_seed_and_input_give_the_same_effects() {
    let play = |seed: u64| -> Vec<Step> {
        let mut s = Session::with_seed(world("zork1"), seed);
        let mut steps = vec![s.start()];
        for command in [
            "n",
            "e",
            "open window",
            "enter",
            "w",
            "take lamp",
            "wait",
            "wait",
        ] {
            steps.push(s.submit(text(command)));
        }
        steps
    };
    assert_eq!(play(42), play(42));
}

#[test]
fn a_tick_lets_time_pass() {
    let mut s = Session::new(world("lighthouse"));
    s.start();
    let before = s.state().turns;
    let step = s.submit(Input::Tick);
    assert_eq!(step.status, Status::Ready);
    assert_eq!(s.state().turns, before + 1);
}

#[test]
fn steps_and_inputs_are_plain_data() {
    let mut s = Session::new(world("lighthouse"));
    let step = s.start();
    let json = serde_json::to_value(&step).unwrap();
    assert_eq!(json["status"]["type"], "ready");
    assert_eq!(json["effects"][0]["type"], "text");
    let back: Step = serde_json::from_value(json).unwrap();
    assert_eq!(back, step);
    let input: Input =
        serde_json::from_value(serde_json::json!({ "type": "text", "value": "n" })).unwrap();
    assert_eq!(input, Input::Text("n".into()));
}

#[test]
fn a_session_restores_from_a_save() {
    let w = world("lighthouse");
    let mut s = Session::new(w.clone());
    s.start();
    s.submit(text("n"));
    let save = serde_json::to_value(s.save()).unwrap();
    let (restored, loaded) = Session::load(w, save).unwrap();
    assert!(!loaded.story_changed);
    assert_eq!(restored.state(), s.state());
}
