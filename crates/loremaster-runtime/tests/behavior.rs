//! Behaviors in Rust: the Zork thief steals, replays the same way, and keeps
//! all of its state in the save game.

#[path = "../../../examples/zork1/thief.rs"]
mod thief;

use loremaster_model::{Id, Location};
use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Game, State, World};
use std::sync::Arc;

fn world(with_thief: bool) -> Arc<World> {
    let files = [
        ("story.yaml", "title: Cave\nstart: cave\nbehaviors: [thief]\n"),
        (
            "rooms/cave.yaml",
            "name: Cave\ntags: [dark]\ndescription: [\"A cave.\"]\nexits:\n  - { direction: east, to: hole }\n",
        ),
        (
            "rooms/hole.yaml",
            "name: Hole\ntags: [dark]\ndescription: [\"A hole.\"]\nexits:\n  - { direction: west, to: cave }\n",
        ),
        (
            "things/lamp.yaml",
            "name: lamp\nlocation: carried\nportable: true\nprops: { lit: true }\n",
        ),
        ("things/gem.yaml", "name: gem\nlocation: carried\nportable: true\n"),
        (
            "things/thief.yaml",
            "name: thief\nlocation: nowhere\ncontainer: true\nprops: { open: true }\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap();
    let mut world = World::compile(project.story).unwrap();
    if with_thief {
        world.register(thief::Thief {
            thief: Id::new("thief"),
            home: Id::new("cave"),
            treasures: vec![Id::new("gem")],
        });
    }
    Arc::new(world)
}

/// Wait until the gem is gone, at most 60 turns. Returns the turn it went and the state.
fn wait_for_theft(world: Arc<World>) -> (Option<u32>, State) {
    let mut game = Game::new(world);
    game.start();
    for _ in 0..60 {
        game.input("wait");
        if game.state().location_of("gem") == Some(&Location::In(Id::new("thief"))) {
            return (Some(game.state().turns), game.state().clone());
        }
    }
    (None, game.state().clone())
}

#[test]
fn the_thief_steals_and_replays_the_same_way() {
    let (when, _) = wait_for_theft(world(true));
    assert!(
        when.is_some(),
        "the thief did not steal the gem in 60 turns"
    );
    assert_eq!(when, wait_for_theft(world(true)).0);
}

#[test]
fn without_the_code_the_story_plays_without_the_thief() {
    let w = world(false);
    assert_eq!(w.missing_behaviors(), [&Id::new("thief")]);
    let (when, state) = wait_for_theft(w);
    assert_eq!(when, None);
    assert_eq!(state.location_of("thief"), Some(&Location::Nowhere));
}

#[test]
fn a_saved_game_goes_on_like_the_game_it_saved() {
    let w = world(true);
    let mut game = Game::new(w.clone());
    game.start();
    for _ in 0..5 {
        game.input("wait");
    }
    let save = serde_json::to_string(game.state()).unwrap();
    let mut restored = Game::restore(w, serde_json::from_str(&save).unwrap());
    for _ in 0..30 {
        let a = game.input("wait").plain_text();
        let b = restored.input("wait").plain_text();
        assert_eq!(a, b);
    }
    assert_eq!(game.state(), restored.state());
}

#[test]
fn the_zork_thief_knows_the_things_of_zork() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/zork1");
    let project = load(loremaster_project::read_dir(&root).unwrap()).unwrap();
    assert!(project.story.behaviors.contains(&Id::new("thief")));
    let t = thief::Thief::zork();
    for id in t.treasures.iter().chain([&t.thief]) {
        assert!(
            project.story.thing(id.as_str()).is_some(),
            "no thing `{id}`"
        );
    }
    assert!(project.story.room(t.home.as_str()).is_some());
    let mut world = World::compile(project.story).unwrap();
    world.register(t);
    assert!(world.missing_behaviors().is_empty());
}

/// Counts the events it gets in `seen`, and writes `other`, which its
/// manifest does not name.
#[derive(Debug)]
struct Counter;

impl loremaster_runtime::Behavior for Counter {
    fn id(&self) -> &str {
        "counter"
    }

    fn manifest(&self) -> loremaster_runtime::Manifest {
        loremaster_runtime::Manifest::new()
            .events([loremaster_runtime::EventKind::Custom])
            .reads(["seen"])
            .writes(["seen"])
    }

    fn handle(
        &self,
        _: &loremaster_runtime::Event,
        cx: &mut loremaster_runtime::Cx<'_>,
    ) -> loremaster_runtime::Flow {
        let seen = match cx.fact("seen") {
            Some(loremaster_model::Value::Int(n)) => *n,
            _ => 0,
        };
        cx.set("seen", loremaster_model::Value::Int(seen + 1));
        cx.set("other", loremaster_model::Value::Bool(true));
        loremaster_runtime::Flow::Continue
    }
}

fn counter_world() -> Arc<World> {
    let files = [
        (
            "story.yaml",
            "title: Bell\nstart: hall\nbehaviors: [counter]\nvars:\n  - { id: seen, type: int, initial: 0 }\n  - { id: other, type: bool, initial: false }\n",
        ),
        ("rooms/hall.yaml", "name: Hall\ndescription: [\"A hall.\"]\n"),
        ("verbs/ring.yaml", "label: Ring\nwords: [ring]\nslots: none\n"),
        (
            "rules/ring.yaml",
            "- id: ring\n  on: { verb: [ring] }\n  instead: true\n  then: [{ emit: bell }]\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let mut world = World::compile(load(files.into()).unwrap().story).unwrap();
    world.register(Counter);
    Arc::new(world)
}

#[test]
fn a_behavior_gets_only_the_events_of_its_manifest() {
    let mut game = Game::new(counter_world());
    game.start();
    game.input("wait");
    game.input("wait");
    assert_eq!(
        game.state().facts.get("seen"),
        Some(&loremaster_model::Value::Int(0)),
        "no turn events"
    );
    let turn = game.input("ring");
    assert_eq!(
        game.state().facts.get("seen"),
        Some(&loremaster_model::Value::Int(1))
    );
    // A write that the manifest does not name is a note for the author.
    assert!(
        turn.log
            .iter()
            .any(|l| l.kind == "author" && l.text.contains("`other`")),
        "{:?}",
        turn.log
    );
    assert!(!turn.log.iter().any(|l| l.text.contains("`seen`")));
}

#[test]
fn the_rules_view_and_the_explorer_use_the_manifest() {
    let world = counter_world();
    assert!(!world.behaviors_watch_time());
    let game = Game::new(world);
    let view = game
        .rules()
        .into_iter()
        .find(|r| r.id == "behavior:counter")
        .unwrap();
    assert!(
        view.when.contains("on custom; reads seen; writes seen"),
        "{}",
        view.when
    );
    // The thief declares start and turn events: waiting matters.
    let thief = world_thief();
    assert_eq!(
        thief.manifest("thief").unwrap().events,
        Some(vec![
            loremaster_runtime::EventKind::Start,
            loremaster_runtime::EventKind::Turn
        ])
    );
    assert!(thief.behaviors_watch_time());
}

#[test]
fn manifest_new_handles_every_event_until_events_narrows_it() {
    use loremaster_runtime::{EventKind, Manifest};
    let m = Manifest::new().reads(["here"]);
    assert!(m.handles(EventKind::Turn) && m.handles(EventKind::Custom));
    let m = m.events([EventKind::Custom]);
    assert!(!m.handles(EventKind::Turn) && m.handles(EventKind::Custom));
    assert!(!m.may_write("here"));
}

fn world_thief() -> Arc<World> {
    world(true)
}

/// A behavior that reads only the narrow view of `Cx` (XP-1).
#[derive(Debug)]
struct Watcher;

impl loremaster_runtime::Behavior for Watcher {
    fn id(&self) -> &str {
        "watcher"
    }

    fn handle(
        &self,
        event: &loremaster_runtime::Event,
        cx: &mut loremaster_runtime::Cx<'_>,
    ) -> loremaster_runtime::Flow {
        if matches!(event, loremaster_runtime::Event::Turn) {
            let dark = cx.room_has_tag(cx.here().as_str(), "dark");
            let line = format!(
                "turn {} dark {dark} hole {} visited {} roll {}",
                cx.turns(),
                cx.neighbours("cave").len(),
                cx.visited("hole"),
                cx.roll(6),
            );
            cx.say(line);
        }
        loremaster_runtime::Flow::Continue
    }
}

#[test]
fn a_behavior_reads_the_narrow_view_and_rolls_through_cx() {
    let files = [
        ("story.yaml", "title: Cave\nstart: cave\nbehaviors: [watcher]\n"),
        (
            "rooms/cave.yaml",
            "name: Cave\ntags: [dark]\ndescription: [\"A cave.\"]\nexits:\n  - { direction: east, to: hole }\n",
        ),
        (
            "rooms/hole.yaml",
            "name: Hole\ndescription: [\"A hole.\"]\nexits:\n  - { direction: west, to: cave }\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let mut world = World::compile(load(files.into()).unwrap().story).unwrap();
    world.register(Watcher);
    let world = Arc::new(world);
    let play = || {
        let mut game = Game::new(world.clone());
        game.start();
        let first = game.input("wait").plain_text();
        let second = game.input("e").plain_text();
        (first, second, game.state().rng)
    };
    let (first, second, rng) = play();
    assert!(
        first.contains("turn 1 dark true hole 1 visited false roll"),
        "{first}"
    );
    assert!(
        second.contains("turn 2 dark false hole 1 visited true roll"),
        "{second}"
    );
    assert_eq!(play(), (first, second, rng), "the rolls replay");
}

/// Says where it was called from; stops when the fact `block` is true.
#[derive(Debug)]
struct Gate;

impl loremaster_runtime::Behavior for Gate {
    fn id(&self) -> &str {
        "gate"
    }

    fn manifest(&self) -> loremaster_runtime::Manifest {
        loremaster_runtime::Manifest::new().reads(["block"])
    }

    fn handle(
        &self,
        event: &loremaster_runtime::Event,
        cx: &mut loremaster_runtime::Cx<'_>,
    ) -> loremaster_runtime::Flow {
        let loremaster_runtime::Event::Call(rule) = event else {
            return loremaster_runtime::Flow::Continue;
        };
        cx.say(format!("The gate hears {rule}."));
        if cx.fact("block") == Some(&loremaster_model::Value::Bool(true)) {
            loremaster_runtime::Flow::Stop
        } else {
            loremaster_runtime::Flow::Continue
        }
    }
}

fn gate_files(behaviors: &str) -> Vec<SourceFile> {
    [
        (
            "story.yaml",
            format!("title: Gate\nstart: hall\nbehaviors: [{behaviors}]\nvars:\n  - {{ id: block, type: bool, initial: false }}\n"),
        ),
        ("rooms/hall.yaml", "name: Hall\ndescription: [\"A hall.\"]\n".into()),
        ("verbs/knock.yaml", "label: Knock\nwords: [knock]\nslots: none\n".into()),
        ("verbs/bar.yaml", "label: Bar\nwords: [bar]\nslots: none\n".into()),
        (
            "rules/knock.yaml",
            "- id: knock\n  on: { verb: [knock] }\n  instead: true\n  then:\n    - say: \"You knock.\"\n    - call: gate\n    - say: \"It opens.\"\n- id: bar\n  on: { verb: [bar] }\n  instead: true\n  then: [{ set: { block: true } }]\n".into(),
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text,
    })
    .into()
}

#[test]
fn call_runs_a_behavior_at_its_place_in_the_rule() {
    let mut world = World::compile(load(gate_files("gate")).unwrap().story).unwrap();
    world.register(Gate);
    let world = Arc::new(world);
    let mut game = Game::new(world.clone());
    game.start();
    let text = game.input("knock").plain_text();
    let (a, b, c) = (
        text.find("You knock.").unwrap(),
        text.find("The gate hears knock.").unwrap(),
        text.find("It opens.").unwrap(),
    );
    assert!(a < b && b < c, "{text}");

    // `Flow::Stop` stops the rule, like `stop`.
    let mut game = Game::new(world);
    game.start();
    game.input("bar");
    let text = game.input("knock").plain_text();
    assert!(text.contains("The gate hears knock."), "{text}");
    assert!(!text.contains("It opens."), "{text}");
}

#[test]
fn a_call_without_the_behavior_is_a_note_and_a_wrong_call_is_an_error() {
    let world = Arc::new(World::compile(load(gate_files("gate")).unwrap().story).unwrap());
    let mut game = Game::new(world);
    game.start();
    let turn = game.input("knock");
    assert!(turn.plain_text().contains("It opens."));
    assert!(
        turn.log
            .iter()
            .any(|l| l.kind == "author" && l.text.contains("knock calls gate")),
        "{:?}",
        turn.log
    );

    let project = load(gate_files("other")).unwrap();
    let errors: Vec<_> = loremaster_project::check(&project)
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert!(
        errors
            .iter()
            .any(|m| m.contains("`call: gate` names no behavior")),
        "{errors:?}"
    );
}

#[test]
fn a_call_to_a_registered_but_unnamed_behavior_says_so() {
    let mut world = World::compile(load(gate_files("other")).unwrap().story).unwrap();
    world.register(Gate);
    let mut game = Game::new(Arc::new(world));
    game.start();
    let turn = game.input("knock");
    assert!(
        turn.log
            .iter()
            .any(|l| l.kind == "author" && l.text.contains("does not name")),
        "{:?}",
        turn.log
    );
}

/// The gate story with a timer and a talk that call the gate; `block`
/// starts true, so the gate stops the effects after it.
fn gate_owners_world() -> Arc<World> {
    let mut files = gate_files("gate");
    for f in &mut files {
        if f.path == "story.yaml" {
            f.text = f.text.replace("initial: false", "initial: true");
        }
    }
    files.extend(
        [
            (
                "timers/clock.yaml",
                "turns: 1\nrunning: true\nat:\n  0: [{ call: gate }, { say: \"The clock ticks on.\" }]\n",
            ),
            (
                "rules/clock.yaml",
                "- id: clock_done\n  on: { timer: clock }\n  then: [{ say: \"The clock has run out.\" }]\n",
            ),
            (
                "timers/bell.yaml",
                "turns: 1\nrunning: true\nat:\n  0: [{ say: \"A bell rings.\" }]\n",
            ),
            (
                "dialogues/sexton.yaml",
                "with: sexton\nstart: [hi]\nnodes:\n  hi:\n    say: \"Hm?\"\n    options:\n      - say: \"Goodbye.\"\n        then: [{ end: drowned }, { say: \"After the end.\" }]\n        go: end\n",
            ),
            ("things/sexton.yaml", "name: sexton\nlocation: { room: hall }\n"),
            (
                "endings/drowned.yaml",
                "kind: death\ntitle: Drowned\n",
            ),
            (
                "things/keeper.yaml",
                "name: keeper\nlocation: { room: hall }\n",
            ),
            (
                "dialogues/keeper.yaml",
                "with: keeper\nstart: [hello]\nnodes:\n  hello:\n    say: \"Yes?\"\n    options:\n      - say: \"Open up.\"\n        then: [{ call: gate }, { say: \"The keeper opens.\" }]\n        go: end\n",
            ),
        ]
        .map(|(path, text)| SourceFile {
            path: path.into(),
            text: text.into(),
        }),
    );
    let mut world = World::compile(load(files).unwrap().story).unwrap();
    world.register(Gate);
    Arc::new(world)
}

#[test]
fn a_call_in_a_timer_or_a_dialogue_names_its_owner_and_can_stop() {
    use loremaster_runtime::{Input, Session};
    let mut s = Session::new(gate_owners_world());
    s.start();
    let text = s.submit(Input::Text("wait".into())).plain_text();
    assert!(text.contains("The gate hears timer:clock."), "{text}");
    assert!(!text.contains("The clock ticks on."), "{text}");
    // The stop ends only the clock's step: its rules still fire when it runs
    // out, and the next timer of the same tick runs its effects.
    assert!(text.contains("The clock has run out."), "{text}");
    assert!(text.contains("A bell rings."), "{text}");

    s.submit(Input::Text("talk to keeper".into()));
    let text = s.choose(0).plain_text();
    assert!(text.contains("The gate hears dialogue:keeper."), "{text}");
    assert!(!text.contains("The keeper opens."), "{text}");
}

#[test]
fn an_end_in_a_dialogue_ends_the_effects_after_it() {
    use loremaster_runtime::{Input, Session};
    let mut s = Session::new(gate_owners_world());
    s.start();
    s.submit(Input::Text("talk to sexton".into()));
    let text = s.choose(0).plain_text();
    assert!(s.state().ended);
    assert!(!text.contains("After the end."), "{text}");
}
