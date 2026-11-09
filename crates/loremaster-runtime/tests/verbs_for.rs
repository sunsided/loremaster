//! The verbs that apply to a thing (#235): rules and frames that name it,
//! and the standard verbs whose default does something for it now.

use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Input, Session, World};
use std::sync::Arc;

fn session() -> Session {
    let files = [
        ("story.yaml", "title: Verbs\nstart: hall\ncarry_limit: 3\n"),
        ("rooms/hall.yaml", "name: Hall\n"),
        ("rooms/cellar.yaml", "name: Cellar\n"),
        (
            "things/lamp.yaml",
            "name: lamp\nportable: true\nprops: { lit: false }\nlocation: { room: hall }\n",
        ),
        (
            "things/coin.yaml",
            "name: coin\nportable: true\nlocation: { room: hall }\n",
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
            "things/safe.yaml",
            "name: safe\ncontainer: true\nkey: safe_key\nprops: { open: false, locked: true }\nlocation: { room: hall }\n",
        ),
        (
            "things/safe_key.yaml",
            "name: safe key\nportable: true\nlocation: { room: cellar }\n",
        ),
        (
            "things/trunk.yaml",
            "name: trunk\ncontainer: true\nkey: iron_key\nprops: { open: false, locked: true }\nlocation: { room: cellar }\n",
        ),
        (
            "things/iron_key.yaml",
            "name: iron key\nportable: true\nlocation: carried\n",
        ),
        (
            "things/cupboard.yaml",
            "name: cupboard\ncontainer: true\nkey: tin_key\nprops: { open: false, locked: true }\nlocation: { room: hall }\n",
        ),
        (
            "things/tin_key.yaml",
            "name: tin key\nportable: true\nlocation: { room: hall }\n",
        ),
        (
            "things/jar.yaml",
            "name: jar\ncontainer: true\nprops: { open: false, transparent: true }\nlocation: { room: hall }\n",
        ),
        (
            "things/pebble.yaml",
            "name: pebble\nportable: true\nlocation: { in: jar }\n",
        ),
        (
            "things/statue.yaml",
            "name: statue\nlocation: { room: hall }\nframes:\n  pull:\n    before: [\"You grip the statue.\"]\nrules:\n  - id: push_statue\n    on: { verb: [push, statue] }\n    then:\n      - say: \"It rocks.\"\n  - id: keeper_lifts\n    on: { verb: [take, statue], actor: keeper }\n    then:\n      - say: \"The keeper lifts it.\"\n",
        ),
        ("things/keeper.yaml", "name: keeper\nlocation: { room: cellar }\n"),
        ("things/barrel.yaml", "name: barrel\nlocation: { room: cellar }\n"),
        ("things/bard.yaml", "name: bard\nlocation: { room: hall }\n"),
        ("things/ghost.yaml", "name: ghost\nlocation: { room: hall }\n"),
        (
            "dialogues/bard.yaml",
            "with: bard\nstart: [hello]\nnodes:\n  hello:\n    say: \"Hello.\"\n",
        ),
        (
            "dialogues/ghost.yaml",
            "with: ghost\nstart: [boo]\nnodes:\n  boo:\n    when: lamp.lit\n    say: \"Boo.\"\n",
        ),
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
fn a_lock_needs_its_key_in_scope_and_a_key_its_lock() {
    let s = session();
    // The safe key is in the cellar.
    assert_eq!(ids(&s, "safe"), ["examine", "put"]);
    // The trunk of the iron key is in the cellar.
    assert_eq!(ids(&s, "iron_key"), ["drop", "examine", "put"]);
    // The tin key lies in the hall: `unlock cupboard` only uses a carried key.
    assert_eq!(ids(&s, "cupboard"), ["examine", "put"]);
}

#[test]
fn take_follows_the_default_take() {
    let mut s = session();
    // Seen through a closed jar, but not reachable.
    assert_eq!(ids(&s, "pebble"), ["examine"]);
    assert!(ids(&s, "coin").contains(&"take".to_string()));
    // Two keys and the lamp fill the carry limit of three.
    s.submit(Input::Text("take lamp".into()));
    assert!(!ids(&s, "coin").contains(&"take".to_string()));
}

#[test]
fn talk_needs_a_start_node_that_applies() {
    let mut s = session();
    assert_eq!(ids(&s, "bard"), ["examine", "talk"]);
    assert_eq!(ids(&s, "ghost"), ["examine"]);
    s.submit(Input::Text("turn on lamp".into()));
    assert_eq!(ids(&s, "ghost"), ["examine", "talk"]);
}

#[test]
fn rules_and_frames_that_name_the_thing_count() {
    let s = session();
    // The rule for the keeper's take is not a command of the player.
    assert_eq!(ids(&s, "statue"), ["examine", "pull", "push"]);
}

#[test]
fn a_thing_out_of_scope_or_unknown_has_no_verbs() {
    let s = session();
    assert!(ids(&s, "barrel").is_empty());
    assert!(ids(&s, "nothing").is_empty());
}
