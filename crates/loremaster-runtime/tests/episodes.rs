//! Episodes (TX-12): the first story exports facts, a relation and a memory,
//! and the second story starts with what it imports.

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring;
use loremaster_runtime::{EPISODE_FORMAT, Input, SaveError, Session, World};
use serde_json::json;
use std::sync::Arc;

const ONE: &str = r#"
title: Island One
start: shore
relations: [trust]
vars:
  - { id: gold, type: int, initial: 3 }
  - { id: met, type: bool, initial: false }
  - { id: mood, type: { enum: [calm, angry] }, initial: calm }
export:
  facts: [gold, met, mood]
  relations: [keeper.trust]
  memories: [keeper.asked]
"#;

const DIALOGUE: &str = r#"
with: keeper
start: [hi]
nodes:
  hi:
    say: [{ says: "Evening." }]
    options:
      - { say: "Who are you?", set: { met: true }, relation: { trust: 2 }, remember: [asked], go: end }
"#;

const TWO: &str = r#"
title: Island Two
start: shore
relations: [trust]
vars:
  - { id: gold, type: int, initial: 0, max: 2 }
  - { id: met, type: int, initial: 0 }
import:
  facts: [gold, met]
  relations: [keeper.trust]
  memories: [keeper.asked]
"#;

const SHORE: &str = r#"
name: Shore
description: ['{remembers(keeper, asked) ? "The keeper nods to you." | "A stranger waits."}']
"#;

const KEEPER: &str = "name: keeper\nlocation: { room: shore }\n";

fn files(story: &str, dialogue: bool) -> Vec<SourceFile> {
    let mut files = vec![
        ("story.yaml", story),
        ("rooms/shore.yaml", SHORE),
        ("things/keeper.yaml", KEEPER),
    ];
    if dialogue {
        files.push(("dialogues/keeper.yaml", DIALOGUE));
    }
    files
        .into_iter()
        .map(|(path, text)| SourceFile {
            path: path.into(),
            text: text.into(),
        })
        .collect()
}

fn world(story: &str, dialogue: bool) -> Arc<World> {
    let project = load(files(story, dialogue)).unwrap();
    let found = check(&project);
    assert!(found.is_empty(), "{found:#?}");
    Arc::new(World::compile(project.story).unwrap())
}

fn input(session: &mut Session, text: &str) -> String {
    authoring::play(session, Input::Text(text.into())).plain_text()
}

/// The episode after the player talked to the keeper in the first story.
fn episode() -> serde_json::Value {
    let mut one = Session::new(world(ONE, true));
    authoring::start(&mut one);
    input(&mut one, "talk to keeper");
    one.choose(0);
    serde_json::to_value(one.episode()).unwrap()
}

#[test]
fn the_first_story_writes_what_it_exports() {
    let episode = episode();
    assert_eq!(episode["format"], EPISODE_FORMAT);
    assert_eq!(episode["version"], 1);
    assert_eq!(episode["story"], "island-one");
    assert_eq!(
        episode["facts"],
        json!({ "gold": 3, "met": true, "mood": "calm" })
    );
    assert_eq!(episode["relations"], json!({ "keeper.trust": 2 }));
    assert_eq!(episode["memories"], json!(["keeper.asked"]));
}

#[test]
fn the_next_story_takes_only_what_it_imports() {
    let world = world(TWO, false);
    let (mut two, imported) = Session::from_episode(world, episode()).unwrap();
    assert_eq!(imported.from, "island-one");
    assert_eq!(
        imported.applied,
        ["fact:gold", "relation:keeper.trust", "memory:keeper.asked"]
    );
    // `mood` is not in `import`, and `met` is a number in the second story.
    assert_eq!(imported.skipped, ["fact:mood"]);
    assert_eq!(imported.misfits, ["fact:met"]);
    let state = two.state();
    // The limit of the second story clamps the gold.
    assert_eq!(state.facts["gold"].as_int(), Some(2));
    assert_eq!(state.facts["met"].as_int(), Some(0));
    assert_eq!(state.relations["keeper.trust"], 2);
    let text = authoring::start(&mut two).plain_text();
    assert!(text.contains("The keeper nods to you."), "{text}");
}

