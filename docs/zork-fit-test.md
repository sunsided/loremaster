# Zork I fit test: puzzle logic

This report tells how five puzzles of Zork I are ported to loremaster, and which part of loremaster each one uses. The ports are in `examples/zork1`. The tests are in `crates/loremaster-runtime/tests/zork.rs` and `examples/zork1/tests/`.

The test checks the levels of the concept "Loremaster puzzle logic": world parts first, then stronger rules, then puzzle machines, then behaviors in Rust.

## Result

All five puzzles are ported. Four of them are data only: the editor, the checks and the state explorer see them. Only the thief is code.

| Puzzle | ZIL routines | Ported with | Files | Data only |
| --- | --- | --- | --- | --- |
| Lamp battery | `I-LANTERN`, `LANTERN` | A timer with `runs_while`, and a `check` rule with `stop` | `timers/lamp_battery.yaml`, `rules/lamp.yaml` | Yes |
| Trophy case and score | `TROPHY-CASE-FCN`, `SCORE-OBJ`, `V-SCORE` | A container, the score, and one rule for each treasure | `things/trophy_case.yaml` (importer), `story.yaml` (importer), `rules/treasures.yaml` | Yes |
| Exorcism | `BELL-F`, `CANDLES-FCN`, `BLACK-BOOK`, `I-XB`, `I-XC` | A machine with 4 states and a timer, and a custom verb | `machines/exorcism.yaml`, `timers/exorcism_window.yaml`, `verbs/ring.yaml` | Yes |
| Dam and reservoir | `DAM-FUNCTION`, `BOLT-F`, `BUTTON-F`, `I-REMPTY`, `I-RFILL` | A machine with 4 states, two timers, `check` rules and a custom verb | `machines/sluice_gates.yaml`, `timers/reservoir_*.yaml`, `rules/dam.yaml`, `verbs/turn.yaml` | Yes |
| Thief | `I-THIEF`, `ROBBER-FUNCTION` | A behavior in Rust | `thief.rs`, `behaviors/thief.yaml` | No |

## What the importer does now

The importer writes more of the world. Before this work, 54 things were in a different place. Now 18 are.

- Things in a container or on a surface are `in:` or `on:` that thing. The holder gets `container: true` or `supporter: true`.
- Scenery in the `GLOBAL` list of more than one room gets `also_in` with the other rooms.
- `VALUE` and `TVALUE` become the properties `value` and `tvalue`.
- `SCORE-MAX` becomes `score: { max: 350 }`, with the ranks of `V-SCORE`.

The other 18 things are global objects (such as the ground and the hands), things that are not visible at the start, and the axe that the troll holds. The troll is not a container.

## Notes for each puzzle

### Lamp battery

The timer starts at 385 turns and counts only while `lamp.lit` is true. The warnings come at 185, 85 and 15 turns left, as in `LAMP-TABLE`. At 0 the lamp goes out. The check rule `dead_lamp` reads `timer.lamp_battery.running`, so no extra variable is necessary.

### Trophy case and score

`rules/treasures.yaml` has 40 rules: one for the first take of each treasure, and one for each treasure in the case. A small script made them from the properties. A rule language with a loop over things would make this file short. That is a possible next step, but it is not necessary.

### Exorcism

The steps must come quickly after each other. The bell starts the timer `exorcism_window`. If the timer runs out before the book, the machine goes back to `idle` and the bell is cold again. The step from `idle` checks `here == entrance_to_hades`, because the machine rules belong to no room. A `within:` field on machines would make this shorter.

### Dam and reservoir

The room texts of the import already read `gates_open` and `low_tide`. The machine sets them. The bolt needs the yellow button first and the wrench in the hands. Two `check` rules with `stop` refuse the turn otherwise. The reservoir becomes passable 8 turns after the gates open.

### Thief

The thief walks through the dark rooms and can take a treasure that the player carries. All of his state is the location of the thief and of the things in his bag, and the random numbers come from the save state. Thus the game replays, and a saved game goes on in the same way (`tests/behavior.rs`). The fights with the thief are not ported.

## What is still missing

- Vehicles: the boat and the basket move the player with a thing. Rules can do this with `goto`, but there is no world part for it.
- Combat with the troll, the thief and the cyclops. This is a good use for a behavior.
- `action.ok`: verb rules can not yet read if the default response did what it should. They read the state, for example `lamp.lit`, in its place.
- A refusal from a `check` rule uses a turn. There is no `free` option.
