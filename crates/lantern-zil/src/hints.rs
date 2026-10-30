//! Knowledge about Zork I that is in routines, not in data, and that the importer can not read.
//!
//! Each entry replaces one small routine with the lantern data that has the same effect.
//! The entries for other games are empty: the importer then reports these exits as not ported.

use crate::out::ExitOut;

/// Objects that the parser uses internally. They are not things in the world.
pub const PSEUDO_OBJECTS: &[&str] = &[
    "GLOBAL-OBJECTS",
    "LOCAL-GLOBALS",
    "ROOMS",
    "INTNUM",
    "PSEUDO-OBJECT",
    "IT",
    "NOT-HERE-OBJECT",
    "ADVENTURER",
    "ME",
];

/// The title of the game, if the importer knows it.
pub fn title(start: &str) -> Option<&'static str> {
    (start == "WEST-OF-HOUSE").then_some("Zork I: The Great Underground Empire")
}

/// Doors with a lock: `(door, locked at the start, key)`. The trap door has no key, but a rule
/// bars it from above.
pub const LOCKS: &[(&str, bool, Option<&str>)] =
    &[("GRATE", true, Some("KEYS")), ("TRAP-DOOR", false, None)];

/// The ZIL flags that the exit of `per_exit` uses in its conditions.
pub fn per_exit_flags(routine: &str) -> &'static [&'static str] {
    match routine {
        "TRAP-DOOR-EXIT" => &["RUG-MOVED"],
        "GRATING-EXIT" => &["GRATE-REVEALED"],
        _ => &[],
    }
}

/// An exit for a `(DIRECTION PER ROUTINE)` exit of a room.
pub fn per_exit(room: &str, routine: &str) -> Option<ExitOut> {
    let to = |to: &str| ExitOut {
        to: to.to_string(),
        ..Default::default()
    };
    match (routine, room) {
        // The trap door is under the rug; it is an exit only after the rug moves.
        ("TRAP-DOOR-EXIT", _) => Some(ExitOut {
            door: Some("trap_door".into()),
            visible: Some("rug_moved".into()),
            ..to("cellar")
        }),
        // The grating is under the leaves; it is an exit only after the leaves move.
        ("GRATING-EXIT", _) => Some(ExitOut {
            door: Some("grate".into()),
            visible: Some("grate_revealed".into()),
            ..to("grating_room")
        }),
        // The original also needs the player to carry the lamp and at most one other item.
        ("UP-CHIMNEY-FUNCTION", _) => Some(ExitOut {
            passable: Some("carried(lamp)".into()),
            blocked: Some("You can't get up there with what you're carrying.".into()),
            ..to("kitchen")
        }),
        ("MAZE-DIODES", room) => {
            let target = match room {
                "MAZE-2" => "maze_4",
                "MAZE-7" => "dead_end_1",
                "MAZE-9" => "maze_11",
                "MAZE-12" => "maze_5",
                _ => return None,
            };
            Some(ExitOut {
                traversal: Some(
                    "You won't be able to get back up to the tunnel you are going through when it gets to the next room."
                        .into(),
                ),
                ..to(target)
            })
        }
        _ => None,
    }
}
