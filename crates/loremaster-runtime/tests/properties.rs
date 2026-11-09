//! Property tests for the puzzle logic: random places of things, and random
//! command lists on the examples. Whatever the player types, the game does
//! not hang or panic, replays the same way, survives a save, and keeps its
//! invariants: places end in a room, carried or nowhere; timers only count
//! down; machines stay in one of their states.
//!
//! Also: a save at any await of a combat or a roll test restores; a variable
//! with limits stays in them; the dice, sum, condition and markup parsers
//! read what they should and refuse the rest at a span in the text; and the
//! YAML model of each example survives an export and a load.

#[path = "../../../examples/zork1/thief.rs"]
mod thief;

use loremaster_lang::{
    Scope, compile_condition, compile_markup, condition_tokens, markup_tokens, parse_expr,
};
use loremaster_model::{Dice, Id, Location, Term, Value, parse_sum};
use loremaster_project::{Pack, SourceFile, load, model_hash, read_dir};
use loremaster_runtime::authoring;
use loremaster_runtime::{AwaitRequest, Input, Session, State, Status, Turn, World};
use proptest::prelude::*;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// One typed command as a whole turn.
fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

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
fn play(world: &Arc<World>, cmds: &[&str]) -> (String, Session) {
    let mut game = Session::new(world.clone());
    let mut out = authoring::start(&mut game).plain_text();
    for c in cmds {
        out.push_str(&input(&mut game, c).plain_text());
        out.push('\n');
        check_invariants(&game);
    }
    (out, game)
}

fn check_invariants(game: &Session) {
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
        let mut state: State = Session::new(museum()).state().clone();
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
        let mut restored = authoring::session_of(world.clone(), serde_json::from_str(&save).unwrap());
        prop_assert_eq!(restored.state(), game.state());
        for c in &cmds[split..] {
            prop_assert_eq!(input(&mut game, c).plain_text(), input(&mut restored, c).plain_text());
            check_invariants(&restored);
        }
        prop_assert_eq!(game.state(), restored.state());
    }

    /// The lamp battery of Zork only counts down: no command adds turns to it.
    #[test]
    fn the_lamp_battery_only_counts_down(cmds in commands(ZORK_COMMANDS, 40)) {
        let mut game = Session::new(zork());
        authoring::start(&mut game);
        let mut last = game.state().timers.get("lamp_battery").copied().unwrap_or(0);
        for c in &cmds {
            input(&mut game, c);
            let now = game.state().timers.get("lamp_battery").copied().unwrap_or(0);
            prop_assert!(now <= last, "{c}: {last} -> {now}");
            last = now;
        }
    }
}

// ---- combat, rolls and `on: next` chains ------------------------------------

/// The Salt Tower: a combat, Skill and Luck tests (an `await` of a roll, then
/// `on: next` steps) and stats with limits, compiled once.
fn salt_tower() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD
        .get_or_init(|| Arc::new(example("salt-tower")))
        .clone()
}

const SALT_COMMANDS: &[&str] = &[
    "roll your character",
    "ring the bell",
    "refuse",
    "pay 3 gold",
    "open the grate",
    "crawl on",
    "test your skill",
    "test your luck",
    "take brass lantern",
    "climb the stairs",
    "say the word hearth",
    "show the letter",
    "look",
    "wait",
];

/// One move of a pick: answer the await (a choice, or a roll total in range
/// when the host rolls), or else type a command. Nothing after the end.
fn salt_move(game: &mut Session, pick: usize) -> String {
    let awaiting = match game.status() {
        Status::Ended { .. } => return String::new(),
        Status::Await { awaiting } => awaiting,
        _ => {
            let cmd = SALT_COMMANDS[pick % SALT_COMMANDS.len()];
            return game.submit(Input::Text(cmd.into())).plain_text();
        }
    };
    // Any kind of await: a total in range for a roll, else one of its answers.
    let answer = match &awaiting.request {
        AwaitRequest::Roll { min, max, .. } => {
            let span = i128::from(*max) - i128::from(*min) + 1;
            assert!(span > 0, "a roll from {min} to {max}");
            Value::Int((i128::from(*min) + pick as i128 % span) as i64)
        }
        request => {
            let answers = request.answers();
            assert!(!answers.is_empty(), "no answer to offer: {request:?}");
            answers[pick % answers.len()].0.clone()
        }
    };
    game.resume(&awaiting.id, answer).unwrap().plain_text()
}

