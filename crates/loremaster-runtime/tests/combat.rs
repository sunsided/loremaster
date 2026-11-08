//! The built-in combat (P5-03): rounds, Luck, escape, saves and the explorer.

use loremaster_model::Value;
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, carry_over, explore, force_rolls, forced_roll_errors};
use loremaster_runtime::{
    AwaitRequest, Effect, RollDetail, Session, Status, Step, World, run_walkthrough,
};
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
        file(
            "things/dead.yaml",
            "name: BONES\nlocation: { room: gate }\nprops: { skill: 1, stamina: 0 }\n",
        ),
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
fn the_portrait_is_the_enemy_still_standing() {
    let mut f = files("{ enemies: [hound, warden], win: yard, loss: fallen }", 0);
    for (thing, text) in [("hound", HOUND), ("warden", WARDEN)] {
        let path = format!("things/{thing}.yaml");
        f.retain(|x| x.path != path);
        f.push(file(
            &path,
            &format!("{text}media: {{ portrait: {thing} }}\n"),
        ));
        f.push(file(
            &format!("assets/{thing}.yaml"),
            &format!("file: {thing}.png\n"),
        ));
    }
    let mut s = Session::new(compiled(f));
    let step = s.start();
    let portrait = |s: &Session| s.media().portrait.map(|c| c.id.to_string());
    assert_eq!(portrait(&s).as_deref(), Some("hound"));
    play(&mut s, &step, "attack", &WIN);
    assert_eq!(stat(&s, "hound.stamina"), 0);
    assert_eq!(portrait(&s).as_deref(), Some("warden"));
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

fn compiled(f: Vec<SourceFile>) -> Arc<World> {
    Arc::new(World::compile(load(f).unwrap().story).unwrap())
}

#[test]
fn a_walk_into_a_combat_is_not_an_attack() {
    let mut f = files(ONE, 0);
    f[7] = file(
        "rules/fight.yaml",
        &format!(
            "- id: walk\n  on: start\n  then:\n    - await: {{ walk_to: yard }}\n    - say: \"After walk.\"\n- id: ambush\n  on: {{ enter: yard }}\n  then:\n    - combat: {ONE}\n"
        ),
    );
    let mut s = Session::new(compiled(f));
    let step = s.start();
    let step = s
        .resume(&await_id(&step), Value::Text("arrived".into()))
        .unwrap();
    assert_eq!(s.combat().unwrap().round, 0, "{}", step.plain_text());
    assert_eq!(options(&step), ["attack", "escape"]);
}

#[test]
fn effects_after_a_combat_do_not_run_when_the_win_room_ends_the_story() {
    for enemy in ["hound", "dead"] {
        let combat = format!("{{ enemies: [{enemy}], win: yard, loss: fallen }}");
        let mut f = files(&combat, 0);
        f[7].text
            .push_str("- id: finish\n  on: { enter: yard }\n  then:\n    - end: done\n");
        f.push(file(
            "endings/done.yaml",
            "kind: win\ntitle: Done\ntext: Over.\n",
        ));
        let mut s = Session::new(compiled(f));
        let mut step = s.start();
        if enemy == "hound" {
            step = play(&mut s, &step, "attack", &WIN);
        }
        assert!(s.state().ended, "{enemy}");
        assert!(!step.plain_text().contains("After."), "{enemy}");
    }
}

#[test]
fn a_room_that_starts_a_combat_won_at_once_does_not_recurse_for_ever() {
    let mut f = files(ONE, 0);
    f[7] = file(
        "rules/fight.yaml",
        "- id: begin\n  on: start\n  then:\n    - goto: yard\n- id: again\n  on: { enter: yard }\n  then:\n    - combat: { enemies: [dead], win: yard, loss: fallen }\n",
    );
    let mut s = Session::new(compiled(f));
    assert_eq!(s.start().status, Status::Ready);
    assert_eq!(s.state().room.as_str(), "yard");
}

#[test]
fn no_luck_test_on_a_wound_that_kills_anyway() {
    let mut f = files(ONE, 8);
    f[0].text = f[0].text.replace("initial: 12", "initial: 1");
    let mut s = Session::new(compiled(f));
    let step = s.start();
    play(&mut s, &step, "attack", &LOSE);
    assert_eq!(s.state().room.as_str(), "fallen");
    assert_eq!(stat(&s, "luck"), 8);
}

#[test]
fn a_live_edit_that_removes_an_enemy_ends_the_combat() {
    let (s, _) = session(ONE, 0);
    let mut f = files(ONE, 0);
    f.remove(5);
    let next = carry_over(&s, compiled(f));
    assert!(next.combat().is_none());
    assert_eq!(next.status(), Status::Ready);
}

#[test]
fn the_checks_reject_a_repeated_enemy_and_a_stamina_floor() {
    let mut f = files("{ enemies: [warden, warden], win: yard, loss: fallen }", 0);
    f[0].text = f[0].text.replace(
        "{ id: stamina, type: int, initial: 12 }",
        "{ id: stamina, type: int, initial: 12, min: 1 }",
    );
    let diags = check(&load(f).unwrap());
    let has = |t: &str| {
        diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains(t))
    };
    assert!(has("named twice") && has("`min` of `stamina`"), "{diags:?}");
}

