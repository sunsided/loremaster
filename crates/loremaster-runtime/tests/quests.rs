//! Quests as machines with `quest: true` and a journal (D8, HG-3, HG-5).

use loremaster_model::Value;
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::{Input, Session, World};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A lock-keeper quest: not started, then searching, then done.
fn files(machine: &str) -> Vec<SourceFile> {
    vec![
        file("story.yaml", "title: Pages\nstart: hall\n"),
        file(
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        file("verbs/ask.yaml", "label: Ask\nwords: [ask]\nslots: none\n"),
        file(
            "verbs/find.yaml",
            "label: Find\nwords: [find]\nslots: none\n",
        ),
        file("machines/pages.yaml", machine),
    ]
}

const QUEST: &str = r#"
quest: true
title: The missing pages
start: unknown
states:
  - unknown
  - { name: searching, journal: "Find the pages of the lock log." }
  - { name: found, journal: "You found the pages.", done: true }
steps:
  - { from: unknown, on: { verb: [ask] }, to: searching }
  - { from: searching, on: { verb: [find] }, to: found }
"#;

fn session(machine: &str) -> Session {
    let world = Arc::new(World::compile(load(files(machine)).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s
}

#[test]
fn a_quest_shows_its_state_in_the_journal() {
    let mut s = session(QUEST);
    assert!(s.journal().is_empty(), "no journal text before it starts");
    s.submit(Input::Text("ask".into()));
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].title, "The missing pages");
    assert_eq!(journal[0].state, "searching");
    assert_eq!(journal[0].text, "Find the pages of the lock log.");
    assert!(!journal[0].done);
    s.submit(Input::Text("find".into()));
    let journal = s.journal();
    assert!(journal[0].done);
    assert_eq!(
        s.state().facts.get("pages"),
        Some(&Value::Text("found".into()))
    );
}

#[test]
fn the_text_journal_lists_the_quests() {
    let mut s = session(QUEST);
    let empty = s.submit(Input::Text("journal".into())).plain_text();
    assert!(empty.contains("Your journal is empty."), "{empty}");
    s.submit(Input::Text("ask".into()));
    let turns = s.state().turns;
    let text = s.submit(Input::Text("journal".into())).plain_text();
    assert!(
        text.contains("The missing pages: Find the pages of the lock log."),
        "{text}"
    );
    assert_eq!(s.state().turns, turns, "reading the journal takes no turn");
}

#[test]
fn a_set_steps_a_quest_too() {
    let mut f = files(QUEST);
    f.push(file(
        "verbs/cheat.yaml",
        "label: Cheat\nwords: [cheat]\nslots: none\n",
    ));
    f.push(file(
        "rules/cheat.yaml",
        "- id: cheat\n  on: { verb: [cheat] }\n  instead: true\n  then: [{ set: { pages: found } }]\n",
    ));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s.submit(Input::Text("cheat".into()));
    assert!(s.journal()[0].done);
}

#[test]
fn the_checks_warn_about_journal_text_outside_a_quest() {
    let warnings = |machine: &str| -> Vec<String> {
        check(&load(files(machine)).unwrap())
            .into_iter()
            .filter(|d| d.severity == Severity::Warning)
            .map(|d| d.message)
            .collect()
    };
    let not_quest = warnings(&QUEST.replace("quest: true\n", ""));
    assert!(
        not_quest
            .iter()
            .any(|m| m.contains("only on a machine with `quest: true`")),
        "{not_quest:?}"
    );
    let silent = warnings(
        "quest: true\nstart: a\nstates: [a, b]\nsteps:\n  - { from: a, on: { verb: [ask] }, to: b }\n",
    );
    assert!(
        silent
            .iter()
            .any(|m| m.contains("no state with `journal:`")),
        "{silent:?}"
    );
    assert!(warnings(QUEST).is_empty(), "{:?}", warnings(QUEST));
}

/// A second quest, finished from the start, declared before `pages`.
const DONE_FIRST: &str = r#"
quest: true
title: The lamp
start: lit
states:
  - { name: lit, journal: "The lamp is lit.", done: true }
"#;

#[test]
fn open_quests_come_before_finished_ones() {
    let mut f = files(QUEST);
    f.insert(0, file("machines/lamp.yaml", DONE_FIRST));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s.submit(Input::Text("ask".into()));
    let order: Vec<_> = s.journal().into_iter().map(|e| e.quest).collect();
    assert_eq!(order, ["pages", "lamp"]);
    let text = s.submit(Input::Text("journal".into())).plain_text();
    assert_eq!(
        text,
        "The missing pages: Find the pages of the lock log.\n\nThe lamp (done): The lamp is lit.",
        "one block per quest"
    );
}

#[test]
fn a_dialogue_steps_a_quest() {
    let mut f = files(QUEST);
    f.push(file(
        "things/keeper.yaml",
        "name: keeper\nlocation: { room: hall }\n",
    ));
    f.push(file(
        "dialogues/keeper.yaml",
        "with: keeper\nstart: [hi]\nnodes:\n  hi:\n    say: \"Yes?\"\n    options:\n      - say: \"The pages?\"\n        then: [{ set: { pages: searching } }]\n        go: end\n",
    ));
    let world = Arc::new(World::compile(load(f).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.start();
    s.submit(Input::Text("talk to keeper".into()));
    s.choose(0);
    assert_eq!(s.journal()[0].state, "searching");
}

#[test]
fn a_gated_out_journal_line_shows_no_entry() {
    let quest = QUEST.replace(
        "journal: \"Find the pages of the lock log.\"",
        "journal: [{ narrate: \"Find the pages.\", when: \"false\" }]",
    );
    let mut s = session(&quest);
    s.submit(Input::Text("ask".into()));
    assert!(s.journal().is_empty());
}

#[test]
fn done_without_journal_text_is_a_warning() {
    let quest = QUEST.replace(
        "{ name: found, journal: \"You found the pages.\", done: true }",
        "{ name: found, done: true }",
    );
    let warnings: Vec<_> = check(&load(files(&quest)).unwrap())
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("`done: true` without `journal:`")),
        "{warnings:?}"
    );
}
