//! Actors and agendas (D6, WO-5, WO-6): steps at clock times or on
//! conditions, walks along exits, the talk that holds a step back, commands
//! of actors in the Command record, and the actors in the save.

use loremaster_model::Value;
use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring;
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{
    Command, Effect, Input, PLAYER, SAVE_VERSION, Session, StepOutcome, Turn, World,
};
use serde_json::json;
use std::sync::Arc;

const STORY: &str = r#"
title: Agendas
start: hall
clock: { start: "22:00", minutes_per_turn: 10 }
vars:
  - { id: burned, type: bool, initial: false }
  - { id: opened_by_keeper, type: bool, initial: false }
  - { id: opened_by_player, type: bool, initial: false }
"#;

const HALL: &str =
    "name: Hall\ndescription: [\"A hall.\"]\nexits:\n  - { direction: north, to: yard }\n";
const YARD: &str = "name: Yard\ndescription: [\"A yard.\"]\nexits:\n  - { direction: south, to: hall }\n  - { direction: east, to: shed }\n";
const SHED: &str =
    "name: Shed\ndescription: [\"A shed.\"]\nexits:\n  - { direction: west, to: yard }\n";
const KEEPER: &str = "name: keeper\nlocation: { room: shed }\n";
const STOVE: &str =
    "name: stove\nlocation: { room: hall }\nlisted: false\nprops: { open: false }\n";
const PAGES: &str = "name: pages\nlocation: { room: hall }\nportable: true\n";
const ENDING: &str = "kind: win\nrank: 1\ntitle: Won\n";
const DIALOGUE: &str = r#"
with: keeper
start: [hello]
nodes:
  hello:
    say: "Evening."
    options:
      - say: "Go on."
        go: more
  more:
    say: "Nothing more."
    options:
      - say: "Bye."
        go: end
"#;

/// The keeper walks from the shed to the hall for 22:30 and burns the pages,
/// if they are still there.
const AGENDA: &str = r#"
actor: keeper
steps:
  - at: "22:30"
    place: hall
    when: not carried(pages)
    then:
      - move: { pages: nowhere }
      - set: { burned: true }
      - say: "The keeper burns the pages."
    closes: [won]
    reason: "The pages are the proof."
"#;

const RULES: &str = r#"
- id: keeper_opens
  on: { verb: open, direct: stove, actor: keeper }
  then:
    - set: { opened_by_keeper: true }
- id: player_opens
  on: { verb: [open, stove] }
  then:
    - set: { opened_by_player: true }
- id: win
  on: { verb: [take, pages] }
  then:
    - end: won
"#;

