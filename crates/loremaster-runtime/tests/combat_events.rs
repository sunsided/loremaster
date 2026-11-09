//! Rule hooks inside a combat (#230): `combat_hit`, `combat_round`,
//! `combat_end` and the `end_combat` effect.

use loremaster_model::Value;
use loremaster_project::{Project, Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore, force_rolls, forced_roll_errors};
use loremaster_runtime::{AwaitRequest, Session, Status, Step, World, run_walkthrough};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A fight with the warden (SKILL 7, STAMINA 8) and the rules `hooks`.
fn project(hooks: &str, extra: Vec<SourceFile>) -> Project {
    let story = "title: Fight\nstart: gate\nvars:\n  - { id: skill, type: int, initial: 10 }\n  - { id: stamina, type: int, initial: 12 }\n  - { id: luck, type: int, initial: 0 }\n  - { id: streak, type: int, initial: 0 }\n  - { id: ended, type: bool, initial: false }\n";
    let fight = "- id: fight\n  on: start\n  then:\n    - combat: { enemies: [warden], win: yard, loss: fallen, escape: road }\n    - say: \"After.\"\n";
    let room = |name: &str| format!("name: {name}\ndescription: [\"{name}.\"]\n");
    let mut files = vec![
        file("story.yaml", story),
        file(
            "things/warden.yaml",
            "name: GATE WARDEN\nlocation: { room: gate }\nprops: { skill: 7, stamina: 8 }\n",
        ),
        file("rules/fight.yaml", fight),
        file("rules/hooks.yaml", hooks),
    ];
    for r in ["gate", "yard", "road", "fallen", "section_7", "shore"] {
        files.push(file(&format!("rooms/{r}.yaml"), &room(r)));
    }
    files.extend(extra);
    load(files).unwrap()
}

fn world(hooks: &str) -> Arc<World> {
    Arc::new(World::compile(project(hooks, Vec::new()).story).unwrap())
}

fn await_id(step: &Step) -> String {
    match &step.status {
        Status::Await { awaiting } => awaiting.id.clone(),
        other => panic!("no await: {other:?}\n{}", step.plain_text()),
    }
}

fn play(s: &mut Session, step: &Step, rolls: &[i64]) -> Step {
    force_rolls(s, rolls);
    let done = s
        .resume(&await_id(step), Value::Text("attack".into()))
        .unwrap();
    assert_eq!(forced_roll_errors(s), Vec::<String>::new());
    done
}

fn int(s: &Session, name: &str) -> i64 {
    match s.state().facts.get(name) {
        Some(Value::Int(n)) => *n,
        other => panic!("{name}: {other:?}"),
    }
}

const WIN: [i64; 2] = [2, 12];
const LOSE: [i64; 2] = [12, 2];

const TWO_IN_A_ROW: &str = "- id: count_win\n  on: { event: combat_round }\n  when: combat_round.won\n  then:\n    - add: { streak: 1 }\n- id: reset\n  on: { event: combat_round }\n  when: not combat_round.won\n  then:\n    - set: { streak: 0 }\n- id: two_in_a_row\n  on: { event: combat_round }\n  when: streak >= 2\n  then:\n    - end_combat: section_7\n- id: note_end\n  on: { event: combat_end }\n  when: combat_end.outcome == ended\n  then:\n    - set: { ended: true }\n";

#[test]
fn two_won_rounds_in_a_row_turn_to_7() {
    let mut s = Session::new(world(TWO_IN_A_ROW));
    let mut step = s.start();
    for rolls in [WIN, LOSE, WIN] {
        step = play(&mut s, &step, &rolls);
        assert_eq!(s.state().room.as_str(), "gate");
    }
    step = play(&mut s, &step, &WIN);
    assert_eq!(s.state().room.as_str(), "section_7");
    assert!(s.state().combat.is_none());
    assert_eq!(int(&s, "warden.stamina"), 2, "the warden still lives");
    assert_eq!(s.state().facts.get("ended"), Some(&Value::Bool(true)));
    assert!(step.plain_text().contains("After."), "the rule goes on");
    assert_eq!(step.status, Status::Ready);
}

const SKILL_LOSS: &str = "- id: warden_weakens\n  on: { event: combat_hit }\n  when: combat_hit.target == \"warden\"\n  then:\n    - add: { warden.skill: -1 }\n";

