//! Action cost (WO-4), concealed things (WO-2) and the carry limit (WO-3).

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring;
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, Turn, World};
use std::sync::Arc;

const STORY: &str = r#"
title: Search
start: hall
clock: { start: "22:00", minutes_per_turn: 1, costs: { search: 10, wait: 5 } }
carry_limit: 1
"#;

const HALL: &str = "name: Hall\ndescription: [\"A hall.\"]\n";
const RUG: &str = "name: rug\nlocation: { room: hall }\nsupporter: true\n";
const KEY: &str = "name: brass key\naliases: [key]\nlocation: { on: rug }\nportable: true\nprops: { concealed: true }\n";
const COIN: &str = "name: coin\nlocation: { room: hall }\nportable: true\n";
const ENDING: &str = "kind: win\nrank: 1\ntitle: Won\n";
const RULES: &str = r#"
- id: win
  on: { verb: [take, key] }
  when: carried(key)
  then:
    - end: won
"#;

fn files(story: &str) -> Vec<SourceFile> {
    [
        ("story.yaml", story),
        ("rooms/hall.yaml", HALL),
        ("things/rug.yaml", RUG),
        ("things/key.yaml", KEY),
        ("things/coin.yaml", COIN),
        ("endings/won.yaml", ENDING),
        ("rules/rules.yaml", RULES),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into()
}

fn world(story: &str) -> Arc<World> {
    Arc::new(World::compile(load(files(story)).unwrap().story).unwrap())
}

fn game(story: &str) -> Session {
    let mut game = Session::new(world(story));
    authoring::start(&mut game);
    game
}

fn input(session: &mut Session, text: &str) -> String {
    let turn: Turn = authoring::play(session, Input::Text(text.into()));
    turn.plain_text()
}

#[test]
fn the_story_checks_clean() {
    let found = check(&load(files(STORY)).unwrap());
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_verb_with_a_cost_moves_the_clock_by_its_minutes() {
    let mut game = game(STORY);
    let start = game.state().minutes;
    input(&mut game, "look");
    assert_eq!(game.state().minutes, start + 1);
    input(&mut game, "wait");
    assert_eq!(game.state().minutes, start + 6);
    input(&mut game, "search rug");
    assert_eq!(game.state().minutes, start + 16);
    assert_eq!(game.state().turns, 3);
}

#[test]
fn a_concealed_thing_is_out_of_scope_until_a_search_reveals_it() {
    let mut game = game(STORY);
    let look = input(&mut game, "look");
    assert!(!look.contains("brass key"), "{look}");
    let take = input(&mut game, "take key");
    assert!(!take.contains("Taken"), "{take}");
    let found = input(&mut game, "look under rug");
    assert!(found.contains("You find a brass key."), "{found}");
    let again = input(&mut game, "search rug");
    assert!(again.contains("You find nothing."), "{again}");
    let look = input(&mut game, "look");
    assert!(look.contains("brass key"), "{look}");
    input(&mut game, "take key");
    assert!(game.state().ended);
}

#[test]
fn a_rule_reveals_a_concealed_thing() {
    let rules =
        "- id: lift\n  on: { verb: [pull, rug] }\n  then:\n    - set: { key.concealed: false }\n";
    let mut files = files(STORY);
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = rules.into();
    let mut game = Session::new(Arc::new(
        World::compile(load(files).unwrap().story).unwrap(),
    ));
    authoring::start(&mut game);
    input(&mut game, "pull rug");
    let take = input(&mut game, "take key");
    assert!(take.contains("Taken"), "{take}");
}

#[test]
fn the_carry_limit_counts_the_things_in_the_hands() {
    let mut game = game(STORY);
    input(&mut game, "search rug");
    assert_eq!(game.state().locations["key"].to_word(), "on:rug");
    let coin = input(&mut game, "take coin");
    assert!(coin.contains("Taken"), "{coin}");
    let full = input(&mut game, "take rug");
    assert!(full.contains("won't budge"), "{full}");
    let full = input(&mut game, "take key");
    assert!(full.contains("Your hands are full."), "{full}");
    assert!(!game.state().ended);
    input(&mut game, "drop coin");
    input(&mut game, "take key");
    assert!(game.state().ended);
}

#[test]
fn the_explorer_searches_where_a_thing_is_concealed() {
    let found = explore(world(STORY), Budget::default());
    assert_eq!(found.endings.len(), 1, "{found:#?}");
    assert_eq!(found.endings[0].path, ["search rug", "take key"]);
}

#[test]
fn a_cost_for_an_unknown_verb_is_an_error() {
    let story = STORY.replace("search: 10", "dig: 10");
    let found: Vec<String> = check(&load(files(&story)).unwrap())
        .into_iter()
        .map(|d| format!("{} {}: {}", d.file, d.at, d.message))
        .collect();
    assert!(
        found
            .iter()
            .any(|f| f == "story.yaml clock.costs.dig: unknown verb `dig`"),
        "{found:#?}"
    );
}

/// The keeper locks up at 22:20 and the game is lost. The cost of `search`
/// decides whether the player finds the key in time.
fn with_keeper(story: &str) -> Arc<World> {
    let mut files = files(story);
    for (path, text) in [
        (
            "things/keeper.yaml",
            "name: keeper\nlocation: { room: hall }\n",
        ),
        ("endings/lost.yaml", "kind: loss\nrank: 2\ntitle: Lost\n"),
        (
            "agendas/keeper.yaml",
            "actor: keeper\nsteps:\n  - at: \"22:20\"\n    then:\n      - end: lost\n",
        ),
    ] {
        files.push(SourceFile {
            path: path.into(),
            text: text.into(),
        });
    }
    Arc::new(World::compile(load(files).unwrap().story).unwrap())
}

#[test]
fn an_action_cost_can_make_an_agenda_step_come_due() {
    let slow = STORY.replace("search: 10", "search: 30");
    // At play: the search ends at 22:30, past the step, which runs in the
    // same turn.
    let mut game = Session::new(with_keeper(&slow));
    authoring::start(&mut game);
    input(&mut game, "search rug");
    assert!(game.state().ended);
    // The explorer sees the same: with a slow search, no path takes the key.
    let wins = |world| {
        explore(world, Budget::default())
            .endings
            .iter()
            .filter(|e| e.path.last().is_some_and(|c| c == "take key"))
            .count()
    };
    assert_eq!(wins(with_keeper(&slow)), 0);
    // A quick search leaves time to take the key.
    assert_eq!(wins(with_keeper(STORY)), 1);
}

/// The files of [`STORY`] with more files, or other text for some.
fn world_with(story: &str, extra: &[(&str, &str)]) -> Arc<World> {
    let mut files = files(story);
    for (path, text) in extra {
        files.retain(|f| f.path != *path);
        files.push(SourceFile {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    Arc::new(World::compile(load(files).unwrap().story).unwrap())
}

fn play(world: Arc<World>) -> Session {
    let mut game = Session::new(world);
    authoring::start(&mut game);
    game
}

#[test]
fn a_concealed_door_can_not_be_named() {
    let hall = "name: Hall\ndescription: [\"A hall.\"]\nexits:\n  - { direction: north, to: yard, door: hatch }\n";
    let world = world_with(
        STORY,
        &[
            ("rooms/hall.yaml", hall),
            (
                "rooms/yard.yaml",
                "name: Yard\ndescription: [\"A yard.\"]\n",
            ),
            (
                "things/hatch.yaml",
                "name: hatch\nlocation: { room: hall }\nlisted: false\nprops: { open: false, concealed: true }\n",
            ),
        ],
    );
    let mut game = play(world);
    assert!(!game.scene().things.iter().any(|t| t.id == "hatch"));
    let open = input(&mut game, "open hatch");
    assert!(!open.contains("You open"), "{open}");
    // The exit stays, and going through does not name the door.
    let go = input(&mut game, "go north");
    assert_eq!(game.state().room.as_str(), "yard");
    assert!(!go.contains("hatch"), "{go}");
}

#[test]
fn a_locked_concealed_door_names_only_the_way() {
    let hall = "name: Hall\ndescription: [\"A hall.\"]\nexits:\n  - { direction: north, to: yard, door: hatch }\n";
    let mut game = play(world_with(
        STORY,
        &[
            ("rooms/hall.yaml", hall),
            (
                "rooms/yard.yaml",
                "name: Yard\ndescription: [\"A yard.\"]\n",
            ),
            (
                "things/hatch.yaml",
                "name: hatch\nlocation: { room: hall }\nlisted: false\nprops: { open: false, locked: true, concealed: true }\n",
            ),
        ],
    ));
    let go = input(&mut game, "go north");
    assert!(go.contains("The way north is locked."), "{go}");
    assert!(!go.contains("hatch"), "{go}");
    assert_eq!(game.state().room.as_str(), "hall");
}

#[test]
fn the_explorer_takes_with_full_hands_when_a_rule_listens_to_take() {
    // A check rule puts the coin down for any take, so `take gem` works with
    // the coin in hand. The win rule names no thing either.
    let rules = r#"
- id: paid
  on: { verb: [take, coin] }
  then:
    - set: { paid: true }
- id: free_a_hand
  on: { check: [take] }
  when: carried(coin)
  then:
    - move: { coin: hall }
- id: win
  on: { verb: [take] }
  when: paid and carried(gem)
  then:
    - end: won
"#;
    let story = format!("{STORY}vars:\n  - {{ id: paid, type: bool, initial: false }}\n");
    let world = world_with(
        &story,
        &[
            ("rules/rules.yaml", rules),
            (
                "things/gem.yaml",
                "name: gem\nlocation: { room: hall }\nportable: true\n",
            ),
        ],
    );
    let found = explore(world, Budget::default());
    assert_eq!(
        found.endings[0].path,
        ["take coin", "take gem"],
        "{found:#?}"
    );
}

#[test]
fn a_search_finds_what_is_on_a_closed_box_or_in_a_glass_one() {
    let chest = "name: chest\nlocation: { room: hall }\ncontainer: true\nsupporter: true\nprops: { open: false }\n";
    let on = KEY.replace("{ on: rug }", "{ on: chest }");
    let mut game = play(world_with(
        STORY,
        &[("things/chest.yaml", chest), ("things/key.yaml", &on)],
    ));
    let found = input(&mut game, "search chest");
    assert!(found.contains("You find a brass key."), "{found}");

    let glass = chest.replace("open: false", "open: false, transparent: true");
    let inside = KEY.replace("{ on: rug }", "{ in: chest }");
    let mut game = play(world_with(
        STORY,
        &[("things/chest.yaml", &glass), ("things/key.yaml", &inside)],
    ));
    let found = input(&mut game, "search chest");
    assert!(found.contains("You find a brass key."), "{found}");

    let shut = play(world_with(
        STORY,
        &[("things/chest.yaml", chest), ("things/key.yaml", &inside)],
    ));
    let mut game = shut;
    let found = input(&mut game, "search chest");
    assert!(found.contains("The chest is closed."), "{found}");
    // The explorer does not search a closed chest; it opens it first.
    let found = explore(
        world_with(
            STORY,
            &[("things/chest.yaml", chest), ("things/key.yaml", &inside)],
        ),
        Budget::default(),
    );
    assert_eq!(
        found.endings[0].path,
        ["open chest", "search chest", "take key"]
    );
}

#[test]
fn the_explorer_drops_a_thing_to_free_a_hand() {
    // The gem wins only after the coin was taken once; with one hand, the
    // coin must go down first.
    let rules = r#"
- id: paid
  on: { verb: [take, coin] }
  then:
    - set: { paid: true }
- id: win
  on: { verb: [take, gem] }
  when: paid and carried(gem)
  then:
    - end: won
"#;
    let story = format!("{STORY}vars:\n  - {{ id: paid, type: bool, initial: false }}\n");
    let world = world_with(
        &story,
        &[
            ("rules/rules.yaml", rules),
            (
                "things/gem.yaml",
                "name: gem\nlocation: { room: hall }\nportable: true\n",
            ),
        ],
    );
    let found = explore(world, Budget::default());
    assert_eq!(found.endings.len(), 1, "{found:#?}");
    assert_eq!(
        found.endings[0].path,
        ["take coin", "drop coin", "take gem"]
    );
}

#[test]
fn a_cost_for_the_score_verb_checks_clean() {
    let story = format!(
        "{}score: {{ max: 1 }}\n",
        STORY.replace("wait: 5", "score: 0")
    );
    let found = check(&load(files(&story)).unwrap());
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_concealed_thing_in_the_hands_does_not_fill_them() {
    let rules = "- id: slip\n  on: { verb: [pull, rug] }\n  then:\n    - move: { key: carried }\n";
    let mut files = files(STORY);
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = rules.into();
    let mut game = Session::new(Arc::new(
        World::compile(load(files).unwrap().story).unwrap(),
    ));
    authoring::start(&mut game);
    input(&mut game, "pull rug");
    assert_eq!(game.state().locations["key"].to_word(), "carried");
    let coin = input(&mut game, "take coin");
    assert!(coin.contains("Taken"), "{coin}");
}
