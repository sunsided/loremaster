//! Awaits: a rule stops, the session asks the host, and the rule goes on with
//! the answer (SE-5, SE-6, SE-7, XP-3).

use loremaster_model::Value;
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{AwaitRequest, Command, Input, Session, Status, Step, World};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A hall with a lever: pulling it asks which way, then says it.
fn files(rule: &str) -> Vec<SourceFile> {
    vec![
        file(
            "story.yaml",
            "title: Lever\nstart: hall\nvars:\n  - { id: path, type: { enum: [none, left, right] }, initial: none }\n  - { id: total, type: int, initial: 0 }\n",
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
        file("rules/lever.yaml", rule),
    ]
}

const CHOICE: &str = r#"
- id: pull_lever
  on: { verb: [pull, lever] }
  instead: true
  then:
    - say: "A voice asks: left or right?"
    - await: { choice: [left, { id: right, text: "the right door" }], into: path }
    - say: "You go {path}."
"#;

fn session(rule: &str) -> Session {
    let world = Arc::new(World::compile(load(files(rule)).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s
}

fn awaiting(step: &Step) -> (String, AwaitRequest) {
    match &step.status {
        Status::Await { awaiting } => (awaiting.id.clone(), awaiting.request.clone()),
        other => panic!("no await: {other:?}\n{}", step.plain_text()),
    }
}

#[test]
fn a_rule_waits_for_a_choice_and_goes_on_with_the_answer() {
    let mut s = session(CHOICE);
    let step = s.submit(Input::Text("pull lever".into()));
    assert!(step.plain_text().contains("left or right"));
    assert!(
        !step.plain_text().contains("You go"),
        "the rest of the rule waits"
    );
    let (id, request) = awaiting(&step);
    let AwaitRequest::Choice { options } = request else {
        panic!("{request:?}")
    };
    assert_eq!(options[1].text, "the right door");
    let turns = s.state().turns;
    let done = s.resume(&id, Value::Text("right".into())).unwrap();
    assert!(
        done.plain_text().contains("You go right."),
        "{}",
        done.plain_text()
    );
    assert_eq!(done.status, Status::Ready);
    assert_eq!(
        s.state().facts.get("path"),
        Some(&Value::Text("right".into()))
    );
    assert_eq!(s.state().turns, turns, "a resume is not a new turn");
}

#[test]
fn a_wrong_answer_is_refused_and_other_input_waits() {
    let mut s = session(CHOICE);
    let (id, _) = awaiting(&s.submit(Input::Text("pull lever".into())));
    assert!(s.resume(&id, Value::Text("up".into())).is_err());
    assert!(s.resume("other", Value::Text("left".into())).is_err());
    let step = s.submit(Input::Command(Command::Go {
        direction: "north".into(),
    }));
    assert!(
        step.plain_text().contains("The story waits"),
        "{}",
        step.plain_text()
    );
    assert!(matches!(step.status, Status::Await { .. }));
}

#[test]
fn a_text_host_answers_by_number_or_by_name() {
    let mut s = session(CHOICE);
    s.submit(Input::Text("pull lever".into()));
    let step = s.submit(Input::Text("1".into()));
    assert!(
        step.plain_text().contains("You go left."),
        "{}",
        step.plain_text()
    );

    let mut s = session(CHOICE);
    s.submit(Input::Text("pull lever".into()));
    let step = s.submit(Input::Text("the right door".into()));
    assert!(
        step.plain_text().contains("You go right."),
        "{}",
        step.plain_text()
    );
}

#[test]
fn a_save_in_the_middle_of_a_rule_restores() {
    let mut s = session(CHOICE);
    let (id, _) = awaiting(&s.submit(Input::Text("pull lever".into())));
    let save = serde_json::to_value(s.save()).unwrap();
    let world = Arc::new(World::compile(load(files(CHOICE)).unwrap().story).unwrap());
    let (mut restored, _) = Session::load(world, save).unwrap();
    let done = restored.resume(&id, Value::Text("left".into())).unwrap();
    assert!(
        done.plain_text().contains("You go left."),
        "{}",
        done.plain_text()
    );
}

const ROLL: &str = r#"
- id: pull_lever
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { roll: 2d6, into: total }
    - say: "You rolled {total}."
"#;

#[test]
fn the_session_answers_a_roll_unless_the_host_asks_to() {
    let mut s = session(ROLL);
    let step = s.submit(Input::Text("pull lever".into()));
    assert_eq!(step.status, Status::Ready, "{}", step.plain_text());
    let Some(Value::Int(total)) = s.state().facts.get("total").cloned() else {
        panic!("no total")
    };
    assert!((2..=12).contains(&total));
    assert!(step.plain_text().contains(&format!("You rolled {total}.")));

    let mut s = session(ROLL);
    s.set_host_rolls(true);
    let (id, request) = awaiting(&s.submit(Input::Text("pull lever".into())));
    assert_eq!(
        request,
        AwaitRequest::Roll {
            dice: "2d6".into(),
            min: 2,
            max: 12
        }
    );
    assert!(s.resume(&id, Value::Int(13)).is_err());
    let done = s.resume(&id, Value::Int(7)).unwrap();
    assert!(
        done.plain_text().contains("You rolled 7."),
        "{}",
        done.plain_text()
    );
}

#[test]
fn the_explorer_tries_each_answer() {
    let world = Arc::new(World::compile(load(files(CHOICE)).unwrap().story).unwrap());
    // The authoring views are in loremaster-workbench; this test reaches the
    // explorer through the primitive behind `Workbench::explore`.
    let found = explore(world, Budget::default());
    assert!(found.never_fired.is_empty(), "{:?}", found.never_fired);
    assert!(found.states >= 3, "{}", found.states);
}

#[test]
fn the_checks_reject_await_outside_rules_and_wrong_shapes() {
    let messages = |extra: Vec<SourceFile>| -> Vec<String> {
        let mut f = files(CHOICE);
        f.extend(extra);
        check(&load(f).unwrap())
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message)
            .collect()
    };
    let timer = messages(vec![file(
        "timers/clock.yaml",
        "turns: 3\nrunning: true\nat:\n  0: [{ await: { choice: [a, b] } }]\n",
    )]);
    assert!(
        timer.iter().any(|m| m.contains("only in rules")),
        "{timer:?}"
    );

    let two = messages(vec![file(
        "rules/two.yaml",
        "- id: two\n  on: { verb: [pull, lever] }\n  then:\n    - await: { choice: [a], roll: 1d6 }\n",
    )]);
    assert!(two.iter().any(|m| m.contains("exactly one")), "{two:?}");

    let wrong = messages(vec![file(
        "rules/wrong.yaml",
        "- id: wrong\n  on: { verb: [pull, lever] }\n  then:\n    - await: { roll: 1d6, into: path }\n",
    )]);
    assert!(
        wrong.iter().any(|m| m.contains("number variable")),
        "{wrong:?}"
    );

    let walk = messages(vec![file(
        "rules/walk.yaml",
        "- id: walk\n  on: { verb: [pull, lever] }\n  then:\n    - await: { walk_to: hall, into: path }\n",
    )]);
    assert!(
        walk.iter()
            .any(|m| m.contains("cannot hold the answer `arrived`")),
        "{walk:?}"
    );
}

#[test]
fn an_action_without_results_takes_any_answer_and_time_waits() {
    let rule = r#"
- id: pull_lever
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { action: open_sluice, into: total }
    - say: "The sluice opened {total} times."
"#;
    let mut s = session(rule);
    let (id, _) = awaiting(&s.submit(Input::Text("pull lever".into())));
    let turns = s.state().turns;
    let tick = s.submit(Input::Tick);
    assert!(
        tick.plain_text().contains("The story waits"),
        "{}",
        tick.plain_text()
    );
    assert_eq!(s.state().turns, turns, "time waits with the rule");
    let done = s.resume(&id, Value::Int(3)).unwrap();
    assert!(
        done.plain_text().contains("opened 3 times"),
        "{}",
        done.plain_text()
    );
}

#[test]
fn a_second_await_skips_the_rest_of_its_rule() {
    let rules = r#"
- id: pull_lever
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { choice: [left, right], into: path }
    - say: "You go {path}."
- id: also
  on: { verb: [pull, lever] }
  then:
    - await: { choice: [left, right], into: path }
    - say: "This line needs an answer."
"#;
    let mut s = session(rules);
    let step = s.submit(Input::Text("pull lever".into()));
    assert!(matches!(step.status, Status::Await { .. }));
    assert!(
        !step.plain_text().contains("needs an answer"),
        "{}",
        step.plain_text()
    );
}

#[test]
fn an_answer_must_fit_the_variable() {
    let rule = r#"
- id: pull_lever
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { action: open_sluice, into: total }
"#;
    let mut s = session(rule);
    let (id, _) = awaiting(&s.submit(Input::Text("pull lever".into())));
    assert!(s.resume(&id, Value::Text("many".into())).is_err());
    assert!(s.resume(&id, Value::Int(2)).is_ok());
}

#[test]
fn a_changed_story_drops_the_waiting_rule() {
    let mut s = session(CHOICE);
    s.submit(Input::Text("pull lever".into()));
    let save = serde_json::to_value(s.save()).unwrap();
    let changed = CHOICE.replace("You go {path}.", "You walk {path}.");
    let world = Arc::new(World::compile(load(files(&changed)).unwrap().story).unwrap());
    let (restored, loaded) = Session::load(world, save).unwrap();
    assert!(loaded.story_changed);
    assert!(
        loaded.dropped.iter().any(|d| d.starts_with("await:")),
        "{:?}",
        loaded.dropped
    );
    assert!(restored.state().awaiting.is_none());
}

#[test]
fn compile_rejects_an_await_of_two_kinds() {
    let rule = "- id: two\n  on: { verb: [pull, lever] }\n  then:\n    - await: { choice: [a], roll: 1d6 }\n";
    let story = load(files(rule)).unwrap().story;
    assert!(World::compile(story).is_err());
}

#[test]
fn the_session_roll_is_a_notice() {
    let mut s = session(ROLL);
    let step = s.submit(Input::Text("pull lever".into()));
    assert!(
        step.effects.iter().any(
            |e| matches!(e, loremaster_runtime::Effect::Notice { entry } if entry.kind == "roll")
        ),
        "{step:?}"
    );
}

#[test]
fn a_check_rule_that_waits_holds_the_action() {
    let mut s = session(
        r#"
- id: ask
  on: { check: [pull, lever] }
  then:
    - await: { choice: [left, right], into: path }
- id: pulled
  on: { verb: [pull, lever] }
  then:
    - say: "Pulled."
"#,
    );
    let step = s.submit(Input::Text("pull lever".into()));
    let Status::Await { awaiting } = &step.status else {
        panic!("no await: {}", step.plain_text());
    };
    assert!(!step.plain_text().contains("Pulled."));
    let done = s
        .resume(&awaiting.id.clone(), Value::Text("left".into()))
        .unwrap();
    assert!(!done.plain_text().contains("Pulled."));
    assert_eq!(s.state().facts["path"], Value::Text("left".into()));
}
