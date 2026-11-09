//! Typed text in the workbench reads the session words as `Session::submit`
//! does: `undo` undoes, unless a talk or a waiting rule takes the text.

use crate::{Host, example, lighthouse};
use loremaster_model::Value;

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

#[test]
fn explorer_paths_replay_through_talks_and_answers() {
    let mut kinds = std::collections::BTreeSet::new();
    let mut rounds = 0;
    for name in [
        "cantina",
        "gatehouse",
        "lighthouse",
        "salt-tower",
        "two-robots",
    ] {
        let mut game = example(name);
        game.start();
        let found = game.explore(loremaster_workbench::Budget::default());
        // A stage host answers rolls and walks itself; a replay plays as the walk did.
        game.session_mut().set_host_rolls(true);
        game.session_mut().set_host_walks(true);
        // The paths to the endings end the game; the paths to room texts end
        // in their room, with the facts of the variant.
        let endings = found.endings.iter().map(|e| (&e.path, None));
        let rooms = found.rooms.iter().flat_map(|r| {
            r.variants
                .iter()
                .map(move |v| (&v.path, Some((r.id.as_str(), &v.facts))))
        });
        for (path, room) in endings.chain(rooms) {
            game.restart();
            for step in path {
                kinds.insert(step.split(' ').next().unwrap_or_default().to_string());
                rounds += usize::from(step.ends_with(" (win the round)"));
                assert!(
                    game.replay_step(step).is_some(),
                    "{name}: no step `{step}` in {path:?}"
                );
            }
            let errors = loremaster_runtime::authoring::forced_roll_errors(game.session_mut());
            assert!(errors.is_empty(), "{name}: forced rolls left: {errors:?}");
            match room {
                Some((room, facts)) => {
                    assert_eq!(game.scene().room, room, "{name}: {path:?}");
                    let now: Vec<String> = game
                        .state()
                        .facts
                        .iter()
                        .filter(|(_, v)| !matches!(v, Value::Bool(false)))
                        .map(|(k, v)| format!("{k} = {v}"))
                        .collect();
                    assert_eq!(&now, facts, "{name}: {path:?}");
                }
                None => {
                    assert!(game.session().state().ended, "{name}: {path:?}");
                    // Nothing plays after the end.
                    let first = path.first().unwrap();
                    assert!(game.replay_step(first).is_none(), "{name}: {path:?}");
                }
            }
        }
    }
    for kind in ["say", "answer"] {
        assert!(kinds.contains(kind), "no `{kind}` step: {kinds:?}");
    }
    assert!(rounds > 0, "no combat round with forced rolls was replayed");
    // No shortest path above leaves a talk.
    let mut game = example("cantina");
    game.start();
    // A forced total that no roll of the step takes is reported, not dropped.
    loremaster_runtime::authoring::force_rolls(game.session_mut(), &[7]);
    assert!(game.replay_step("go in").is_some(), "no step `go in`");
    let errors = loremaster_runtime::authoring::forced_roll_errors(game.session_mut());
    assert_eq!(errors, ["no roll took the forced rolls 7"]);
    for step in ["talk blackbeard", "leave"] {
        assert!(game.replay_step(step).is_some(), "no step `{step}`");
    }
    assert!(game.session().talk().is_none(), "the talk is over");
    assert!(game.replay_step("leave").is_none(), "no talk to leave");
}
