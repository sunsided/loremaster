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