#[test]
fn when_the_enemy_is_hit_it_loses_1_skill() {
    let mut s = Session::new(world(SKILL_LOSS));
    let mut step = s.start();
    step = play(&mut s, &step, &WIN);
    assert_eq!(int(&s, "warden.skill"), 6);
    assert_eq!(int(&s, "warden.stamina"), 6);
    play(&mut s, &step, &LOSE);
    assert_eq!(
        int(&s, "warden.skill"),
        6,
        "a wound of the player is no hit on it"
    );
    assert_eq!(int(&s, "stamina"), 10);
}

const FOG: &str = "- id: fog_lifts\n  on: { event: combat_round }\n  when: combat_round.round == 3\n  then:\n    - say: \"The fog lifts, and the warden flees.\"\n    - end_combat: shore\n";

#[test]
fn after_round_3_the_enemy_flees() {
    let mut s = Session::new(world(FOG));
    let mut step = s.start();
    for rolls in [WIN, LOSE, WIN] {
        step = play(&mut s, &step, &rolls);
    }
    assert_eq!(s.state().room.as_str(), "shore");
    assert!(s.state().combat.is_none());
    let text = step.plain_text();
    assert!(
        text.contains("The fog lifts") && text.contains("After."),
        "{text}"
    );
}

const ARMOUR: &str = "- id: armour\n  on: { event: combat_hit }\n  when: combat_hit.target == \"player\" and combat_hit.enemy == \"warden\"\n  then:\n    - set: { combat_hit.damage: 0 }\n";

#[test]
fn a_hook_changes_the_damage_of_the_hit() {
    let mut s = Session::new(world(ARMOUR));
    let mut step = s.start();
    step = play(&mut s, &step, &LOSE);
    assert_eq!(int(&s, "stamina"), 12, "the armour takes the wound");
    play(&mut s, &step, &WIN);
    assert_eq!(int(&s, "warden.stamina"), 6, "the player's hit counts");
}

#[test]
fn hooks_work_when_the_host_rolls() {
    let mut s = Session::new(world(ARMOUR));
    s.set_host_rolls(true);
    let mut step = s.start();
    step = s
        .resume(&await_id(&step), Value::Text("attack".into()))
        .unwrap();
    for total in [12, 2] {
        match &step.status {
            Status::Await { awaiting } => {
                assert!(matches!(awaiting.request, AwaitRequest::Roll { .. }));
            }
            other => panic!("{other:?}"),
        }
        step = s.resume(&await_id(&step), Value::Int(total)).unwrap();
    }
    assert_eq!(int(&s, "stamina"), 12);
    assert_eq!(int(&s, "combat_hit.round"), 1);
}

#[test]
fn a_walkthrough_and_the_explorer_see_the_hooks() {
    let walk = file(
        "tests/t.yaml",
        "steps:\n  - { do: attack, rolls: [2, 12] }\n  - { do: attack, rolls: [2, 12] }\nexpect: { room: section_7 }\n",
    );
    let p = project(TWO_IN_A_ROW, vec![walk]);
    let world = Arc::new(World::compile(p.story.clone()).unwrap());
    let result = run_walkthrough(world.clone(), &p.walkthroughs[0].1, "t");
    assert_eq!(
        result.failures,
        Vec::<String>::new(),
        "{}",
        result.transcript
    );
    let found = explore(world, Budget::default());
    let reached = |id: &str| found.rooms.iter().any(|r| r.id == id && r.reached);
    assert!(reached("section_7") && reached("fallen"), "{found:?}");
}

#[test]
fn the_checks_know_the_combat_events() {
    let diags = check(&project(
        &format!("{TWO_IN_A_ROW}{SKILL_LOSS}{ARMOUR}"),
        Vec::new(),
    ));
    let found: Vec<_> = (diags.iter())
        .filter(|d| d.severity != Severity::Info)
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(found, Vec::<String>::new());
    let bad = "- id: bad\n  on: { event: combat_hit }\n  when: combat_hit.damag > 1\n  then:\n    - end_combat: nowhere\n";
    let diags = check(&project(bad, Vec::new()));
    let errors: Vec<_> = (diags.iter())
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(
        errors
            .iter()
            .any(|m| m.contains("did you mean `combat_hit.damage`")),
        "{errors:?}"
    );
    assert!(errors.iter().any(|m| m.contains("nowhere")), "{errors:?}");
}
