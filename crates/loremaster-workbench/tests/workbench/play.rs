//! Typed text in the workbench reads the session words as `Session::submit`
//! does: `undo` undoes, unless a talk or a waiting rule takes the text.

use crate::{Host, example, lighthouse};

#[test]
fn typed_undo_undoes_the_last_input() {
    let mut game = lighthouse();
    game.start();
    let start = game.scene().room;
    game.input("n");
    assert_ne!(game.scene().room, start);
    assert_eq!(game.session().undo_steps(), 1);

    let turn = game.input("undo");
    assert!(
        turn.plain_text().starts_with("Undone."),
        "{}",
        turn.plain_text()
    );
    assert_eq!(game.scene().room, start);
    assert_eq!(game.session().undo_steps(), 0);
}

#[test]
fn typed_undo_during_a_talk_goes_to_the_talk() {
    let mut game = example("cantina");
    game.start();
    game.input("in");
    game.input("talk to figure");
    assert!(game.session().talk().is_some());
    let steps = game.session().undo_steps();

    let turn = game.input("undo");
    assert!(
        !turn.plain_text().contains("Undone."),
        "{}",
        turn.plain_text()
    );
    assert!(game.session().talk().is_some(), "the talk stays open");
    assert!(game.session().undo_steps() >= steps, "nothing was undone");
}
