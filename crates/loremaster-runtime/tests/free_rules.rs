//! Free rules (#223): a rule with `free: true` lets the command that fires
//! it take no time. The clock, the turn count and the timers stay, as for a
//! command that the game refuses.

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, World, authoring, run_walkthrough};
use std::sync::Arc;

const STORY: &str = r#"
title: Box
start: hall
clock: { start: "22:00", minutes_per_turn: 1 }
timers:
  - { id: fuse, turns: 3, running: true }
"#;

const RULES: &str = r#"
- id: already_open
  on: { check: [open, box] }
  when: box.open
  free: true
  then: [{ say: "It is already open." }, stop]
- id: boom
  on: { timer: fuse }
  then: [{ end: blown }]
"#;

fn files(rules: &str, walk: Option<&str>) -> Vec<SourceFile> {
    let mut files = vec![
        ("story.yaml", STORY),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        (
            "things/box.yaml",
            "name: box\nlocation: { room: hall }\ncontainer: true\nprops: { open: false }\n",
        ),
        ("endings/blown.yaml", "kind: loss\nrank: 1\ntitle: Blown\n"),
        ("rules/rules.yaml", rules),
    ];
    if let Some(w) = walk {
        files.push(("tests/free.yaml", w));
    }
    files
        .into_iter()
        .map(|(path, text)| SourceFile {
            path: path.into(),
            text: text.into(),
        })
        .collect()
}

fn world(rules: &str) -> Arc<World> {
    let project = load(files(rules, None)).unwrap();
    assert!(check(&project).is_empty(), "{:#?}", check(&project));
    Arc::new(World::compile(project.story).unwrap())
}

fn say(game: &mut Session, text: &str) -> String {
    authoring::play(game, Input::Text(text.into())).plain_text()
}

#[test]
fn a_free_rule_does_not_move_the_clock_or_the_timers() {
    let mut game = Session::new(world(RULES));
    authoring::start(&mut game);
    say(&mut game, "open box");
    let (minutes, turns) = (game.state().minutes, game.state().turns);
    let fuse = game.state().timers["fuse"];
    for _ in 0..5 {
        let text = say(&mut game, "open box");
        assert!(text.contains("already open"), "{text}");
    }
    let s = game.state();
    assert_eq!(
        (s.minutes, s.turns, s.timers["fuse"]),
        (minutes, turns, fuse)
    );
    assert!(!s.ended);
    // Any other command takes its turn as before.
    say(&mut game, "examine box");
    assert_eq!(game.state().turns, turns + 1);
}

#[test]
fn a_free_enter_rule_does_not_make_the_move_free() {
    let rules = format!(
        "{RULES}- id: hello\n  on: {{ enter: yard }}\n  free: true\n  then: [{{ say: \"A yard.\" }}]\n"
    );
    let mut files = files(&rules, None);
    files[1].text =
        "name: Hall\ndescription: [\"A hall.\"]\nexits:\n  - { direction: north, to: yard }\n"
            .into();
    files.push(SourceFile {
        path: "rooms/yard.yaml".into(),
        text: "name: Yard\ndescription: [\"A yard.\"]\n".into(),
    });
    let project = load(files).unwrap();
    // `loremaster check` warns about it; the game ignores it.
    assert!(
        check(&project).iter().any(|d| d.at == "free"),
        "{:#?}",
        check(&project)
    );
    let mut game = Session::new(Arc::new(World::compile(project.story).unwrap()));
    authoring::start(&mut game);
    let turns = game.state().turns;
    assert!(say(&mut game, "n").contains("A yard."));
    assert_eq!(game.state().turns, turns + 1);
}

#[test]
fn a_walkthrough_counts_a_free_command_as_no_turn() {
    // Three turns run the fuse out: the opening and two waits, not the refusals.
    let walk = "steps:\n  - open box\n  - open box\n  - open box\n  - open box\n  - wait\n  - { do: open box, see: already open }\nexpect: { ended: false }\n";
    let project = load(files(RULES, Some(walk))).unwrap();
    let w = Arc::new(World::compile(project.story).unwrap());
    let (path, walk) = &project.walkthroughs[0];
    let result = run_walkthrough(w, walk, path);
    assert!(result.passed(), "{:#?}", result.failures);
}

#[test]
fn the_explorer_finds_no_new_state_in_a_free_refusal() {
    // A free refusal leads back into the state it came from: it adds no
    // state. (Without `free` it is one more way to run the fuse down, as
    // `wait` is, so here the count stays the same.)
    let free = explore(world(RULES), Budget::default());
    assert!(free.never_fired.is_empty(), "{free:#?}");
    assert!(!free.endings.is_empty(), "{free:#?}");
    let paid = explore(
        world(&RULES.replace("  free: true\n", "")),
        Budget::default(),
    );
    assert!(
        free.states <= paid.states,
        "{} vs {}",
        free.states,
        paid.states
    );
}
