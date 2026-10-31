//! Property tests for the puzzle logic: random places of things, and random
//! command lists on the examples. Whatever the player types, the game does
//! not hang or panic, replays the same way, survives a save, and keeps its
//! invariants: places end in a room, carried or nowhere; timers only count
//! down; machines stay in one of their states.

#[path = "../../../examples/zork1/thief.rs"]
mod thief;

use loremaster_model::{Id, Location};
use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{Game, State, World};
use proptest::prelude::*;
use std::path::Path;
use std::sync::{Arc, OnceLock};

// ---- worlds -----------------------------------------------------------------

fn example(name: &str) -> World {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    let project = load(read_dir(&root).unwrap()).unwrap_or_else(|e| panic!("{e:#?}"));
    World::compile(project.story).unwrap()
}

/// Zork I with the thief, compiled once for all cases.
fn zork() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD
        .get_or_init(|| {
            let mut w = example("zork1");
            w.register(thief::Thief::zork());
            Arc::new(w)
        })
        .clone()
}

/// A small story with containers, a supporter, a timer and a machine.
fn museum() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD
        .get_or_init(|| {
            let files = [
                (
                    "story.yaml",
                    "title: Museum\nstart: hall\ntimers:\n  - { id: guard, turns: 5, running: true, repeat: true }\n",
                ),
                (
                    "rooms/hall.yaml",
                    "name: Hall\ndescription: [\"A hall.\"]\nexits:\n  - { direction: east, to: annex }\n",
                ),
                (
                    "rooms/annex.yaml",
                    "name: Annex\ndescription: [\"An annex.\"]\nexits:\n  - { direction: west, to: hall }\n",
                ),
                (
                    "things/case.yaml",
                    "name: case\nlocation: { room: hall }\ncontainer: true\ncapacity: 2\nprops: { open: false, transparent: true }\n",
                ),
                (
                    "things/box.yaml",
                    "name: box\nlocation: { room: hall }\nportable: true\ncontainer: true\nprops: { open: true }\n",
                ),
                (
                    "things/bag.yaml",
                    "name: bag\nlocation: { in: box }\nportable: true\ncontainer: true\nprops: { open: true }\n",
                ),
                ("things/table.yaml", "name: table\nlocation: { room: annex }\nsupporter: true\n"),
                ("things/coin.yaml", "name: coin\nlocation: { on: table }\nportable: true\n"),
                ("things/gem.yaml", "name: gem\nlocation: { in: bag }\nportable: true\n"),
                (
                    "machines/alarm.yaml",
                    "start: quiet\nsteps:\n  - { from: quiet, on: { verb: [put, gem, case] }, to: ringing }\n  - { from: ringing, on: { timer: guard }, to: quiet }\n  - { from: [quiet, ringing], on: { verb: [take, gem] }, to: quiet }\n",
                ),
            ]
            .map(|(path, text)| SourceFile {
                path: path.into(),
                text: text.into(),
            });
            let project = load(files.into()).unwrap();
            Arc::new(World::compile(project.story).unwrap())
        })
        .clone()
}

// ---- commands ---------------------------------------------------------------

const MUSEUM_COMMANDS: &[&str] = &[
    "e",
    "w",
    "wait",
    "look",
    "i",
    "take box",
    "take bag",
    "take gem",
    "take coin",
    "drop box",
    "drop bag",
    "drop gem",
    "drop coin",
    "open case",
    "close case",
    "open box",
    "close box",
    "open bag",
    "close bag",
    "put gem in case",
    "put gem in bag",
    "put bag in box",
    "put box in bag",
    "put coin on table",
    "put bag in case",
    "put box in case",
    "put gem in gem",
    "x case",
    "x box",
    "score",
];

const ZORK_COMMANDS: &[&str] = &[
    "n",
    "s",
    "e",
    "w",
    "u",
    "d",
    "ne",
    "nw",
    "se",
    "sw",
    "in",
    "out",
    "wait",
    "look",
    "i",
    "open mailbox",
    "take leaflet",
    "open window",
    "go to window",
    "take lamp",
    "turn on lamp",
    "turn off lamp",
    "open case",
    "put leaflet in case",
    "move rug",
    "open trap door",
    "take sword",
    "take all",
    "ring bell",
    "read book",
    "turn bolt with wrench",
    "push yellow button",
    "score",
];

fn commands(list: &'static [&'static str], max: usize) -> impl Strategy<Value = Vec<&'static str>> {
    prop::collection::vec(prop::sample::select(list), 0..max)
}

