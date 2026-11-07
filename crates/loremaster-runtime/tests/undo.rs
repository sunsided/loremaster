//! Snapshot, restore, undo and the death prompt (SE-8, SE-9).

use loremaster_model::Value;
use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{AwaitRequest, Effect, Input, Session, Status, Step, World};
use proptest::prelude::*;
use std::path::Path;
use std::sync::Arc;

fn lighthouse() -> Arc<World> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    Arc::new(World::compile(load(read_dir(&dir).unwrap()).unwrap().story).unwrap())
}

fn text(s: &str) -> Input {
    Input::Text(s.into())
}

/// A pier: jumping off it drowns the player.
fn pier(header: &str) -> Arc<World> {
    let file = |path: &str, text: &str| SourceFile {
        path: path.into(),
        text: text.into(),
    };
    let files = vec![
        file("story.yaml", &format!("title: Pier\nstart: pier\n{header}")),
        file(
            "rooms/pier.yaml",
            "name: Pier\ndescription: [\"Planks over deep water.\"]\n",
        ),
        file(
            "endings/drowned.yaml",
            "kind: death\ntitle: Drowned\ntext: The water closes over you.\n",
        ),
        file(
            "verbs/jump.yaml",
            "label: Jump\nwords: [jump]\nslots: none\n",
        ),
        file(
            "rules/jump.yaml",
            "- id: jump_off\n  on: { verb: [jump] }\n  instead: true\n  then: [{ end: drowned }]\n",
        ),
    ];
    Arc::new(World::compile(load(files).unwrap().story).unwrap())
}

fn prompt(step: &Step) -> (String, Vec<String>) {
    match &step.status {
        Status::Await { awaiting } => match &awaiting.request {
            AwaitRequest::DeathPrompt { options, .. } => (
                awaiting.id.clone(),
                options.iter().map(|o| o.to_string()).collect(),
            ),
            other => panic!("not a death prompt: {other:?}"),
        },
        other => panic!("no death prompt: {other:?}"),
    }
}

#[test]
fn undo_goes_back_one_input_and_shows_the_room() {
    let mut s = Session::new(lighthouse());
    s.start();
    let start = s.state().clone();
    s.submit(text("n"));
    let cottage = s.state().clone();
    s.submit(text("open drawer"));
    let step = s.undo();
    assert_eq!(s.state(), &cottage);
    assert!(
        step.plain_text().contains("Undone."),
        "{}",
        step.plain_text()
    );
    assert!(
        step.plain_text().contains("Cottage"),
        "{}",
        step.plain_text()
    );
    assert_eq!(s.state(), &cottage, "showing the room changes nothing");
    s.submit(text("undo"));
    assert_eq!(s.state(), &start);
    let none = s.undo();
    assert!(none.plain_text().contains("nothing to undo"));
    assert_eq!(s.state(), &start);
}

#[test]
fn an_input_that_changes_nothing_is_not_an_undo_step() {
    let mut s = Session::new(lighthouse());
    s.start();
    s.submit(text("n"));
    let steps = s.undo_steps();
    s.submit(text("xyzzy plover"));
    assert_eq!(s.undo_steps(), steps);
}

#[test]
fn the_host_sets_the_undo_depth() {
    let mut s = Session::new(lighthouse());
    s.start();
    s.set_undo_depth(2);
    for command in ["n", "open drawer", "take key", "take lamp"] {
        s.submit(text(command));
    }
    assert_eq!(s.undo_steps(), 2);
    s.set_undo_depth(0);
    assert_eq!(s.undo_steps(), 0);
    s.submit(text("s"));
    assert_eq!(s.undo_steps(), 0);
}

#[test]
fn a_snapshot_restores_and_the_restore_can_be_undone() {
    let mut s = Session::new(lighthouse());
    s.start();
    s.submit(text("n"));
    let snap = s.snapshot();
    s.submit(text("open drawer"));
    s.submit(text("take key"));
    let later = s.state().clone();
    let step = s.restore(&snap).unwrap();
    assert_eq!(s.state(), snap.state());
    assert!(step.plain_text().contains("Restored."));
    s.undo();
    assert_eq!(s.state(), &later);
    let json = serde_json::to_value(&snap).unwrap();
    assert_eq!(
        serde_json::from_value::<loremaster_runtime::Snapshot>(json).unwrap(),
        snap
    );
}

#[test]
fn restart_starts_again_with_the_same_seed() {
    let mut s = Session::with_seed(lighthouse(), 9);
    let opening = s.start();
    let fresh = s.state().clone();
    s.submit(text("n"));
    let again = s.restart();
    assert_eq!(again.plain_text(), opening.plain_text());
    assert_eq!(s.state(), &fresh);
}

