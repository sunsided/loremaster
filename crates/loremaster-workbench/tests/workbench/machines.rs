//! The puzzle machines view.

use crate::{Host, world_of};
use loremaster_workbench::Workbench;

fn chapel() -> Workbench {
    let mut bench = Workbench::new(world_of(&[
        (
            "story.yaml",
            r#"title: Chapel
start: hall
verbs:
  - { id: ring, label: Ring, words: [ring], default: "Nothing rings." }
timers:
  - { id: window, turns: 6 }
"#,
        ),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A chapel.\"]\n",
        ),
        (
            "things/bell.yaml",
            "name: bell\nlocation: { room: hall }\nportable: true\n",
        ),
        (
            "things/candles.yaml",
            "name: candles\nlocation: { room: hall }\nportable: true\nprops: { lit: false }\n",
        ),
        (
            "things/book.yaml",
            "name: book\nlocation: { room: hall }\nportable: true\n",
        ),
        (
            "machines/exorcism.yaml",
            r#"start: idle
steps:
  - from: idle
    on: { verb: [ring, bell] }
    to: rung
    instead: true
    say: "The bell rings red hot."
    then: [{ start: window }]
  - from: rung
    on: { verb: [turn_on, candles] }
    when: carried(candles)
    to: lit
  - from: lit
    on: { verb: [examine, book] }
    to: done
    say: "The spirits flee."
  - from: [rung, lit]
    on: { timer: window }
    to: idle
    say: "The spirits jeer."
"#,
        ),
    ]));
    bench.start();
    bench
}

fn say(g: &mut Workbench, cmd: &str) -> String {
    g.input(cmd).plain_text()
}

#[test]
fn the_machine_view_follows_the_steps() {
    let mut g = chapel();
    assert_eq!(g.machines()[0].state, "idle");
    say(&mut g, "ring bell");
    assert_eq!(g.machines()[0].state, "rung");
    say(&mut g, "take candles");
    say(&mut g, "light candles");
    let view = &g.machines()[0];
    assert_eq!(view.state, "lit");
    assert_eq!(view.start, "idle");
    assert_eq!(view.states, ["idle", "rung", "lit", "done"]);
    assert!(view.steps[0].fired && view.steps[1].fired);
    assert!(!view.steps[2].fired);
}

#[test]
fn a_timer_resets_the_machine_and_the_first_step_is_ready_again() {
    let mut g = chapel();
    say(&mut g, "ring bell");
    for _ in 0..6 {
        say(&mut g, "wait");
    }
    let view = &g.machines()[0];
    assert_eq!(view.state, "idle");
    assert!(view.steps[0].ready);
}
