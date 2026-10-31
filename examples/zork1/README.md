# Zork I

This folder contains Zork I: The Great Underground Empire, by Tim Anderson, Marc Blank, Bruce Daniels and Dave Lebling (Infocom, 1980). Use it to test lantern with a large map: 110 rooms, more than 300 exits and more than 130 things.

## Source and license

The source is the ZIL code in [historicalsource/zork1](https://github.com/historicalsource/zork1), commit `97b7b3d`. Microsoft published it under the MIT License. The file `LICENSE` in this folder is a copy of that license. It applies to the text and data in this folder. Keep it with the files.

The MIT License does not give rights to the name "Zork". Do not use the name for loremaster itself.

## Files

| Path | Contents | Made by |
| --- | --- | --- |
| `story.yaml`, `rooms/`, `things/` | Rooms, exits, things, containers, the score and their texts. | The importer. Do not edit these files. |
| `IMPORT.md` | The parts that the importer did not convert. | The importer. |
| `rules/`, `timers/`, `machines/`, `verbs/` | Puzzles that are ported by hand. | An author. The importer does not change these folders. |
| `behaviors/`, `thief.rs` | The thief, a behavior in Rust. | An author. |
| `tests/` | Walkthrough tests. | An author. |

## Import again

Run `task import:zork`. The task gets the ZIL source into `target/zil/zork1` and runs the importer. The importer replaces the room and thing files of the earlier import. It does not change `rules/`, `README.md` or `LICENSE`.

## What you can play

- The full map, with the room descriptions of the original. 109 of 110 room descriptions are imported. Some descriptions change with the state, for example the kitchen window and the trap door.
- Doors and locks: the kitchen window, the trap door, and the grating with its key.
- Conditional exits, for example the troll that blocks the passages until `troll_flag` is true.
- Dark rooms. The brass lantern is a light source. Its battery runs down (`timers/lamp_battery.yaml`).
- Containers and surfaces, for example the trophy case, the mailbox and the kitchen table. Scenery such as the forest and the white house is in all of its rooms.
- The score: points for each treasure when you take it first, and when you put it in the trophy case. The command `score` shows it.
- Puzzles that are ported by hand: the rug and the trap door, the leaves on the grating, the exorcism at the entrance to Hades (`machines/exorcism.yaml`) and the dam with the reservoir (`machines/sluice_gates.yaml`).
- The thief, with `cargo run -p loremaster-runtime --example zork`. The editor and `loremaster play` do not have his code, so they play without him.

Read `docs/zork-fit-test.md` for how each puzzle is ported.

## What is not ported

Most of the behaviour of the original is in ZIL routines. The importer does not port routines. `IMPORT.md` lists each one. These parts of the original are not ported now:

- The troll, the cyclops and combat.
- The boat and the river, the basket in the coal mine, the mirror rooms and the loud room.
- Most special responses of objects.
