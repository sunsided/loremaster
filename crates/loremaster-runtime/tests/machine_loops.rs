//! Machine steps with `on: next` (P5-02): they run by themselves in the same
//! turn, they loop, and the step limit ends a loop that does not stop.

use loremaster_model::Value;
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, Status, Step, World};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A hall with a lever, and one machine. `extra` goes into story.yaml.
fn files(extra: &str, machine: &str) -> Vec<SourceFile> {
    vec![
        file(
            "story.yaml",
            &format!(
                "title: Loops\nstart: hall\n{extra}vars:\n  - {{ id: total, type: int, initial: 0 }}\n  - {{ id: blow, type: {{ enum: [none, hit, miss] }}, initial: none }}\n"
            ),
        ),
        file(
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall with a lever.\"]\n",
        ),
        file(
            "things/lever.yaml",
            "name: lever\nlocation: { room: hall }\n",
        ),
        file("verbs/pull.yaml", "label: Pull\nwords: [pull]\n"),
        file("machines/m.yaml", machine),
    ]
}

fn errors(f: Vec<SourceFile>) -> Vec<String> {
    check(&load(f).unwrap())
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

fn world(extra: &str, machine: &str) -> Arc<World> {
    let f = files(extra, machine);
    let errors = errors(f.clone());
    assert!(errors.is_empty(), "{errors:?}");
    Arc::new(World::compile(load(f).unwrap().story).unwrap())
}

fn session(extra: &str, machine: &str) -> Session {
    let mut s = Session::new(world(extra, machine));
    s.start();
    s
}

fn state(s: &Session, var: &str) -> Value {
    s.state().facts[var].clone()
}

fn await_id(step: &Step) -> String {
    match &step.status {
        Status::Await { awaiting } => awaiting.id.clone(),
        other => panic!("no await: {other:?}\n{}", step.plain_text()),
    }
}

/// A Luck test: roll, compare, outcome, all in the turn of the command.
const LUCK: &str = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: rolling, instead: true }
  - from: rolling
    on: next
    to: judged
    then:
      - await: { roll: 2d6, into: total }
  - { from: judged, on: next, when: "total <= 12", to: lucky }
  - { from: judged, on: next, to: unlucky }
  - { from: lucky, on: next, to: idle, say: "You are lucky." }
  - { from: unlucky, on: next, to: idle, say: "You are unlucky." }
"#;

#[test]
fn a_luck_test_runs_its_steps_in_one_turn() {
    let mut s = session("", LUCK);
    let step = s.submit(Input::Text("pull lever".into()));
    assert_eq!(step.status, Status::Ready, "{}", step.plain_text());
    assert!(
        step.plain_text().contains("You are lucky."),
        "{}",
        step.plain_text()
    );
    assert_eq!(state(&s, "m"), Value::Text("idle".into()));
}

/// A counter loop: the step goes back to its own state until `when` fails.
const COUNTER: &str = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: counting, instead: true }
  - { from: counting, on: next, when: "total < 5", to: counting, then: [{ add: { total: 1 } }] }
  - { from: counting, on: next, to: done, say: "Done at {total}." }
"#;

#[test]
fn a_loop_with_a_counter_ends_by_its_condition() {
    let mut s = session("", COUNTER);
    let step = s.submit(Input::Text("pull lever".into()));
    assert!(
        step.plain_text().contains("Done at 5."),
        "{}",
        step.plain_text()
    );
    assert_eq!(state(&s, "total"), Value::Int(5));
    assert_eq!(state(&s, "m"), Value::Text("done".into()));
}

/// A loop whose condition always holds.
const ENDLESS: &str = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: counting, instead: true }
  - { from: counting, on: next, when: "total >= 0", to: counting, then: [{ add: { total: 1 } }] }
"#;

#[test]
fn the_step_limit_ends_a_loop_that_does_not_stop() {
    let mut s = session("step_limit: 10\n", ENDLESS);
    let step = s.submit(Input::Text("pull lever".into()));
    assert_eq!(step.status, Status::Ready);
    assert_eq!(state(&s, "total"), Value::Int(10));
    assert!(
        step.plain_text()
            .contains("stopped after 10 automatic steps"),
        "{}",
        step.plain_text()
    );
    // The default limit is 100.
    let mut s = session("", ENDLESS);
    s.submit(Input::Text("pull lever".into()));
    assert_eq!(state(&s, "total"), Value::Int(100));
    // The explorer ends too.
    let found = explore(world("step_limit: 10\n", ENDLESS), Budget::default());
    assert!(found.never_fired.is_empty(), "{:?}", found.never_fired);
}