/// Play the commands from the start. Returns the transcript and the game.
fn play(world: &Arc<World>, cmds: &[&str]) -> (String, Game) {
    let mut game = Game::new(world.clone());
    let mut out = game.start().plain_text();
    for c in cmds {
        out.push_str(&game.input(c).plain_text());
        out.push('\n');
        check_invariants(&game);
    }
    (out, game)
}

fn check_invariants(game: &Game) {
    let state = game.state();
    let story = &game.world().story;
    for id in state.locations.keys() {
        // Every place ends in a room, carried or nowhere: no loop of holders.
        let outer = state.outer_location(id.as_str());
        assert!(
            matches!(
                outer,
                Some(Location::Room(_) | Location::Carried | Location::Nowhere)
            ),
            "{id} has no outer place: {:?}",
            state.locations.get(id)
        );
    }
    for t in &story.timers {
        if let Some(left) = state.timers.get(&t.id) {
            assert!(*left <= t.turns, "timer {} at {left} of {}", t.id, t.turns);
        }
    }
    for m in &story.machines {
        let now = state.facts.get(m.id.as_str()).and_then(|v| v.as_text());
        assert!(
            now.is_some_and(|s| m.all_states().iter().any(|x| x == s)),
            "machine {} in {now:?}",
            m.id
        );
    }
}

// ---- properties -------------------------------------------------------------

/// A place for thing `i` of `n`: a room, carried, nowhere, or in or on another thing.
fn place(n: usize) -> impl Strategy<Value = Location> {
    prop_oneof![
        (0..2usize).prop_map(|r| Location::Room(Id::new(format!("r{r}")))),
        Just(Location::Carried),
        Just(Location::Nowhere),
        (0..n).prop_map(|t| Location::In(Id::new(format!("t{t}")))),
        (0..n).prop_map(|t| Location::On(Id::new(format!("t{t}")))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Any places, loops of holders too: the walks up the holders end, and they agree.
    #[test]
    fn walks_up_the_holders_end_and_agree(places in prop::collection::vec(place(6), 1..6)) {
        let mut state: State = Game::new(museum()).state().clone();
        state.locations = places
            .iter()
            .enumerate()
            .map(|(i, p)| (Id::new(format!("t{i}")), p.clone()))
            .collect();
        for i in 0..places.len() {
            let id = format!("t{i}");
            let outer = state.outer_location(&id).cloned();
            prop_assert!(outer.as_ref().is_none_or(|l| l.holder().is_none()));
            prop_assert_eq!(state.is_carried(&id), outer == Some(Location::Carried));
            prop_assert_eq!(state.is_in_room(&id, "r0"), outer == Some(Location::Room(Id::new("r0"))));
            let _ = state.closed_around(&id, true);
            let _ = state.closed_around(&id, false);
            for c in state.contents(&id) {
                prop_assert_eq!(state.locations[c].holder().map(Id::as_str), Some(id.as_str()));
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Containers, a timer and a machine under random commands: no holder loop, timers in
    /// range, machine in a state; the same commands give the same text.
    #[test]
    fn the_museum_keeps_its_invariants_and_replays(cmds in commands(MUSEUM_COMMANDS, 40)) {
        let world = museum();
        let (a, game) = play(&world, &cmds);
        let (b, again) = play(&world, &cmds);
        prop_assert_eq!(a, b);
        prop_assert_eq!(game.state(), again.state());
    }

    /// A save in the middle of a game goes on exactly like the game it saved, the thief included.
    #[test]
    fn a_saved_zork_game_goes_on_like_the_game_it_saved(
        cmds in commands(ZORK_COMMANDS, 30),
        split in 0usize..30,
    ) {
        let world = zork();
        let split = split.min(cmds.len());
        let (_, mut game) = play(&world, &cmds[..split]);
        let save = serde_json::to_string(game.state()).unwrap();
        let mut restored = Game::restore(world.clone(), serde_json::from_str(&save).unwrap());
        prop_assert_eq!(restored.state(), game.state());
        for c in &cmds[split..] {
            prop_assert_eq!(game.input(c).plain_text(), restored.input(c).plain_text());
            check_invariants(&restored);
        }
        prop_assert_eq!(game.state(), restored.state());
    }

    /// The lamp battery of Zork only counts down: no command adds turns to it.
    #[test]
    fn the_lamp_battery_only_counts_down(cmds in commands(ZORK_COMMANDS, 40)) {
        let mut game = Game::new(zork());
        game.start();
        let mut last = game.state().timers.get("lamp_battery").copied().unwrap_or(0);
        for c in &cmds {
            game.input(c);
            let now = game.state().timers.get("lamp_battery").copied().unwrap_or(0);
            prop_assert!(now <= last, "{c}: {last} -> {now}");
            last = now;
        }
    }
}
