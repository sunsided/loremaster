//! The built-in combat (P5-03): rounds, Luck, escape, saves and the explorer.

use loremaster_model::Value;
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore, force_rolls, forced_roll_errors};
use loremaster_runtime::{AwaitRequest, Session, Status, Step, World, run_walkthrough};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

const WARDEN: &str =
    "name: GATE WARDEN\nlocation: { room: gate }\nprops: { skill: 7, stamina: 8 }\n";
const HOUND: &str = "name: HOUND\nlocation: { room: gate }\nprops: { skill: 5, stamina: 2 }\n";

fn files(combat: &str, luck: i64) -> Vec<SourceFile> {
    let vars = format!(
        "title: Fight\nstart: gate\nvars:\n  - {{ id: skill, type: int, initial: 10 }}\n  - {{ id: stamina, type: int, initial: 12 }}\n  - {{ id: luck, type: int, initial: {luck} }}\n"
    );
    let rule = format!(
        "- id: fight\n  on: {{ event: go }}\n  then:\n    - combat: {combat}\n    - say: \"After.\"\n- id: begin\n  on: start\n  then:\n    - emit: go\n"
    );
    let room = |name: &str| format!("name: {name}\ndescription: [\"{name}.\"]\n");
    vec![
        file("story.yaml", &vars),
        file("rooms/gate.yaml", &room("Gate")),
        file("rooms/yard.yaml", &room("Yard")),
        file("rooms/road.yaml", &room("Road")),
        file(
            "rooms/fallen.yaml",
            "name: Fallen\ndescription: [\"Fallen.\"]\n",
        ),
        file("things/warden.yaml", WARDEN),
        file("things/hound.yaml", HOUND),
        file("rules/fight.yaml", &rule),
    ]
}

const ONE: &str = "{ enemies: [warden], win: yard, loss: fallen, escape: road }";

fn world(combat: &str, luck: i64) -> Arc<World> {
    Arc::new(World::compile(load(files(combat, luck)).unwrap().story).unwrap())
}

fn session(combat: &str, luck: i64) -> (Session, Step) {
    let mut s = Session::new(world(combat, luck));
    let step = s.start();
    (s, step)
}

fn await_id(step: &Step) -> String {
    match &step.status {
        Status::Await { awaiting } => awaiting.id.clone(),
        other => panic!("no await: {other:?}\n{}", step.plain_text()),
    }
}