#[test]
fn a_death_ending_asks_to_undo_restart_or_restore() {
    let mut s = Session::new(pier(""));
    s.start();
    s.submit(text("wait"));
    let before = s.state().clone();
    let died = s.submit(text("jump"));
    assert!(s.state().ended);
    assert!(matches!(died.effects.last(), Some(Effect::Ending { .. })));
    let (id, options) = prompt(&died);
    assert_eq!(options, ["undo", "restart", "restore"]);
    assert!(s.resume(&id, Value::Text("restore".into())).is_err());
    assert!(s.resume(&id, Value::Text("fly".into())).is_err());
    let back = s.resume(&id, Value::Text("undo".into())).unwrap();
    assert_eq!(back.status, Status::Ready);
    assert_eq!(s.state(), &before);

    s.submit(text("jump"));
    let restarted = s.submit(text("restart"));
    assert_eq!(restarted.status, Status::Ready);
    assert!(!s.state().ended);
}

#[test]
fn a_death_without_undo_offers_no_undo_and_a_story_can_turn_the_prompt_off() {
    let mut s = Session::new(pier(""));
    s.set_undo_depth(0);
    s.start();
    let (_, options) = prompt(&s.submit(text("jump")));
    assert_eq!(options, ["restart", "restore"]);

    let mut s = Session::new(pier("death_prompt: false\n"));
    s.start();
    let died = s.submit(text("jump"));
    assert!(
        matches!(died.status, Status::Ended { .. }),
        "{:?}",
        died.status
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Undo after any inputs walks back through the exact states before them.
    #[test]
    fn undo_restores_each_earlier_state(commands in prop::collection::vec(
        prop::sample::select(vec![
            "n", "s", "e", "w", "up", "down", "open drawer", "take key", "take lamp",
            "take matches", "light lamp", "unlock iron door with key", "wait", "look",
            "turn on lens", "drop lamp", "inventory",
        ]),
        0..20,
    )) {
        let mut s = Session::new(lighthouse());
        s.start();
        let mut states = vec![s.state().clone()];
        for command in &commands {
            s.submit(text(command));
            if s.state() != states.last().unwrap() {
                states.push(s.state().clone());
            }
        }
        prop_assert_eq!(s.undo_steps(), states.len() - 1);
        while states.len() > 1 {
            states.pop();
            s.undo();
            prop_assert_eq!(s.state(), states.last().unwrap());
        }
    }
}

#[test]
fn a_snapshot_of_another_story_is_refused_and_a_restore_counts_as_started() {
    let mut s = Session::new(lighthouse());
    s.start();
    s.submit(text("n"));
    let snap = s.snapshot();
    let mut other = Session::new(pier(""));
    other.start();
    assert!(other.restore(&snap).is_err());

    let mut fresh = Session::new(lighthouse());
    fresh.restore(&snap).unwrap();
    let again = fresh.start();
    assert!(again.effects.is_empty(), "{again:?}");
    assert_eq!(fresh.state(), snap.state());
}

#[test]
fn undo_during_a_talk_is_text_for_the_talk() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cantina");
    let world = Arc::new(World::compile(load(read_dir(&dir).unwrap()).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s.submit(text("in"));
    s.submit(text("talk to figure"));
    let talking = s.state().clone();
    s.submit(text("undo"));
    assert!(s.state().talk.is_some(), "the talk is still open");
    assert_ne!(s.undo_steps(), 0);
    assert!(s.state().turns >= talking.turns);
}

#[test]
fn the_text_restore_at_the_death_prompt_says_what_to_do() {
    let mut s = Session::new(pier(""));
    s.start();
    s.submit(text("jump"));
    let step = s.submit(text("restore"));
    assert!(
        step.plain_text().contains("restore a save"),
        "{}",
        step.plain_text()
    );
    assert!(s.state().ended);
}

const COMMANDS: &[&str] = &[
    "n",
    "s",
    "e",
    "w",
    "up",
    "down",
    "open drawer",
    "take key",
    "take lamp",
    "take matches",
    "light lamp",
    "unlock iron door with key",
    "wait",
    "look",
    "turn on lens",
    "drop lamp",
    "inventory",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Random play, a snapshot, more play, a restore: the state is the one of
    /// the snapshot, and the same inputs after it give the same effects as
    /// the first time (also through a save game and a fresh session).
    #[test]
    fn a_snapshot_restores_the_state_and_the_play_after_it(
        before in prop::collection::vec(prop::sample::select(COMMANDS.to_vec()), 0..12),
        after in prop::collection::vec(prop::sample::select(COMMANDS.to_vec()), 1..12),
    ) {
        let mut s = Session::with_seed(lighthouse(), 3);
        s.start();
        for c in &before {
            s.submit(text(c));
        }
        let snap = s.snapshot();
        let save = serde_json::to_value(s.save()).unwrap();
        let first: Vec<Step> = after.iter().map(|c| s.submit(text(c))).collect();

        s.restore(&snap).unwrap();
        prop_assert_eq!(s.state(), snap.state());
        let again: Vec<Step> = after.iter().map(|c| s.submit(text(c))).collect();
        prop_assert_eq!(&again, &first);

        let (mut loaded, _) = Session::load(lighthouse(), save).unwrap();
        prop_assert_eq!(loaded.state(), snap.state());
        let from_save: Vec<Step> = after.iter().map(|c| loaded.submit(text(c))).collect();
        prop_assert_eq!(&from_save, &first);
    }
}
