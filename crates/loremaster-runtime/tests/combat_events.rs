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

const STORY: &str = "title: Fight\nstart: gate\nvars:\n  - { id: skill, type: int, initial: 10 }\n  - { id: stamina, type: int, initial: 12 }\n  - { id: luck, type: int, initial: 0 }\n  - { id: streak, type: int, initial: 0 }\n  - { id: ended, type: bool, initial: false }\n  - { id: how, type: { enum: [none, win, loss, escape, ended] }, initial: none }\n";

fn fight(combat: &str) -> SourceFile {
    file(
        "rules/fight.yaml",
        &format!(
            "- id: fight\n  on: start\n  then:\n    - combat: {combat}\n    - say: \"After.\"\n"
        ),
    )
}

/// A fight with the warden (SKILL 7, STAMINA 8) and the rules `hooks`:
/// story.yaml, the warden, the fight and the hooks first, then the rooms.
fn files(hooks: &str) -> Vec<SourceFile> {
    let room = |name: &str| format!("name: {name}\ndescription: [\"{name}.\"]\n");
    let mut files = vec![
        file("story.yaml", STORY),
        file(
            "things/warden.yaml",
            "name: GATE WARDEN\nlocation: { room: gate }\nprops: { skill: 7, stamina: 8 }\n",
        ),
        fight("{ enemies: [warden], win: yard, loss: fallen, escape: road }"),
        file("rules/hooks.yaml", hooks),
    ];
    if hooks.is_empty() {
        files[3] = file("rooms/hall.yaml", "name: Hall\ndescription: [\"Hall.\"]\n");
    }
    for r in ["gate", "yard", "road", "fallen", "section_7", "shore"] {
        files.push(file(&format!("rooms/{r}.yaml"), &room(r)));
    }
    files
}

fn project(hooks: &str, extra: Vec<SourceFile>) -> Project {
    let mut f = files(hooks);
    f.extend(extra);
    load(f).unwrap()
}

fn compile(f: Vec<SourceFile>) -> Arc<World> {
    Arc::new(World::compile(load(f).unwrap().story).unwrap())
}

/// The messages of the diagnostics of `severity`.
fn messages(f: Vec<SourceFile>, severity: Severity) -> Vec<String> {
    (check(&load(f).unwrap()).iter())
        .filter(|d| d.severity == severity)
        .map(|d| d.message.clone())
        .collect()
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
    answer(s, step, "attack", rolls)
}

fn answer(s: &mut Session, step: &Step, answer: &str, rolls: &[i64]) -> Step {
    force_rolls(s, rolls);
    let done = s
        .resume(&await_id(step), Value::Text(answer.into()))
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
    assert_eq!(
        int(&s, "warden.stamina"),
        8,
        "the first total is the warden's"
    );
    assert_eq!(int(&s, "combat_hit.round"), 1);
}

const OUTCOMES: &str = "- id: won\n  on: { event: combat_end }\n  when: combat_end.outcome == win and combat_end.enemy == \"warden\"\n  then:\n    - set: { how: win }\n- id: lost\n  on: { event: combat_end }\n  when: combat_end.outcome == loss\n  then:\n    - set: { how: loss }\n- id: fled\n  on: { event: combat_end }\n  when: combat_end.outcome == escape\n  then:\n    - set: { how: escape }\n- id: ended\n  on: { event: combat_end }\n  when: combat_end.outcome == ended\n  then:\n    - set: { how: ended }\n";

fn how(s: &Session) -> &Value {
    s.state().facts.get("how").unwrap()
}

#[test]
fn combat_end_tells_win_loss_and_escape() {
    let mut s = Session::new(world(OUTCOMES));
    let mut step = s.start();
    for _ in 0..4 {
        step = play(&mut s, &step, &WIN);
    }
    assert_eq!(s.state().room.as_str(), "yard");
    assert_eq!(how(&s), &Value::Text("win".into()));
    assert!(step.plain_text().contains("After."));

    let mut s = Session::new(world(OUTCOMES));
    let mut step = s.start();
    for _ in 0..6 {
        step = play(&mut s, &step, &LOSE);
    }
    assert_eq!(s.state().room.as_str(), "fallen");
    assert_eq!(how(&s), &Value::Text("loss".into()));

    let mut s = Session::new(world(OUTCOMES));
    let step = s.start();
    answer(&mut s, &step, "escape", &[]);
    assert_eq!(s.state().room.as_str(), "road");
    assert_eq!(how(&s), &Value::Text("escape".into()));
}

