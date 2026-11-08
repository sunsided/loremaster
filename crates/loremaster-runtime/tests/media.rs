//! Media slots and sound effects (ME-1, ME-2), on `examples/gatehouse`.

use loremaster_model::Value;
use loremaster_model::{Expect, WalkStep, Walkthrough};
use loremaster_project::{load, read_dir};
use loremaster_runtime::{
    ARRIVED, Effect, Input, MediaView, REFUSED, Session, World, run_walkthrough,
};
use std::path::Path;
use std::sync::Arc;

fn world() -> Arc<World> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/gatehouse");
    let project = load(read_dir(&dir).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    Arc::new(World::compile(project.story).unwrap())
}

fn id(cue: &Option<loremaster_runtime::MediaCue>) -> &str {
    cue.as_ref().map_or("", |c| c.id.as_str())
}

fn play(game: &mut Session, text: &str) -> Vec<Effect> {
    game.submit(Input::Text(text.into())).effects
}

#[test]
fn slots_follow_the_state() {
    let mut game = Session::new(world());
    game.start();
    let media = game.media();
    assert_eq!(id(&media.illustration), "gate_closed");
    assert_eq!(id(&media.music), "calm");
    assert_eq!(id(&media.ambience), "wind");
    assert_eq!(id(&media.portrait), "");

    play(&mut game, "open gate");
    assert_eq!(id(&game.media().illustration), "gate_open");

    // A machine state wins over the room.
    play(&mut game, "pull rope");
    assert_eq!(id(&game.media().music), "alarm");
    play(&mut game, "pull rope");
    assert_eq!(id(&game.media().music), "calm");

    // The partner of a talk gives the portrait, with its variant.
    play(&mut game, "north");
    play(&mut game, "talk to keeper");
    assert_eq!(id(&game.media().portrait), "keeper");
    game.submit(Input::Choice(0));
    assert_eq!(id(&game.media().portrait), "keeper_glad");
}

#[test]
fn a_sound_is_an_effect_with_its_caption() {
    let mut game = Session::new(world());
    game.start();
    let effects = play(&mut game, "open gate");
    let sound = effects
        .iter()
        .find_map(|e| match e {
            Effect::Sound { sound } => Some(sound),
            _ => None,
        })
        .expect("a sound effect");
    assert_eq!(sound.id, "gate_creak");
    assert_eq!(sound.file, "assets/gate_creak.wav");
    assert_eq!(
        sound.caption.as_deref(),
        Some("[The gate creaks on its hinges.]")
    );
    // It is a one-shot: the next step does not play it again.
    assert!(
        !play(&mut game, "look")
            .iter()
            .any(|e| matches!(e, Effect::Sound { .. }))
    );
}

#[test]
fn music_is_not_saved_and_comes_back_after_a_restore() {
    let world = world();
    let mut game = Session::new(world.clone());
    game.start();
    play(&mut game, "pull rope");
    let before: MediaView = game.media();
    assert_eq!(id(&before.music), "alarm");

    let save = serde_json::to_string(&game.save()).unwrap();
    for asset in ["alarm", "calm", "wind", "gate_closed"] {
        assert!(
            !save.contains(&format!("\"{asset}\"")),
            "{asset} in the save"
        );
    }
    let (mut restored, _) =
        Session::load(world, serde_json::from_str(&save).unwrap()).expect("the save loads");
    assert_eq!(restored.media(), before);
    assert!(
        !play(&mut restored, "look")
            .iter()
            .any(|e| matches!(e, Effect::Sound { .. }))
    );
}

#[test]
fn a_walkthrough_checks_sounds() {
    let walk = |sound: &str| Walkthrough {
        name: None,
        steps: vec![WalkStep::Checked {
            command: "open gate".into(),
            expect: Expect {
                sound: vec![sound.into()],
                ..Expect::default()
            },
            plan: Box::default(),
            rolls: vec![],
        }],
        seed: None,
        expect: Expect::default(),
        pending: None,
    };
    assert!(run_walkthrough(world(), &walk("gate_creak"), "t").passed());
    let failed = run_walkthrough(world(), &walk("bell"), "t");
    assert!(failed.failures[0].contains("does not play the sound `bell`"));
}

fn regions(game: &Session) -> Vec<(String, String)> {
    (game.media().regions.iter())
        .map(|r| {
            let to = r.thing.clone().or(r.exit.clone()).unwrap_or_default();
            (to, r.name.clone())
        })
        .collect()
}

#[test]
fn hotspots_show_what_the_player_sees() {
    let mut game = Session::new(world());
    game.start();
    assert_eq!(
        regions(&game),
        [
            ("iron_gate".into(), "iron gate".into()),
            ("rope".into(), "bell rope".into())
        ]
    );
    let region = &game.media().regions[0];
    assert_eq!(region.rect, [31, 33, 38, 67]);
    // The open gate has its own image with the same hotspots.
    play(&mut game, "open gate");
    assert_eq!(id(&game.media().illustration), "gate_open");
    assert_eq!(regions(&game).len(), 2);
    play(&mut game, "north");
    assert_eq!(
        regions(&game),
        [
            ("south".into(), "The Road".into()),
            ("keeper".into(), "keeper".into())
        ]
    );
}

#[test]
fn a_stage_host_walks_before_the_command() {
    use loremaster_runtime::{AwaitRequest, Status};
    let mut game = Session::new(world());
    game.set_host_walks(true);
    game.start();
    let step = game.submit(Input::Text("open gate".into()));
    let Status::Await { awaiting } = step.status else {
        panic!("no walk: {:?}", step.status);
    };
    assert!(matches!(
        &awaiting.request,
        AwaitRequest::WalkTo { thing: Some(t), .. } if t.as_str() == "iron_gate"
    ));
    assert!(!game.state().flag("iron_gate.open"), "the gate waits");
    // A refused walk drops the command.
    game.resume(&awaiting.id, Value::Text(REFUSED.into()))
        .unwrap();
    assert!(!game.state().flag("iron_gate.open"));

    let Status::Await { awaiting } = game.submit(Input::Text("open gate".into())).status else {
        panic!("no walk");
    };
    let done = game
        .resume(&awaiting.id, Value::Text(ARRIVED.into()))
        .unwrap();
    assert!(game.state().flag("iron_gate.open"));
    assert!(
        done.effects
            .iter()
            .any(|e| matches!(e, Effect::Sound { .. }))
    );

    // A move walks to the exit first.
    let Status::Await { awaiting } = game.submit(Input::Text("north".into())).status else {
        panic!("no walk");
    };
    assert!(matches!(
        &awaiting.request,
        AwaitRequest::WalkTo { exit: Some(d), .. } if d == "north"
    ));
    game.resume(&awaiting.id, Value::Text(ARRIVED.into()))
        .unwrap();
    assert_eq!(game.state().room.as_str(), "yard");
}

#[test]
fn a_text_host_walks_at_once() {
    let mut game = Session::new(world());
    game.start();
    let step = game.submit(Input::Text("open gate".into()));
    assert_eq!(step.status, loremaster_runtime::Status::Ready);
    assert!(game.state().flag("iron_gate.open"));
}