#[test]
fn other_data_is_refused() {
    let world = world(TWO, false);
    let mut save = json!({ "format": "loremaster-save", "version": 1 });
    assert!(matches!(
        Session::from_episode(world.clone(), save.clone()),
        Err(SaveError::UnknownFormat(_))
    ));
    save["format"] = json!(EPISODE_FORMAT);
    save["version"] = json!(2);
    assert!(matches!(
        Session::from_episode(world, save),
        Err(SaveError::NewerVersion { .. })
    ));
}

#[test]
fn the_checks_reject_unknown_items() {
    let story = r#"
title: Broken
start: shore
relations: [trust]
vars:
  - { id: gold, type: int, initial: 0 }
  - { id: met, type: bool, initial: false }
export:
  facts: [silver, keeper.trust]
  relations: [keeper.fear, ghost.trust, trust]
  memories: [keeper.forgot, gull.asked, keeper.asked]
import:
  facts: [gold]
  memories: [ghost.asked]
"#;
    let mut files = files(story, true);
    files.push(SourceFile {
        path: "things/gull.yaml".into(),
        text: "name: gull\nlocation: { room: shore }\n".into(),
    });
    let found = check(&load(files).unwrap());
    let found: Vec<_> = found
        .iter()
        .map(|d| format!("{}: {}", d.at, d.message))
        .collect();
    assert_eq!(
        found,
        [
            "export.facts[0]: `silver` is not a story variable or a thing property",
            "export.facts[1]: `keeper.trust` is not a story variable or a thing property",
            "export.relations[0]: unknown relation axis `fear`",
            "export.relations[1]: unknown thing `ghost`",
            "export.relations[2]: `trust` is not `thing.axis`",
            "export.memories[0]: no dialogue of `keeper` remembers `forgot`",
            // Only the keeper's dialogue gives `asked`.
            "export.memories[1]: no dialogue of `gull` remembers `asked`",
            "import.memories[0]: unknown thing `ghost`",
        ]
    );
}

#[test]
fn a_limit_reads_the_imported_value_of_its_variable() {
    let story = r#"
title: Island Two
start: shore
vars:
  - { id: gold, type: int, initial: 0, max: purse }
  - { id: purse, type: int, initial: 1 }
import:
  facts: [gold, purse]
  memories: [keeper.asked]
"#;
    let world = world(story, false);
    let episode = json!({
        "format": EPISODE_FORMAT, "version": 1, "story": "island-one",
        "facts": { "gold": 3, "purse": 5 },
    });
    let (two, _) = Session::from_episode(world, episode).unwrap();
    // `purse` sorts after `gold`, but the clamp uses its imported 5, not 1.
    assert_eq!(two.state().facts["gold"].as_int(), Some(3));
}

#[test]
fn a_chain_of_limits_settles() {
    let story = r#"
title: Island Two
start: shore
vars:
  - { id: gold, type: int, initial: 0, max: purse }
  - { id: purse, type: int, initial: 1, max: cap }
  - { id: cap, type: int, initial: 1 }
import:
  facts: [gold, purse, cap]
  memories: [keeper.asked]
"#;
    let world = world(story, false);
    let episode = json!({
        "format": EPISODE_FORMAT, "version": 1, "story": "island-one",
        "facts": { "gold": 9, "purse": 8, "cap": 4 },
    });
    let (two, _) = Session::from_episode(world, episode).unwrap();
    assert_eq!(two.state().facts["purse"].as_int(), Some(4));
    assert_eq!(two.state().facts["gold"].as_int(), Some(4));
}

#[test]
fn a_huge_imported_relation_saturates() {
    let story = format!("{ONE}import:\n  relations: [keeper.trust]\n");
    let world = world(&story, true);
    let episode = json!({
        "format": EPISODE_FORMAT, "version": 1, "story": "island-one",
        "relations": { "keeper.trust": i32::MAX },
    });
    let (mut two, _) = Session::from_episode(world, episode).unwrap();
    authoring::start(&mut two);
    input(&mut two, "talk to keeper");
    two.choose(0);
    assert_eq!(two.state().relations["keeper.trust"], i32::MAX);
}
