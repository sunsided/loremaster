//! Text pools on dramaturge's core (P4-01): the progress of an inline
//! `one_of` group is keyed under its owner, so two rooms never share it.

use loremaster_project::{SourceFile, load};
use loremaster_runtime::authoring;
use loremaster_runtime::{Input, Session, World};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

#[test]
fn groups_with_the_same_index_keep_their_own_progress() {
    let group = "description:\n  - one_of:\n      - mode: cycle\n        key: c\n        variants: [\"One.\", \"Two.\"]\n";
    let project = load(vec![
        file("story.yaml", "title: T\nstart: a\n"),
        file(
            "rooms/a.yaml",
            &format!("name: A\n{group}exits:\n  - {{ direction: east, to: b }}\n"),
        ),
        file("rooms/b.yaml", &format!("name: B\n{group}")),
    ])
    .unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    authoring::play(&mut game, Input::Text("look".into()));
    let text = authoring::play(&mut game, Input::Text("e".into())).plain_text();
    // Room b starts its own cycle: the first variant, not the third showing.
    assert!(text.contains("One."), "{text}");
    let keys: Vec<_> = game.state().progress.keys().cloned().collect();
    assert!(keys.contains(&"room:a/0/c".to_string()), "{keys:?}");
    assert!(keys.contains(&"room:b/0/c".to_string()), "{keys:?}");
    assert_eq!(game.state().progress["room:a/0/c"].shown, 2);
    assert_eq!(game.state().progress["room:b/0/c"].shown, 1);
}

#[test]
fn the_room_text_of_a_group_mentions_its_things() {
    // An unlisted thing that only a group's text names can still be found.
    let project = load(vec![
        file("story.yaml", "title: T\nstart: a\n"),
        file(
            "rooms/a.yaml",
            "name: A\ndescription:\n  - one_of:\n      - \"A brass bell hangs here.\"\n",
        ),
        file(
            "things/bell.yaml",
            "name: bell\nlocation: { room: a }\nlisted: false\n",
        ),
    ])
    .unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world);
    authoring::start(&mut game);
    let parts = authoring::Inspector::of(&game).room_parts();
    let bell = parts.objects.iter().find(|p| p.id == "bell").unwrap();
    assert_eq!(
        bell.why.as_deref(),
        Some("not listed: the room text mentions it")
    );
}
