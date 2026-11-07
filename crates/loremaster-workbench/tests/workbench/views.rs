//! Edge cases of the views: undo of author commands, the map's paired
//! exits, and a story that the checks did not see.

use crate::{Host, world_of};
use loremaster_model::Value;
use loremaster_project::{SourceFile, load};
use loremaster_runtime::World;
use loremaster_workbench::{AuthorCommand, Workbench};
use std::sync::Arc;

const STORY: &str =
    "title: Yard\nstart: hall\nvars:\n  - { id: rang, type: bool, initial: false }\n";
const HALL: &str = "name: Hall\ndescription:\n  - \"A hall.\"\nexits:\n  - { direction: north, to: yard, visible: \"rang\" }\n";
const YARD: &str =
    "name: Yard\ndescription:\n  - \"A yard.\"\nexits:\n  - { direction: south, to: hall }\n";

fn bench() -> Workbench {
    let mut bench = Workbench::new(world_of(&[
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", YARD),
    ]));
    bench.start();
    bench
}

#[test]
fn undo_goes_back_over_an_author_command() {
    let mut bench = bench();
    bench.input("wait");
    bench.author(AuthorCommand::Set {
        name: "rang".into(),
        value: Value::Bool(true),
    });
    assert!(bench.state().flag("rang"));
    bench.session_mut().undo();
    // Only the author command is undone, not the turn before it.
    assert!(!bench.state().flag("rang"));
    assert_eq!(bench.state().turns, 1);
}

#[test]
fn a_pair_of_exits_is_one_link_that_shows_when_either_exit_shows() {
    let bench = bench();
    let map = bench.map();
    let links: Vec<_> = map
        .links
        .iter()
        .filter(|l| (l.from == "hall") != (l.to == "hall"))
        .collect();
    assert_eq!(links.len(), 1, "{:?}", map.links);
    // hall to yard is hidden until `rang`, yard to hall shows.
    assert!(links[0].visible && !links[0].one_way);
}

#[test]
fn a_verb_rule_without_a_verb_does_not_break_the_rules_view() {
    // `loremaster check` rejects `verb: []`; World::compile does not run the checks.
    let files = [
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", YARD),
        (
            "rules/odd.yaml",
            "- id: odd\n  on: { verb: [] }\n  then: [{ say: \"Odd.\" }]\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let world = World::compile(load(files.into()).unwrap().story).unwrap();
    let bench = Workbench::new(Arc::new(world));
    let rules = bench.rules();
    assert!(rules.iter().any(|r| r.id == "odd"));
}

#[test]
fn the_rules_view_shows_a_line_as_it_reads() {
    let files = [
        ("story.yaml", STORY),
        ("rooms/hall.yaml", HALL),
        ("rooms/yard.yaml", YARD),
        (
            "rules/talk.yaml",
            "- id: plain\n  on: { verb: [wait] }\n  then: [{ say: \"Odd.\" }]\n- id: speech\n  on: { verb: [wait] }\n  then: [{ say: [\"He nods.\", { says: \"Yes?\" }] }]\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let world = World::compile(load(files.into()).unwrap().story).unwrap();
    let bench = Workbench::new(Arc::new(world));
    let rules = bench.rules();
    let then = |id: &str| rules.iter().find(|r| r.id == id).unwrap().then.clone();
    assert_eq!(then("plain"), ["say \"Odd.\""]);
    assert_eq!(then("speech"), ["say He nods. \"Yes?\""]);
}