#[test]
fn the_rolls_of_a_luck_test_carry_the_target_and_the_result() {
    let (mut s, step) = session(ONE, 9);
    let step = play(&mut s, &step, "attack", &[5, 11]);
    let rolls = rolls(&step);
    assert_eq!(rolls[0].who.as_deref(), Some("GATE WARDEN"));
    assert_eq!(rolls[1].who.as_deref(), Some("You"));
    assert_eq!(rolls[1].total, 11);
    assert!(rolls[1].faces.is_empty(), "a forced total has no faces");

    let step = play(&mut s, &step, "luck", &[4]);
    let roll = rolls_of_one(&step);
    assert_eq!(roll.who.as_deref(), Some("LUCK"));
    assert_eq!(roll.target, Some(9));
    assert_eq!(roll.result.as_deref(), Some("lucky"));
}

#[test]
fn a_roll_without_a_forced_total_has_its_faces() {
    let (mut s, step) = session(ONE, 9);
    let id = await_id(&step);
    let step = s.resume(&id, Value::Text("attack".into())).unwrap();
    let roll = rolls_of_one(&step);
    assert_eq!(roll.faces.len(), 2);
    assert_eq!(roll.faces.iter().sum::<i64>(), roll.total);
}

fn rolls(step: &Step) -> Vec<&RollDetail> {
    (step.effects.iter())
        .filter_map(|e| match e {
            Effect::Roll { roll } => Some(roll),
            _ => None,
        })
        .collect()
}

fn rolls_of_one(step: &Step) -> &RollDetail {
    rolls(step)[0]
}

const STATS: &str = "  - { id: stamina_initial, type: int, initial: 19 }\n  - { id: hearth, type: bool, initial: false }\nstats:\n  - { var: stamina, label: STAMINA, short: ST, max: stamina_initial }\n  - { var: skill, label: SKILL, max: 12 }\n  - { var: hearth, label: HEARTH }\n";

fn with_stats(stats: &str) -> Vec<SourceFile> {
    let mut f = files(ONE, 9);
    f[0].text.push_str(stats);
    f
}

#[test]
fn the_stat_sheet_has_the_current_values() {
    let world = World::compile(load(with_stats(STATS)).unwrap().story).unwrap();
    let mut s = Session::new(Arc::new(world));
    s.start();
    let stats = s.stats();
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0].label, "STAMINA");
    assert_eq!(stats[0].value, Value::Int(12));
    assert_eq!(stats[0].max, Some(19));
    assert_eq!(stats[1].max, Some(12));
    assert_eq!(stats[2].value, Value::Bool(false));
}

#[test]
fn a_story_without_stats_has_no_sheet() {
    let (s, _) = session(ONE, 9);
    assert!(s.stats().is_empty());
}

#[test]
fn check_refuses_a_bad_stat() {
    for (stats, wanted) in [
        ("stats:\n  - { var: nope, label: X }\n", "`nope`"),
        (
            "stats:\n  - { var: skill, label: A }\n  - { var: skill, label: B }\n",
            "twice",
        ),
        (
            "  - { id: w, type: { enum: [a] }, initial: a }\nstats:\n  - { var: w, label: X }\n",
            "`w`",
        ),
        (
            "  - { id: f, type: bool, initial: false }\nstats:\n  - { var: f, label: X, max: 3 }\n",
            "`max`",
        ),
        (
            "stats:\n  - { var: skill, label: X, max: nope }\n",
            "`nope`",
        ),
    ] {
        let found = check(&load(with_stats(stats)).unwrap());
        assert!(
            found
                .iter()
                .any(|d| d.severity == Severity::Error && d.message.contains(wanted)),
            "{stats}: {found:?}"
        );
    }
}

#[test]
fn an_ending_clears_a_combat_that_a_rule_started_on_the_same_event() {
    let mut f = files("{ enemies: [warden], win: yard, loss: fallen }", 0);
    f.push(file("endings/over.yaml", "kind: neutral\ntitle: Over\n"));
    f.push(file(
        "rules/zz_end.yaml",
        "- { id: finish, on: { event: go }, then: [{ end: over }] }\n",
    ));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    assert!(s.state().ended);
    assert!(s.state().combat.is_none());
}