fn options(step: &Step) -> Vec<String> {
    match &step.status {
        Status::Await { awaiting } => match &awaiting.request {
            AwaitRequest::Choice { options } => options.iter().map(|o| o.id.to_string()).collect(),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// Play one answer with forced rolls (the walkthrough path).
fn play(s: &mut Session, step: &Step, answer: &str, rolls: &[i64]) -> Step {
    force_rolls(s, rolls);
    let done = s
        .resume(&await_id(step), Value::Text(answer.into()))
        .unwrap();
    assert_eq!(forced_roll_errors(s), Vec::<String>::new());
    done
}

fn stat(s: &Session, name: &str) -> i64 {
    match s.state().facts.get(name) {
        Some(Value::Int(n)) => *n,
        other => panic!("{name}: {other:?}"),
    }
}

/// A round the player wins and one the player loses, as enemy and player rolls.
const WIN: [i64; 2] = [2, 12];
const LOSE: [i64; 2] = [12, 2];

#[test]
fn the_fight_path_of_the_salt_tower_loses_two_stamina() {
    let (mut s, mut step) = session("{ enemies: [warden], win: yard, loss: fallen }", 0);
    assert_eq!(options(&step), ["attack"], "no escape without a target");
    let start = stat(&s, "stamina");
    for rolls in [WIN, LOSE, WIN, WIN, WIN] {
        step = play(&mut s, &step, "attack", &rolls);
    }
    assert_eq!(s.state().room.as_str(), "yard");
    assert_eq!(stat(&s, "stamina"), start - 2);
    assert_eq!(stat(&s, "warden.stamina"), 0);
    assert!(s.state().combat.is_none());
    assert!(step.plain_text().contains("After."), "the rule goes on");
    assert_eq!(step.status, Status::Ready);
}

#[test]
fn a_walkthrough_forces_a_combat_with_rolls() {
    let mut f = files("{ enemies: [warden], win: yard, loss: fallen }", 0);
    f.push(file(
        "tests/t.yaml",
        "steps:\n  - { do: attack, rolls: [2, 12] }\n  - { do: attack, rolls: [12, 2], facts: { stamina: 10 } }\n  - { do: attack, rolls: [2, 12] }\n  - { do: attack, rolls: [2, 12] }\n  - { do: attack, rolls: [2, 12], facts: { stamina: 10 } }\nexpect: { room: yard }\n",
    ));
    let project = load(f).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let result = run_walkthrough(world, &project.walkthroughs[0].1, "t");
    assert_eq!(
        result.failures,
        Vec::<String>::new(),
        "{}",
        result.transcript
    );
}

#[test]
fn the_player_loses() {
    let (mut s, mut step) = session(ONE, 0);
    for _ in 0..6 {
        step = play(&mut s, &step, "attack", &LOSE);
    }
    assert_eq!(s.state().room.as_str(), "fallen");
    assert_eq!(stat(&s, "stamina"), 0);
    assert!(s.state().combat.is_none());
}

#[test]
fn equal_strengths_hurt_nobody() {
    let (mut s, step) = session(ONE, 0);
    // Enemy 9 + 7, player 6 + 10.
    let step = play(&mut s, &step, "attack", &[9, 6]);
    assert_eq!(stat(&s, "stamina"), 12);
    assert_eq!(stat(&s, "warden.stamina"), 8);
    assert_eq!(options(&step), ["attack", "escape"]);
}

#[test]
fn escape_costs_two_stamina_and_leaves() {
    let (mut s, step) = session(ONE, 0);
    assert_eq!(options(&step), ["attack", "escape"]);
    let done = play(&mut s, &step, "escape", &[]);
    assert_eq!(s.state().room.as_str(), "road");
    assert_eq!(stat(&s, "stamina"), 10);
    assert_eq!(done.status, Status::Ready);
}

#[test]
fn luck_changes_the_damage_in_both_directions() {
    // (the round, the Luck roll, stamina after, warden after, luck is 8 then 7).
    let cases: [([i64; 2], i64, i64, i64); 4] = [
        (WIN, 2, 12, 4),  // lucky hit: 4 damage
        (WIN, 12, 12, 7), // unlucky hit: 1 damage
        (LOSE, 2, 11, 8), // lucky wound: 1 damage
        (LOSE, 12, 9, 8), // unlucky wound: 3 damage
    ];
    for (round, luck_roll, stamina, warden) in cases {
        let (mut s, step) = session(ONE, 8);
        let step = play(&mut s, &step, "attack", &round);
        assert_eq!(options(&step), ["luck", "go_on"]);
        assert_eq!(stat(&s, "stamina"), 12, "the hit waits for the test");
        let step = play(&mut s, &step, "luck", &[luck_roll]);
        assert_eq!(stat(&s, "stamina"), stamina);
        assert_eq!(stat(&s, "warden.stamina"), warden);
        assert_eq!(stat(&s, "luck"), 7);
        assert_eq!(options(&step), ["attack", "escape"]);
    }
}

#[test]
fn going_on_takes_the_normal_damage_and_luck_stays() {
    let (mut s, step) = session(ONE, 8);
    let step = play(&mut s, &step, "attack", &LOSE);
    play(&mut s, &step, "go_on", &[]);
    assert_eq!(stat(&s, "stamina"), 10);
    assert_eq!(stat(&s, "luck"), 8);
}

#[test]
fn no_luck_test_on_a_killing_hit_or_without_luck() {
    let (mut s, step) = session("{ enemies: [hound], win: yard, loss: fallen }", 8);
    play(&mut s, &step, "attack", &WIN);
    assert_eq!(s.state().room.as_str(), "yard", "the hit killed the hound");
    let (mut s, step) = session(ONE, 0);
    let step = play(&mut s, &step, "attack", &WIN);
    assert_eq!(options(&step), ["attack", "escape"]);
}

#[test]
fn two_enemies_fight_one_after_the_other() {
    let (mut s, step) = session("{ enemies: [hound, warden], win: yard, loss: fallen }", 0);
    let step = play(&mut s, &step, "attack", &WIN);
    assert_eq!(stat(&s, "hound.stamina"), 0);
    assert_eq!(s.state().room.as_str(), "gate", "the warden is next");
    let view = s.combat().unwrap();
    assert_eq!(view.round, 1);
    assert_eq!(view.enemies[0].strength, Some(7));
    assert_eq!(view.enemies[1].strength, None);
    assert_eq!(view.player_strength, Some(22));
    assert_eq!(view.enemies[1].stamina, 8);
    assert!(view.log[0].contains("You hit HOUND"), "{:?}", view.log);
    let mut step = step;
    for _ in 0..4 {
        step = play(&mut s, &step, "attack", &WIN);
    }
    assert_eq!(s.state().room.as_str(), "yard");
    let _ = step;
}

#[test]
fn enemies_can_fight_all_at_once() {
    let (mut s, step) = session(
        "{ enemies: [warden, hound], win: yard, loss: fallen, together: true }",
        0,
    );
    // The warden is the target; both roll. The player beats the warden, the hound wins.
    let step = play(&mut s, &step, "attack", &[2, 12, 3]);
    let _ = step;
    assert_eq!(stat(&s, "warden.stamina"), 6, "the player beats the warden");
    assert_eq!(stat(&s, "stamina"), 10, "the hound hits");
}

#[test]
fn a_save_in_the_middle_of_a_combat_restores() {
    let (mut s, step) = session(ONE, 8);
    let step = play(&mut s, &step, "attack", &LOSE);
    let save = serde_json::to_value(s.save()).unwrap();
    let (mut restored, _) = Session::load(world(ONE, 8), save).unwrap();
    assert_eq!(restored.status(), step.status);
    assert_eq!(restored.combat(), s.combat());
    let step = play(&mut restored, &step, "luck", &[2]);
    assert_eq!(stat(&restored, "stamina"), 11);
    assert_eq!(options(&step), ["attack", "escape"]);
}

#[test]
fn a_second_answer_to_the_same_await_is_refused() {
    let (mut s, step) = session(ONE, 0);
    let id = await_id(&step);
    s.resume(&id, Value::Text("attack".into())).unwrap();
    assert!(s.resume(&id, Value::Text("attack".into())).is_err());
}

#[test]
fn the_explorer_sees_win_loss_and_escape_and_ends() {
    let found = explore(world(ONE, 8), Budget::default());
    assert!(!found.truncated, "{found:?}");
    let reached = |id: &str| found.rooms.iter().any(|r| r.id == id && r.reached);
    assert!(reached("yard") && reached("fallen") && reached("road"));
}

#[test]
fn the_checks_reject_a_bad_combat() {
    let mut f = files("{ enemies: [warden], win: yard, loss: nowhere }", 0);
    f[5] = file("things/warden.yaml", "name: X\nlocation: { room: gate }\n");
    let diags = check(&load(f).unwrap());
    let errors: Vec<_> = (diags.iter())
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("needs a number `skill`")),
        "{errors:?}"
    );
    assert!(errors.iter().any(|m| m.contains("nowhere")), "{errors:?}");
}
