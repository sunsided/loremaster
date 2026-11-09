//! The verbs that apply to a thing (#235): rules and frames that name it,
//! and the standard verbs whose default does something for it now.

use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Input, Session, World};
use std::sync::Arc;

fn session() -> Session {
    let files = [
        ("story.yaml", "title: Verbs\nstart: hall\n"),
        ("rooms/hall.yaml", "name: Hall\n"),
        ("rooms/cellar.yaml", "name: Cellar\n"),
        (
            "things/lamp.yaml",
            "name: lamp\nportable: true\nprops: { lit: false }\nlocation: { room: hall }\n",
        ),
        (
            "things/chest.yaml",
            "name: chest\ncontainer: true\nkey: brass_key\nprops: { open: false, locked: true }\nlocation: { room: hall }\n",
        ),
        (
            "things/brass_key.yaml",
            "name: brass key\nportable: true\nlocation: carried\n",
        ),
        (
            "things/statue.yaml",
            "name: statue\nlocation: { room: hall }\nframes:\n  pull:\n    before: [\"You grip the statue.\"]\nrules:\n  - id: push_statue\n    on: { verb: [push, statue] }\n    then:\n      - say: \"It rocks.\"\n",
        ),
        ("things/barrel.yaml", "name: barrel\nlocation: { room: cellar }\n"),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    let mut s = Session::new(Arc::new(World::compile(project.story).unwrap()));
    s.start();
    s
}

fn ids(s: &Session, thing: &str) -> Vec<String> {
    s.verbs_for(thing).into_iter().map(|v| v.id).collect()
}

#[test]
fn standard_verbs_follow_the_properties_and_the_state() {
    let mut s = session();
    assert_eq!(ids(&s, "lamp"), ["examine", "take", "turn_on", "use"]);
    assert_eq!(ids(&s, "chest"), ["examine", "put", "unlock", "use"]);
    assert_eq!(
        ids(&s, "brass_key"),
        ["drop", "examine", "put", "unlock", "use"]
    );

    s.submit(Input::Text("unlock chest with brass key".into()));
    assert_eq!(ids(&s, "chest"), ["examine", "lock", "open", "put", "use"]);
    s.submit(Input::Text("open chest".into()));
    assert_eq!(ids(&s, "chest"), ["close", "examine", "lock", "put", "use"]);
}

#[test]
fn rules_and_frames_that_name_the_thing_count() {
    let s = session();
    assert_eq!(ids(&s, "statue"), ["examine", "pull", "push"]);
}

#[test]
fn a_thing_out_of_scope_or_unknown_has_no_verbs() {
    let s = session();
    assert!(ids(&s, "barrel").is_empty());
    assert!(ids(&s, "nothing").is_empty());
}