fn files(agenda: &str) -> Vec<SourceFile> {
    [
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", YARD),
        ("rooms/shed.yaml", SHED),
        ("things/keeper.yaml", KEEPER),
        ("things/stove.yaml", STOVE),
        ("things/pages.yaml", PAGES),
        ("endings/won.yaml", ENDING),
        ("dialogues/keeper.yaml", DIALOGUE),
        ("rules/rules.yaml", RULES),
        ("agendas/keeper.yaml", agenda),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into()
}

fn world(agenda: &str) -> Arc<World> {
    Arc::new(World::compile(load(files(agenda)).unwrap().story).unwrap())
}

fn game(agenda: &str) -> Session {
    let mut game = Session::new(world(agenda));
    authoring::start(&mut game);
    game
}

fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

fn keeper_room(game: &Session) -> String {
    game.state().locations["keeper"].to_word()
}

#[test]
fn the_agenda_checks_clean() {
    let found = check(&load(files(AGENDA)).unwrap());
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn an_actor_walks_one_room_per_turn_and_runs_the_step_at_its_place() {
    let mut game = game(AGENDA);
    // 22:10 and 22:20: not due yet.
    input(&mut game, "wait");
    input(&mut game, "wait");
    assert_eq!(keeper_room(&game), "shed");
    // 22:30: due. The keeper walks shed -> yard -> hall, one room a turn.
    input(&mut game, "wait");
    assert_eq!(keeper_room(&game), "yard");
    assert_eq!(game.state().step_outcome("keeper", "1"), None);
    let arrives = input(&mut game, "wait").plain_text();
    assert_eq!(keeper_room(&game), "hall");
    assert!(arrives.contains("The keeper comes in."), "{arrives}");
    assert!(!arrives.contains("burns"), "{arrives}");
    // Next turn, at its place, the step runs.
    let burns = input(&mut game, "wait").plain_text();
    assert!(burns.contains("The keeper burns the pages."), "{burns}");
    assert_eq!(
        game.state().step_outcome("keeper", "1"),
        Some(StepOutcome::Ran)
    );
    assert_eq!(game.state().locations["pages"].to_word(), "nowhere");
    // The agenda is over.
    let later = input(&mut game, "wait").plain_text();
    assert!(!later.contains("burns"), "{later}");
}

#[test]
fn an_agenda_walk_gives_a_typed_move_and_the_notice() {
    let mut game = Session::new(world(AGENDA));
    game.start();
    let mut moves = Vec::new();
    for _ in 0..4 {
        let step = game.submit(Input::Text("wait".into()));
        for e in &step.effects {
            match e {
                Effect::Moved { moved } => moves.push((
                    moved.actor.to_string(),
                    moved.name.clone(),
                    moved.from.as_ref().map(|r| r.to_string()),
                    moved.to.to_string(),
                )),
                Effect::Notice { entry } if entry.kind == "move" => {
                    assert!(entry.text.starts_with("keeper -> "), "{}", entry.text);
                }
                _ => {}
            }
        }
    }
    let walk = |from: &str, to: &str| {
        (
            "keeper".to_string(),
            "keeper".to_string(),
            Some(from.to_string()),
            to.to_string(),
        )
    };
    assert_eq!(moves, [walk("shed", "yard"), walk("yard", "hall")]);
}

#[test]
fn a_timed_step_whose_condition_fails_is_skipped() {
    let mut game = game(&AGENDA.replace("not carried(pages)", "carried(pages)"));
    for _ in 0..3 {
        input(&mut game, "wait");
    }
    // 22:30: due, and the player does not carry the pages: skipped, no walk.
    assert_eq!(
        game.state().step_outcome("keeper", "1"),
        Some(StepOutcome::Skipped)
    );
    assert_eq!(keeper_room(&game), "shed");
}

#[test]
fn a_step_without_time_waits_for_its_condition() {
    let agenda = r#"
actor: keeper
steps:
  - id: wake
    when: opened_by_player
    then:
      - say: "The keeper hears the stove."
"#;
    let mut game = game(agenda);
    for _ in 0..5 {
        input(&mut game, "wait");
    }
    assert_eq!(game.state().step_outcome("keeper", "wake"), None);
    let turn = input(&mut game, "open stove").plain_text();
    assert!(turn.contains("The keeper hears the stove."), "{turn}");
    assert_eq!(
        game.state().step_outcome("keeper", "wake"),
        Some(StepOutcome::Ran)
    );
}

#[test]
fn a_talk_with_the_actor_holds_the_step_back() {
    let agenda = r#"
actor: keeper
steps:
  - at: "22:30"
    then:
      - say: "The keeper yawns."
"#;
    let mut game = game(agenda);
    let keeper = Command::verb("talk", vec!["keeper".into()]);
    // To the shed by 22:20.
    input(&mut game, "n");
    input(&mut game, "e");
    // 22:30 in the turn of the talk: the step is due, and waits.
    let talk = authoring::play(&mut game, Input::Command(keeper));
    assert!(
        !talk.plain_text().contains("yawns"),
        "{}",
        talk.plain_text()
    );
    assert!(
        talk.log
            .iter()
            .any(|e| e.kind == "agenda" && e.text == "keeper#1 waits for the talk"),
        "{:?}",
        talk.log
    );
    let more = input(&mut game, "1").plain_text();
    assert!(!more.contains("yawns"), "{more}");
    // The last answer ends the talk; the step runs late, in the same turn.
    let bye = input(&mut game, "1").plain_text();
    assert!(bye.contains("The keeper yawns."), "{bye}");
}

#[test]
fn an_actor_runs_a_command_with_itself_in_the_record() {
    let agenda = r#"
actor: keeper
steps:
  - at: "22:10"
    place: shed
    do: { verb: open, direct: stove }
    then:
      - say: "Done."
"#;
    let mut game = game(agenda);
    let turn = input(&mut game, "wait");
    // The keeper's rule fires, the player's rule for the same verb does not,
    // and the default action of `open` (the player's) does not run.
    let facts = &game.state().facts;
    assert_eq!(facts["opened_by_keeper"], Value::Bool(true));
    assert_eq!(facts["opened_by_player"], Value::Bool(false));
    assert_eq!(facts["stove.open"], Value::Bool(false));
    assert!(turn.plain_text().contains("Done."), "{}", turn.plain_text());

    // The player's command fires the player's rule only.
    let mut game = self::game(agenda.replace("22:10", "23:50").as_str());
    input(&mut game, "open stove");
    assert_eq!(game.state().facts["opened_by_player"], Value::Bool(true));
    assert_eq!(game.state().facts["opened_by_keeper"], Value::Bool(false));
}

#[test]
fn a_check_rule_can_refuse_an_actors_command() {
    let rules = RULES.to_string()
        + r#"
- id: no_keeper
  on: { check: open, actor: keeper }
  then:
    - stop
"#;
    let mut files = files(
        r#"
actor: keeper
steps:
  - at: "22:10"
    do: [open, stove]
    then:
      - say: "Done."
  - then:
      - say: "Next."
"#,
    );
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = rules;
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    let turn = input(&mut game, "wait").plain_text();
    assert!(!turn.contains("Done."), "{turn}");
    assert_eq!(
        game.state().step_outcome("keeper", "1"),
        Some(StepOutcome::Skipped)
    );
    // The refused try took the keeper's turn: the next step runs a turn later.
    assert!(!turn.contains("Next."), "{turn}");
    let next = input(&mut game, "wait").plain_text();
    assert!(next.contains("Next."), "{next}");
}

#[test]
fn the_player_is_the_actor_of_typed_commands() {
    let cmd = Command::verb("take", vec!["pages".into()]);
    let record = cmd.record().unwrap();
    assert_eq!(record.actor.as_str(), PLAYER);
    assert_eq!(record.direct.as_ref().map(|d| d.as_str()), Some("pages"));
    // A host can still send the record without the new slots.
    let wire: Command =
        serde_json::from_value(json!({ "type": "verb", "verb": "take", "objects": ["pages"] }))
            .unwrap();
    assert_eq!(wire, cmd);
    // Only the player takes commands for now (WO-5 switches the actor later).
    let mut game = game(AGENDA);
    let ordered: Command = serde_json::from_value(
        json!({ "type": "verb", "verb": "open", "objects": ["stove"], "actor": "keeper" }),
    )
    .unwrap();
    let turn = authoring::play(&mut game, Input::Command(ordered)).plain_text();
    assert!(turn.contains("Only the player takes commands"), "{turn}");
}

#[test]
fn the_save_keeps_the_actors_and_an_old_save_migrates() {
    let mut game = game(AGENDA);
    for _ in 0..5 {
        input(&mut game, "wait");
    }
    let save = serde_json::to_value(game.save()).unwrap();
    assert_eq!(save["version"], SAVE_VERSION);
    assert_eq!(
        save["state"]["actors"],
        json!({ "keeper": { "steps": { "1": "ran" } } })
    );
    let (restored, loaded) = Session::load(world(AGENDA), save.clone()).unwrap();
    assert!(loaded.dropped.is_empty(), "{:?}", loaded.dropped);
    assert_eq!(restored.state(), game.state());

    // Version 1 had no actors: the agenda starts from its first step.
    let mut old = save.clone();
    old["version"] = json!(1);
    old["state"].as_object_mut().unwrap().remove("actors");
    let (restored, loaded) = Session::load(world(AGENDA), old).unwrap();
    assert_eq!(loaded.from_version, 1);
    assert!(restored.state().actors.is_empty());

    // A story without the step drops its progress, and says so.
    let other = AGENDA.replace("  - at:", "  - id: burn\n    at:");
    let (restored, loaded) = Session::load(world(&other), save).unwrap();
    assert!(loaded.story_changed);
    assert_eq!(loaded.dropped, ["agenda:keeper#1"]);
    assert!(restored.state().actors["keeper"].steps.is_empty());
}

#[test]
fn the_explorer_waits_for_agenda_steps() {
    // The story has no timer and no time rule. The explorer offers `wait`
    // because it has an agenda (as for timers), and finds the ending of the
    // step.
    let agenda = r#"
actor: keeper
steps:
  - at: "22:10"
    then:
      - end: won
"#;
    let files: Vec<SourceFile> = [
        ("story.yaml", STORY),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        (
            "things/keeper.yaml",
            "name: keeper\nlocation: { room: hall }\n",
        ),
        ("endings/won.yaml", ENDING),
        ("agendas/keeper.yaml", agenda),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into();
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let found = explore(world, Budget::default());
    assert_eq!(found.endings.len(), 1, "{found:#?}");
    assert_eq!(found.endings[0].path, ["wait"]);
}

#[test]
fn the_checks_find_mistakes_in_agendas() {
    let agenda = r#"
actor: stove
steps:
  - at: "22:30"
    place: attic
    do: { verb: open, direct: lamp, actor: keeper }
    then:
      - say: "Hm."
    closes: [lost]
  - id: two
    reason: "Nothing closes."
"#;
    let mut files = files(agenda);
    files.push(SourceFile {
        path: "agendas/other.yaml".into(),
        text: "actor: stove\nsteps: []\n".into(),
    });
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = "- id: r\n  on: { verb: open, actor: pages }\n  then: [stop]\n".into();
    let found: Vec<String> = check(&load(files).unwrap())
        .into_iter()
        .map(|d| format!("{} {}: {}", d.file, d.at, d.message))
        .collect();
    for want in [
        "agendas/keeper.yaml steps[0].place: unknown room `attic`",
        "agendas/keeper.yaml steps[0].do.actor: the agenda's actor runs the command; leave `actor` out",
        "agendas/keeper.yaml steps[0].do: unknown thing `lamp`",
        "agendas/keeper.yaml steps[0].closes[0]: unknown ending `lost` in `closes`",
        "agendas/keeper.yaml steps[0].closes: `closes` of step `1` of agenda `keeper` needs a `reason`: why the endings close on purpose",
        "agendas/keeper.yaml steps[1].reason: `reason` of step `two` of agenda `keeper` without `closes`",
        "agendas/keeper.yaml steps[1]: the step does nothing: give it `place`, `do` or `then`",
        "agendas/other.yaml actor: `stove` has two agendas; an actor has one",
        "rules/rules.yaml on.verb.actor: `pages` is a thing, but not an actor: give it an agenda in agendas/",
    ] {
        assert!(found.iter().any(|f| f == want), "{want}\n{found:#?}");
    }
    // One message for the actor of a `do`.
    let at_actor = found
        .iter()
        .filter(|f| f.starts_with("agendas/keeper.yaml steps[0].do.actor:"))
        .count();
    assert_eq!(at_actor, 1, "{found:#?}");
}

#[test]
fn an_indirect_object_needs_a_direct_one() {
    let agenda = "actor: keeper\nsteps:\n  - do: { verb: use, indirect: stove }\n";
    let found: Vec<String> = check(&load(files(agenda)).unwrap())
        .into_iter()
        .map(|d| format!("{} {}: {}", d.file, d.at, d.message))
        .collect();
    let want = "agendas/keeper.yaml steps[0].do.indirect: `indirect` needs a `direct` object: the indirect one is the second";
    assert!(found.iter().any(|f| f == want), "{found:#?}");
}

#[test]
fn a_step_whose_command_ends_the_story_runs_no_effects() {
    let agenda = r#"
actor: keeper
steps:
  - at: "22:10"
    place: shed
    do: [open, stove]
    then:
      - set: { burned: true }
"#;
    let mut files = files(agenda);
    let rules = &mut files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text;
    *rules = rules.replace(
        "    - set: { opened_by_keeper: true }",
        "    - set: { opened_by_keeper: true }\n    - end: won",
    );
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    input(&mut game, "wait");
    let state = game.state();
    assert!(state.ended);
    assert_eq!(state.ending.as_ref().map(|e| e.as_str()), Some("won"));
    assert_eq!(state.facts["opened_by_keeper"], Value::Bool(true));
    assert_eq!(state.facts["burned"], Value::Bool(false));
}

#[test]
fn the_explorer_tries_each_preposition_of_a_rule() {
    let rules = r#"
- id: put_in
  on: { verb: put, direct: pages, indirect: stove, preposition: in }
  then: [{ set: { burned: true } }]
- id: put_on
  on: { verb: put, direct: pages, indirect: stove, preposition: on }
  then: [{ set: { opened_by_player: true } }]
"#;
    let mut files = files(AGENDA);
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = rules.into();
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let found = explore(world, Budget::default());
    for rule in ["put_in", "put_on"] {
        assert!(
            !found.never_fired.iter().any(|r| r == rule),
            "{rule}: {found:#?}"
        );
    }
}

/// The keeper burns the pages in the first turn. Taking them is the best
/// ending; examining the stove is a worse one that stays.
fn burning(closes: &str) -> loremaster_project::Project {
    let agenda = format!(
        r#"
actor: keeper
steps:
  - at: "22:10"
    when: not carried(pages)
    then:
      - move: {{ pages: nowhere }}
{closes}"#
    );
    let rules = "- id: win\n  on: { verb: [take, pages] }\n  then: [{ end: won }]\n- id: lose\n  on: { verb: [examine, stove] }\n  then: [{ end: lost }]\n";
    let files: Vec<SourceFile> = [
        ("story.yaml", STORY),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        (
            "things/keeper.yaml",
            "name: keeper\nlocation: { room: hall }\n",
        ),
        ("things/stove.yaml", STOVE),
        ("things/pages.yaml", PAGES),
        ("endings/won.yaml", ENDING),
        ("endings/lost.yaml", "kind: loss\nrank: 2\ntitle: Lost\n"),
        ("rules/rules.yaml", rules),
        ("agendas/keeper.yaml", agenda.as_str()),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into();
    load(files).unwrap()
}

fn ending_warnings(project: &loremaster_project::Project) -> Vec<loremaster_model::Diagnostic> {
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    let found = explore(world, Budget::default());
    assert!(found.ending_checks.proven, "{found:#?}");
    found
        .ending_checks
        .diagnostics()
        .into_iter()
        .map(|d| project.locate(d))
        .collect()
}

#[test]
fn an_agenda_steps_closes_accepts_its_point_of_no_return() {
    let marked = burning("    closes: [won]\n    reason: \"The pages burn.\"\n");
    let found = ending_warnings(&marked);
    assert!(found.is_empty(), "{found:#?}");

    let found = ending_warnings(&burning(""));
    assert!(
        found.iter().any(|d| d.file == "agendas/keeper.yaml"
            && d.at == "agenda:keeper/steps[0].closes"
            && d.message.contains("`won`")),
        "{found:#?}"
    );
}

#[test]
fn a_waiting_player_does_not_refuse_an_actor() {
    let agenda = "actor: keeper\nsteps:\n  - do: [open, stove]\n";
    let wait = "- { id: wait, on: { check: [take, pages] }, then: [{ await: { choice: [left, right] } }] }\n";
    let mut f = files(agenda);
    f.push(SourceFile {
        path: "rules/wait.yaml".into(),
        text: wait.into(),
    });
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    let turn = input(&mut game, "take pages");
    assert!(game.state().awaiting.is_some());
    assert_eq!(game.state().facts["opened_by_keeper"], Value::Bool(true));
    assert!(
        !turn.log.iter().any(|l| l.text.contains("refused")),
        "{:?}",
        turn.log
    );
}

#[test]
fn a_check_of_an_actor_that_waits_holds_its_command() {
    let agenda = "actor: keeper\nsteps:\n  - do: [open, stove]\n";
    let wait = "- { id: wait, on: { check: open, direct: stove, actor: keeper }, then: [{ await: { choice: [left, right] } }] }\n";
    let mut f = files(agenda);
    f.push(SourceFile {
        path: "rules/wait.yaml".into(),
        text: wait.into(),
    });
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    let turn = input(&mut game, "wait");
    assert!(game.state().awaiting.is_some());
    assert_eq!(game.state().facts["opened_by_keeper"], Value::Bool(false));
    assert_eq!(
        game.state().step_outcome("keeper", "1"),
        Some(StepOutcome::Ran)
    );
    assert!(
        !turn.log.iter().any(|l| l.text.contains("refused")),
        "{:?}",
        turn.log
    );
}