const HIT_ENDS: &str = "- id: warden_yields\n  on: { event: combat_hit }\n  when: combat_hit.target == \"warden\"\n  then:\n    - end_combat: shore\n";

#[test]
fn end_combat_from_a_hit_counts_the_hit_then_ends() {
    let mut s = Session::new(world(&format!("{HIT_ENDS}{OUTCOMES}")));
    let step = s.start();
    let step = play(&mut s, &step, &WIN);
    assert_eq!(s.state().room.as_str(), "shore");
    assert!(s.state().combat.is_none());
    assert_eq!(int(&s, "warden.stamina"), 6, "the hit still counts");
    assert_eq!(how(&s), &Value::Text("ended".into()));
    assert!(step.plain_text().contains("After."));
}

#[test]
fn end_combat_from_a_hit_counts_every_hit_of_the_round() {
    let ends = "- id: give_up\n  on: { event: combat_hit }\n  when: combat_hit.target == \"player\"\n  then:\n    - end_combat: shore\n";
    let mut f = files(ends);
    f[2] = fight("{ enemies: [warden, hound], win: yard, loss: fallen, together: true }");
    f.push(file(
        "things/hound.yaml",
        "name: HOUND\nlocation: { room: gate }\nprops: { skill: 5, stamina: 4 }\n",
    ));
    let mut s = Session::new(compile(f));
    let step = s.start();
    // Both enemies hit the player in one round.
    play(&mut s, &step, &[12, 12, 2]);
    assert_eq!(s.state().room.as_str(), "shore");
    assert_eq!(int(&s, "stamina"), 8, "both hits count");
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

#[test]
fn the_checks_reject_flow_effects_in_combat_hooks() {
    let bad = "- id: bad\n  on: { event: combat_round }\n  then:\n    - goto: shore\n    - await: { roll: 2d6 }\n";
    let errors = messages(files(bad), Severity::Error);
    for what in ["`goto`", "`await`"] {
        assert!(
            errors.iter().any(|m| m.contains(&format!("cannot {what}"))),
            "{errors:?}"
        );
    }
}

#[test]
fn the_checks_warn_about_end_combat_without_a_combat_event() {
    let stray = "- id: stray\n  on: { enter: shore }\n  then:\n    - end_combat: yard\n";
    let warnings = messages(files(stray), Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("`end_combat` works only")),
        "{warnings:?}"
    );
    let mut f = files("");
    f.push(file(
        "timers/t.yaml",
        "turns: 2\nat:\n  0:\n    - end_combat: yard\n",
    ));
    let errors = messages(f, Severity::Error);
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`end_combat` work only in rules")),
        "{errors:?}"
    );
}

#[test]
fn the_checks_warn_about_a_combat_hook_without_a_combat() {
    let mut f = files(SKILL_LOSS);
    f[2] = file(
        "rules/fight.yaml",
        "- id: fight\n  on: start\n  then:\n    - say: \"No fight.\"\n",
    );
    let warnings = messages(f, Severity::Warning);
    assert!(
        warnings
            .iter()
            .any(|m| m == "rule on `combat_hit`, but the story has no `combat`"),
        "{warnings:?}"
    );
}

#[test]
fn the_combat_event_ids_are_reserved() {
    let mut f = files("");
    f.push(file(
        "rooms/combat_hit.yaml",
        "name: Hit\ndescription: [\"Hit.\"]\n",
    ));
    let errors = messages(f, Severity::Error);
    assert!(
        errors.iter().any(|m| m.contains("the room `combat_hit`")),
        "{errors:?}"
    );
    let mut f = files("");
    f[0] = file(
        "story.yaml",
        &format!("{STORY}interface:\n  events:\n    - {{ id: combat_end }}\n"),
    );
    let errors = messages(f, Severity::Error);
    assert!(
        errors.iter().any(|m| m.contains("the event `combat_end`")),
        "{errors:?}"
    );
}

#[test]
fn an_emitted_combat_event_with_a_listener_is_no_contract_error() {
    let hooks = "- id: send\n  on: { enter: shore }\n  then:\n    - emit: combat_hit\n- id: hear\n  on: { event: combat_hit }\n  then:\n    - add: { streak: 1 }\n";
    let mut f = files(hooks);
    f[0] = file(
        "story.yaml",
        &format!("{STORY}interface:\n  events:\n    - {{ id: other }}\n"),
    );
    f.push(file(
        "rules/other.yaml",
        "- id: other\n  on: { event: other }\n  then:\n    - add: { streak: 1 }\n",
    ));
    let errors = messages(f, Severity::Error);
    assert_eq!(errors, Vec::<String>::new());
}