/// The stats stay within their `min` and `max`; the `max` names another variable.
fn check_salt_limits(game: &Session) {
    let int = |name: &str| {
        game.state()
            .facts
            .get(name)
            .and_then(Value::as_int)
            .unwrap()
    };
    for (stat, cap) in [
        ("skill", "skill_initial"),
        ("stamina", "stamina_initial"),
        ("luck", "luck_initial"),
    ] {
        let (n, cap) = (int(stat), int(cap));
        assert!((0..=cap.max(0)).contains(&n), "{stat} = {n} of {cap}");
    }
    assert!(int("gold") >= 0, "gold = {}", int("gold"));
}

fn salt_session(host_rolls: bool, seed: u64) -> Session {
    let mut game = Session::with_seed(salt_tower(), seed);
    game.set_host_rolls(host_rolls);
    game.start();
    game
}

/// A story whose start rule makes the given writes, then adds 0 to both
/// limited variables, so that their last write comes after every change of `cap`.
fn limited(writes: &[String]) -> World {
    let mut then: String = writes.iter().map(|w| format!("    - {w}\n")).collect();
    then.push_str("    - add: { hp: 0, st: 0 }\n");
    let files = [
        (
            "story.yaml",
            "title: Limits\nstart: hall\nvars:\n  - { id: hp, type: int, initial: 5, min: -3, max: 10 }\n  - { id: cap, type: int, initial: 4 }\n  - { id: st, type: int, initial: 2, min: 0, max: cap }\n"
                .to_string(),
        ),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n".to_string(),
        ),
        (
            "rules/start.yaml",
            format!("- id: writes\n  on: start\n  then:\n{then}"),
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text,
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    World::compile(project.story).unwrap()
}

const LIMITED: &[&str] = &["hp", "st", "cap"];

/// A write to `hp`, `st` or `cap`: `set`, `add` (of any `i64` too) or `calc`.
fn write() -> impl Strategy<Value = String> {
    let var = || prop::sample::select(LIMITED);
    prop_oneof![
        (var(), -20i64..20).prop_map(|(v, n)| format!("set: {{ {v}: {n} }}")),
        (var(), any::<i64>()).prop_map(|(v, n)| format!("add: {{ {v}: {n} }}")),
        (var(), -20i64..20).prop_map(|(v, n)| format!("add: {{ {v}: {n} }}")),
        (
            var(),
            var(),
            prop::sample::select(&["+", "-"][..]),
            0i64..20
        )
            .prop_map(|(v, w, op, n)| format!("calc: {{ {v}: \"{w} {op} {n}\" }}")),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// A save at any point of the Salt Tower, at the await of a combat round or
    /// a roll test too, goes on exactly like the game it saved; the stats stay
    /// in their limits.
    #[test]
    fn a_saved_salt_tower_game_goes_on_like_the_game_it_saved(
        picks in prop::collection::vec(0usize..64, 0..40),
        split in 0usize..40,
        host_rolls in any::<bool>(),
        seed in any::<u64>(),
    ) {
        let mut game = salt_session(host_rolls, seed);
        let split = split.min(picks.len());
        for &p in &picks[..split] {
            salt_move(&mut game, p);
            check_salt_limits(&game);
        }
        let save = serde_json::to_value(game.save()).unwrap();
        let (mut restored, _) = Session::load(salt_tower(), save).unwrap();
        restored.set_host_rolls(host_rolls);
        prop_assert_eq!(restored.status(), game.status());
        for &p in &picks[split..] {
            prop_assert_eq!(salt_move(&mut game, p), salt_move(&mut restored, p));
            check_salt_limits(&restored);
        }
        prop_assert_eq!(game.state(), restored.state());
    }

    /// The same seed and moves give the same game.
    #[test]
    fn the_salt_tower_replays(picks in prop::collection::vec(0usize..64, 0..40), seed in any::<u64>()) {
        let (mut a, mut b) = (salt_session(false, seed), salt_session(false, seed));
        for &p in &picks {
            prop_assert_eq!(salt_move(&mut a, p), salt_move(&mut b, p));
        }
        prop_assert_eq!(a.state(), b.state());
    }

    /// Any writes, huge `add`s too: a variable with `min` and `max` ends in its
    /// range, also when a limit is another variable (when the limits cross, `min` wins).
    #[test]
    fn a_limited_variable_stays_in_range(writes in prop::collection::vec(write(), 0..12)) {
        let mut game = Session::new(Arc::new(limited(&writes)));
        authoring::start(&mut game);
        let int = |name: &str| game.state().facts.get(name).and_then(Value::as_int).unwrap();
        prop_assert!((-3..=10).contains(&int("hp")), "hp = {}", int("hp"));
        let (st, cap) = (int("st"), int("cap"));
        prop_assert!(st >= 0 && (st <= cap || st == 0), "st = {st}, cap = {cap}");
    }
}

/// Two huge `add`s once overflowed (a panic in a debug build); now they saturate.
#[test]
fn a_huge_add_saturates() {
    let max = format!("add: {{ cap: {} }}", i64::MAX);
    let min = format!("add: {{ cap: {} }}", i64::MIN);
    for writes in [[max.clone(), max], [min.clone(), min]] {
        let mut game = Session::new(Arc::new(limited(&writes)));
        authoring::start(&mut game);
        let cap = game.state().facts["cap"].as_int().unwrap();
        assert!(cap == i64::MAX || cap == i64::MIN, "cap = {cap}");
    }
}

// ---- parsers ----------------------------------------------------------------

/// A span of an error or a token lies in the text, on char boundaries.
fn in_text(src: &str, span: &std::ops::Range<usize>) -> bool {
    src.get(span.clone()).is_some()
}

/// A valid condition over the names of the museum.
fn condition() -> impl Strategy<Value = String> {
    let atom = prop::sample::select(
        &[
            "true",
            "false",
            "case.open",
            "box.open",
            "visited(hall)",
            "carried(gem)",
            "alarm == quiet",
            "turns >= 3",
        ][..],
    )
    .prop_map(str::to_string);
    atom.prop_recursive(4, 16, 2, |inner| {
        prop_oneof![
            inner.clone().prop_map(|a| format!("not {a}")),
            inner.clone().prop_map(|a| format!("({a})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} and {b}")),
            (inner.clone(), inner).prop_map(|(a, b)| format!("{a} or {b}")),
        ]
    })
}

/// Text of the pieces that matter to the parsers, and any other characters.
fn noise() -> impl Strategy<Value = String> {
    let piece = prop_oneof![
        prop::sample::select(
            &[
                "{",
                "}",
                "{{",
                "}}",
                "|",
                ":",
                "?",
                "*",
                "(",
                ")",
                ".",
                ",",
                "\"",
                "'",
                "=",
                "!",
                "<",
                ">",
                "-",
                "+",
                "d",
                "0",
                "9",
                " ",
                "and",
                "or",
                "not",
                "first",
                "case.open",
                "night",
                "visited",
                "ä",
                "€",
                "\u{1F600}",
            ][..]
        )
        .prop_map(str::to_string),
        any::<char>().prop_map(String::from),
    ];
    prop::collection::vec(piece, 0..24).prop_map(|parts| parts.concat())
}

/// A term of a sum, as `parse_sum` reads it.
fn term() -> impl Strategy<Value = Term> {
    prop_oneof![
        (0i64..=i64::MAX).prop_map(Term::Int),
        "[a-z_][a-z0-9_]{0,6}(\\.[a-z_]{1,4})?".prop_map(Term::Name),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Dice within the limits read back as written; any text is read or refused.
    #[test]
    fn dice_round_trip(
        count in 1u32..=Dice::MAX_COUNT,
        sides in 1u32..=Dice::MAX_SIDES,
        bonus in -Dice::MAX_BONUS..=Dice::MAX_BONUS,
        junk in noise(),
    ) {
        let text = match bonus {
            0 => format!("{count}d{sides}"),
            b => format!(" {count}D{sides}{b:+} "),
        };
        prop_assert_eq!(Dice::parse(&text), Some(Dice { count, sides, bonus }));
        // Text that reads as dice reads the same once written plainly.
        if let Some(d) = Dice::parse(&junk) {
            let plain = format!("{}d{}{:+}", d.count, d.sides, d.bonus);
            prop_assert_eq!(Dice::parse(&plain), Some(d), "{:?}", junk);
        }
    }

    /// A sum reads back its terms and signs; a bad sum names a part of its text.
    #[test]
    fn sums_round_trip(terms in prop::collection::vec((any::<bool>(), term()), 1..6), junk in noise()) {
        let mut text = String::new();
        for (i, (neg, term)) in terms.iter().enumerate() {
            match (i, neg) {
                (0, true) => text.push('-'),
                (0, false) => {}
                (_, true) => text.push_str(" - "),
                (_, false) => text.push_str(" + "),
            }
            match term {
                Term::Int(n) => text.push_str(&n.to_string()),
                Term::Name(n) => text.push_str(n),
            }
        }
        let expected: Vec<_> = terms.iter().map(|(neg, t)| (if *neg { -1 } else { 1 }, t.clone())).collect();
        prop_assert_eq!(parse_sum(&text), Ok(expected));
        if let Err((_, span)) = parse_sum(&junk) {
            prop_assert!(in_text(&junk, &span), "{junk:?}: {span:?}");
        }
    }

    /// A valid condition compiles; any text compiles or is refused at a span in it.
    #[test]
    fn conditions_compile_or_refuse_cleanly(good in condition(), junk in noise()) {
        let scope = Scope::from_story(&museum().story);
        let compiled = compile_condition(&good, &scope);
        prop_assert!(compiled.is_ok(), "{good}: {compiled:?}");
        for src in [junk.clone(), format!("{good} {junk}")] {
            if let Err(e) = parse_expr(&src) {
                prop_assert!(in_text(&src, &e.span), "{src:?}: {e:?}");
            }
            if let Err(e) = compile_condition(&src, &scope) {
                prop_assert!(in_text(&src, &e.span), "{src:?}: {e:?}");
            }
            for t in condition_tokens(&src) {
                prop_assert!(in_text(&src, &t.span), "{src:?}: {t:?}");
            }
        }
    }

    /// Markup compiles or is refused at a span in it, and its tokens lie in it.
    #[test]
    fn markup_compiles_or_refuses_cleanly(good in condition(), junk in noise()) {
        let scope = Scope::from_story(&museum().story);
        let valid = format!("A {{{good}: lit | dark}} room{{first: , new,}}.");
        let compiled = compile_markup(&valid, &scope);
        prop_assert!(compiled.is_ok(), "{valid}: {compiled:?}");
        for src in [junk.clone(), format!("{valid}{junk}")] {
            if let Err(e) = compile_markup(&src, &scope) {
                prop_assert!(in_text(&src, &e.span), "{src:?}: {e:?}");
            }
            for t in markup_tokens(&src) {
                prop_assert!(in_text(&src, &t.span), "{src:?}: {t:?}");
            }
        }
    }
}

// ---- the YAML model ---------------------------------------------------------

/// The story folders of the examples (with a `story.yaml`, at most two levels down), by
/// their path below `examples/`.
fn story_folders(root: &Path, depth: u8) -> Vec<String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        if path.join("story.yaml").exists() {
            out.push(path.file_name().unwrap().to_string_lossy().into_owned());
        } else if depth > 1 {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.extend(
                story_folders(&path, depth - 1)
                    .into_iter()
                    .map(|d| format!("{name}/{d}")),
            );
        }
    }
    out.sort();
    out
}

/// Each example story as one pack, exported to YAML and loaded again, gives
/// the same story by `model_hash`.
#[test]
fn every_example_round_trips_through_an_exported_pack() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let folders = story_folders(&root, 2);
    assert!(folders.iter().any(|f| f == "lighthouse"), "{folders:?}");
    for name in &folders {
        let dir = root.join(name);
        let files = read_dir(&dir).unwrap();
        let direct = load(files.clone()).unwrap_or_else(|e| panic!("{name}: {e:#?}"));
        let mut pack = Pack::from_files(files).unwrap_or_else(|e| panic!("{name}: {e:#?}"));
        pack.name = "export".into();
        let exported = vec![SourceFile {
            path: "packs/export.yaml".into(),
            text: pack.to_yaml(),
        }];
        let again = load(exported).unwrap_or_else(|e| panic!("{name}: {e:#?}"));
        assert_eq!(
            model_hash(&again.story),
            model_hash(&direct.story),
            "{name}"
        );
    }
}
