# Zork I

This folder contains Zork I: The Great Underground Empire, by Tim Anderson, Marc Blank, Bruce Daniels and Dave Lebling (Infocom, 1980). Use it to test lantern with a large map: 110 rooms, more than 300 exits and more than 130 things.

## Source and license

The source is the ZIL code in [historicalsource/zork1](https://github.com/historicalsource/zork1), commit `97b7b3d`. Microsoft published it under the MIT License. The file `LICENSE` in this folder is a copy of that license. It applies to the text and data in this folder. Keep it with the files.

The MIT License does not give rights to the name "Zork". Do not use the name for lantern itself.

## Files

| Path | Contents | Made by |
| --- | --- | --- |
| `story.yaml`, `rooms/`, `things/` | Rooms, exits, things and their texts. | The importer. Do not edit these files. |
| `IMPORT.md` | The parts that the importer did not convert. | The importer. |
| `rules/` | Puzzles that are ported by hand. | An author. The importer does not change this folder. |

## Import again

Run `task import:zork`. The task gets the ZIL source into `target/zil/zork1` and runs the importer. The importer replaces the room and thing files of the earlier import. It does not change `rules/`, `README.md` or `LICENSE`.

## What you can play

- The full map, with the room descriptions of the original. 109 of 110 room descriptions are imported. Some descriptions change with the state, for example the kitchen window and the trap door.
- Doors and locks: the kitchen window, the trap door, and the grating with its key.
- Conditional exits, for example the troll that blocks the passages until `troll_flag` is true.
- Dark rooms. The brass lantern is a light source.
- Two puzzles that are ported by hand: the rug and the trap door, and the leaves on the grating.

## What is not ported

Most of the behaviour of the original is in ZIL routines: the troll and the thief, combat, the lamp battery, the dam, the boat, the score and others. The importer does not port routines. `IMPORT.md` lists each one. These features of the original have no equivalent in lantern now:

- Containers. Things in a container are in the room of the container.
- Scenery in more than one room. The importer puts it in the first room only.
- Exits that only show a message, such as "The door is boarded".
- Timers, characters that move on their own, and random events.
- The score.
