//! Queries for graphical hosts (#194): links on the room after a restore,
//! the aliases of a thing, one fact without a copy of the state, and the
//! clock as a number.

use loremaster_model::Value;
use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Effect, Input, LinkKind, Session, Step, World};
use std::sync::Arc;

fn lockhouse() -> Arc<World> {
    let files = [
        (
            "story.yaml",
            "title: Lockhouse\nstart: kitchen\nclock:\n  start: \"08:30\"\n",
        ),
        (
            "rooms/kitchen.yaml",
            "name: Kitchen\ndescription: [\"Hester sits by the stove.\"]\n",
        ),
        (
            "things/hester.yaml",
            "name: Hester Reed\naliases: [hester, daughter]\nproper: true\nlocation: { room: kitchen }\n",
        ),
        (
            "things/boots.yaml",
            "name: pair of boots\nhead: boots\nnames:\n  - { when: lock == empty, text: boots of Amos, head: boots }\nlocation: { room: kitchen }\n",
        ),
        (
            "machines/lock.yaml",
            "start: full\nsteps:\n  - { from: full, on: { verb: [examine, hester] }, to: empty }\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    Arc::new(World::compile(project.story).unwrap())
}

/// The ids of the things that the text of a step links.
fn linked(step: &Step) -> Vec<String> {
    (step.effects.iter())
        .filter_map(|e| match e {
            Effect::Text { block } => Some(block),
            _ => None,
        })
        .flat_map(|b| b.runs.iter())
        .filter_map(|r| r.link.as_ref())
        .filter(|l| l.kind == LinkKind::Thing)
        .map(|l| l.id.clone())
        .collect()
}

#[test]
fn the_room_after_a_restore_or_an_undo_has_its_links() {
    let mut s = Session::new(lockhouse());
    let opening = s.start();
    assert!(linked(&opening).contains(&"hester".to_string()));
    let snapshot = s.snapshot();
    s.submit(Input::Text("wait".into()));
    let restored = s.restore(&snapshot).unwrap();
    assert!(
        linked(&restored).contains(&"hester".to_string()),
        "{restored:#?}"
    );
    s.submit(Input::Text("wait".into()));
    let undone = s.undo();
    assert!(linked(&undone).contains(&"hester".to_string()));
}

#[test]
fn a_thing_in_the_scene_has_its_aliases() {
    let mut s = Session::new(lockhouse());
    s.start();
    let scene = s.scene();
    let hester = scene.things.iter().find(|t| t.id == "hester").unwrap();
    assert_eq!(hester.name, "Hester Reed");
    assert_eq!(hester.aliases, ["hester", "daughter"]);
}

#[test]
fn a_host_reads_one_fact_and_a_machine_state() {
    let mut s = Session::new(lockhouse());
    s.start();
    assert_eq!(s.fact("lock"), Some(&Value::Text("full".into())));
    s.submit(Input::Text("examine hester".into()));
    assert_eq!(s.fact("lock"), Some(&Value::Text("empty".into())));
    assert_eq!(s.fact("nothing"), None);
}

#[test]
fn the_scene_has_the_clock_as_a_number() {
    let mut s = Session::new(lockhouse());
    s.start();
    let scene = s.scene();
    assert_eq!(scene.time, "08:30");
    assert_eq!(scene.minutes, 8 * 60 + 30);
}

#[test]
fn a_thing_has_its_head_noun() {
    let mut s = Session::new(lockhouse());
    s.start();
    let head = |s: &Session, id: &str| s.thing(id).unwrap().head;
    // The last word of the name, and the author's head.
    assert_eq!(head(&s, "hester"), "Reed");
    assert_eq!(head(&s, "boots"), "boots");
    // The head of the name stage that holds now.
    s.submit(Input::Text("examine hester".into()));
    assert_eq!(s.thing("boots").unwrap().name, "boots of Amos");
    assert_eq!(head(&s, "boots"), "boots");
}
