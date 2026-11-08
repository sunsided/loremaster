//! The host contract: declared actions and events, their checks (IF-2), the
//! host events in play, the interface hash (IF-5) and walkthroughs (IF-6).

use loremaster_model::{Interface, Value};
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::{Session, World, run_walkthrough};
use std::collections::BTreeMap;
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

const STORY: &str = r#"
title: Contract
start: hall
vars:
  - { id: lock, type: { enum: [closed, success, broken_pick] }, initial: closed }
  - { id: alive, type: bool, initial: true }
interface:
  actions:
    - { id: lockpick, args: { difficulty: int }, results: [success, broken_pick] }
  events:
    - { id: killed, args: { boss: text, level: int }, when: alive }
"#;

const RULES: &str = r#"
- id: pick
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { action: lockpick, args: { difficulty: 3 }, into: lock }
    - say: "{lock == success: It opens.}{lock == broken_pick: The pick snaps.}"
- id: on_killed
  on: { event: killed }
  then:
    - say: "You killed {killed.boss}."
"#;

fn files(rules: &str) -> Vec<SourceFile> {
    vec![
        file("story.yaml", STORY),
        file(
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall with a lever.\"]\n",
        ),
        file(
            "things/lever.yaml",
            "name: lever\nlocation: { room: hall }\n",
        ),
        file("verbs/pull.yaml", "label: Pull\nwords: [pull]\n"),
        file("rules/lever.yaml", rules),
    ]
}