#[test]
fn the_checks_reject_a_loop_that_can_never_stop() {
    let bad = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: a }
  - { from: a, on: next, to: b }
  - { from: b, on: next, to: a }
"#;
    let found = errors(files("", bad));
    assert!(
        found.iter().any(|m| m.contains("loop forever")),
        "{found:?}"
    );
    let rule = "- id: r\n  on: next\n  then: []\n";
    let mut f = files("", COUNTER);
    f.push(file("rules/r.yaml", rule));
    let found = errors(f);
    assert!(
        found.iter().any(|m| m.contains("only in a machine step")),
        "{found:?}"
    );
}

/// Combat: each round waits for the player's choice, then loops.
const COMBAT: &str = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: round, instead: true }
  - from: round
    on: next
    when: "total < 3"
    to: round
    then:
      - await: { choice: [hit, miss], into: blow }
      - add: { total: 1 }
      - say: "Round {total}: {blow}."
  - { from: round, on: next, to: over, say: "The fight is over." }
"#;

#[test]
fn a_save_in_the_middle_of_a_loop_restores() {
    let mut s = session("", COMBAT);
    let mut id = await_id(&s.submit(Input::Text("pull lever".into())));
    for round in 1..=3 {
        // Save and restore at each await.
        let save = serde_json::to_value(s.save()).unwrap();
        let (restored, _) = Session::load(world("", COMBAT), save).unwrap();
        s = restored;
        let step = s.resume(&id, Value::Text("hit".into())).unwrap();
        let text = step.plain_text();
        assert!(text.contains(&format!("Round {round}: hit.")), "{text}");
        if round < 3 {
            id = await_id(&step);
        } else {
            assert!(text.contains("The fight is over."), "{text}");
            assert_eq!(step.status, Status::Ready);
        }
    }
    assert_eq!(state(&s, "m"), Value::Text("over".into()));
}

#[test]
fn the_start_state_runs_its_steps_when_the_game_starts() {
    let machine = "start: a\nsteps:\n  - { from: a, on: next, to: b }\n";
    let s = session("", machine);
    assert_eq!(state(&s, "m"), Value::Text("b".into()));
}

/// Machines that start each other share the limit of the turn.
const TWO: &str = r#"
start: idle
steps:
  - { from: idle, on: { verb: [pull, lever] }, to: spin, instead: true }
  - { from: spin, on: next, when: "total >= 0", to: spin, then: [{ add: { total: 1 } }, { emit: tick }] }
"#;

#[test]
fn nested_machines_share_the_step_limit() {
    let inner = "start: spin\nsteps:\n  - { from: spin, on: { event: tick }, to: spin }\n  - { from: spin, on: next, when: \"total >= 0\", to: spin, then: [{ add: { total: 1 } }, { emit: tick }] }\n";
    let mut f = files("step_limit: 7\n", TWO);
    f.push(file("machines/n.yaml", inner));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    // `n` starts in `spin`, so its chain already used the limit of the first turn.
    assert_eq!(state(&s, "total"), Value::Int(7));
    let step = s.submit(Input::Text("pull lever".into()));
    assert_eq!(step.status, Status::Ready);
    let notes = step.plain_text().matches("step_limit").count();
    assert_eq!(notes, 1, "{}", step.plain_text());
    // Each automatic step adds 1: with a limit per machine, it would be more.
    assert_eq!(state(&s, "total"), Value::Int(14));
}

#[test]
fn an_await_at_the_start_does_not_hold_back_other_machines() {
    let waits = "start: a\nsteps:\n  - { from: a, on: next, to: b, then: [{ await: { choice: [left, right] } }] }\n";
    let mut f = files("", waits);
    f.push(file(
        "machines/n.yaml",
        "start: a\nsteps:\n  - { from: a, on: next, to: b }\n",
    ));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    assert_eq!(state(&s, "n"), Value::Text("b".into()));
}

#[test]
fn a_loop_no_step_can_reach_is_no_error() {
    let m = "start: a\nstates: [a, g, h]\nsteps:\n  - { from: g, on: next, to: h }\n  - { from: h, on: next, to: g }\n";
    let found = errors(files("", m));
    assert!(
        !found.iter().any(|m| m.contains("loop forever")),
        "{found:?}"
    );
}