fn errors(rules: &str) -> Vec<String> {
    check(&load(files(rules)).unwrap())
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

fn world() -> Arc<World> {
    Arc::new(World::compile(load(files(RULES)).unwrap().story).unwrap())
}

#[test]
fn a_correct_story_has_no_errors_and_awaits_take_the_declared_results() {
    assert_eq!(errors(RULES), Vec::<String>::new());
    let story = load(files(RULES)).unwrap().story;
    let loremaster_model::Effect::Await(spec) = &story.rules[0].then[0] else {
        panic!("first effect is the await");
    };
    assert_eq!(spec.results.len(), 2);
}

#[test]
fn an_undeclared_action_fails() {
    let e = errors(&RULES.replace("action: lockpick", "action: pickpocket"));
    assert!(
        e.iter()
            .any(|m| m.contains("`pickpocket` is not in `interface:`")),
        "{e:?}"
    );
}

#[test]
fn a_wrong_argument_type_fails() {
    let e = errors(&RULES.replace("difficulty: 3", "difficulty: \"hard\""));
    assert!(e.iter().any(|m| m.contains("`difficulty`")), "{e:?}");
}

#[test]
fn a_missing_branch_fails() {
    let e = errors(&RULES.replace("{lock == broken_pick: The pick snaps.}", ""));
    assert!(
        e.iter().any(|m| m.contains("result `broken_pick`")),
        "{e:?}"
    );
}

#[test]
fn an_event_with_one_side_must_be_declared() {
    let unheard = format!("{RULES}\n- id: send\n  on: start\n  then:\n    - emit: helo\n");
    let e = errors(&unheard);
    assert!(
        e.iter()
            .any(|m| m.contains("`helo` is not in `interface:`")),
        "{e:?}"
    );
    let unsent = format!("{RULES}\n- id: hear\n  on: {{ event: helo }}\n  then:\n    - stop\n");
    let e = errors(&unsent);
    assert!(e.iter().any(|m| m.contains("`helo`")), "{e:?}");
    // Sent and heard inside the story: its own event, no declaration needed.
    let both = format!(
        "{RULES}\n- id: send\n  on: start\n  then:\n    - emit: tick_tock\n- id: hear\n  on: {{ event: tick_tock }}\n  then:\n    - stop\n"
    );
    assert_eq!(errors(&both), Vec::<String>::new());
}

#[test]
fn a_host_event_runs_rules_and_its_arguments_read_as_facts() {
    let mut s = Session::new(world());
    s.start();
    let args = BTreeMap::from([
        ("boss".to_string(), Value::Text("the ogre".into())),
        ("level".to_string(), Value::Int(3)),
    ]);
    let step = s.host_event("killed", args).unwrap();
    assert!(
        step.plain_text().contains("You killed the ogre."),
        "{}",
        step.plain_text()
    );
}

#[test]
fn a_host_event_is_refused_when_undeclared_mistyped_or_its_precondition_fails() {
    let mut s = Session::new(world());
    s.start();
    assert!(s.host_event("nope", BTreeMap::new()).is_err());
    let wrong = BTreeMap::from([
        ("boss".to_string(), Value::Int(1)),
        ("level".to_string(), Value::Int(3)),
    ]);
    assert!(s.host_event("killed", wrong).unwrap_err().contains("boss"));
    s.submit(loremaster_runtime::Input::Text("pull lever".into()));
    let args = BTreeMap::from([
        ("boss".to_string(), Value::Text("x".into())),
        ("level".to_string(), Value::Int(1)),
    ]);
    // A rule waits: an event cannot come between.
    assert!(s.host_event("killed", args).unwrap_err().contains("waits"));
    let mut dead = Session::new(Arc::new(
        World::compile(
            load(vec![
                file(
                    "story.yaml",
                    &STORY.replace("initial: true", "initial: false"),
                ),
                file("rooms/hall.yaml", "name: Hall\ndescription: [\"Hall.\"]\n"),
                file(
                    "rules/r.yaml",
                    "- { id: r, on: { event: killed }, then: [stop] }\n",
                ),
            ])
            .unwrap()
            .story,
        )
        .unwrap(),
    ));
    dead.start();
    let args = BTreeMap::from([
        ("boss".to_string(), Value::Text("x".into())),
        ("level".to_string(), Value::Int(1)),
    ]);
    assert!(
        dead.host_event("killed", args)
            .unwrap_err()
            .contains("precondition")
    );
}

#[test]
fn the_session_refuses_a_host_built_for_another_interface() {
    let same = world().story.interface.clone().unwrap();
    assert!(Session::for_host(world(), &same).is_ok());
    let other = Interface::from_json(r#"{ "actions": [{ "id": "dance" }] }"#).unwrap();
    let Err(mismatch) = Session::for_host(world(), &other) else {
        panic!("the session must refuse");
    };
    let text = mismatch.to_string();
    assert!(text.contains("added: action lockpick"), "{text}");
    assert!(text.contains("event killed"), "{text}");
    assert!(text.contains("removed: action dance"), "{text}");
}

fn walk(yaml: &str) -> loremaster_runtime::WalkResult {
    let mut all = files(RULES);
    all.push(file("tests/t.yaml", yaml));
    let project = load(all).unwrap();
    let (_, w) = project.walkthroughs[0].clone();
    run_walkthrough(world(), &w, "t")
}

#[test]
fn a_walkthrough_gives_only_declared_results() {
    let ok = walk("steps:\n  - pull lever\n  - { do: success, see: \"It opens\" }\n");
    assert!(ok.passed(), "{:?}", ok.failures);
    let bad = walk("steps:\n  - pull lever\n  - { do: \"1\" }\n");
    assert!(
        bad.failures
            .iter()
            .any(|f| f.contains("not a declared result")),
        "{:?}",
        bad.failures
    );
}

#[test]
fn a_walkthrough_reports_host_events() {
    let ok = walk(
        "steps:\n  - { event: killed, args: { boss: ogre, level: 2 }, see: \"You killed ogre\" }\n",
    );
    assert!(ok.passed(), "{:?}\n{}", ok.failures, ok.transcript);
    let bad = walk("steps:\n  - { event: killed, args: { boss: 3, level: 2 } }\n");
    assert!(!bad.passed());
}

#[test]
fn an_event_step_while_awaiting_is_no_answer_to_the_action() {
    let r = walk("steps:\n  - pull lever\n  - { event: killed, args: { boss: ogre, level: 2 } }\n");
    assert!(
        r.failures.iter().any(|f| f.contains("waits")),
        "{:?}",
        r.failures
    );
    assert!(
        !r.failures
            .iter()
            .any(|f| f.contains("not a declared result")),
        "{:?}",
        r.failures
    );
}
