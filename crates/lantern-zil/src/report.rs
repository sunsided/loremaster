//! The import report: a Markdown file that lists what the importer did not convert.

use crate::convert::{Note, World};
use std::fmt::Write as _;

fn table(out: &mut String, heading: &str, intro: &str, columns: (&str, &str), notes: &[Note]) {
    let _ = writeln!(out, "\n## {heading}\n\n{intro}\n");
    if notes.is_empty() {
        let _ = writeln!(out, "None.");
        return;
    }
    let _ = writeln!(out, "| {} | {} |\n| --- | --- |", columns.0, columns.1);
    for n in notes {
        let detail = n.detail.replace('|', "\\|");
        let _ = writeln!(out, "| `{}` | {detail} |", n.owner);
    }
}

pub fn render(world: &World) -> String {
    let r = &world.report;
    let exits: usize = world
        .rooms
        .iter()
        .flat_map(|room| &room.exits)
        .filter(|e| e.to.is_some())
        .count();
    let mut out = String::new();
    let _ = writeln!(out, "# Import report: {}\n", world.story.title);
    let _ = writeln!(
        out,
        "lantern-zil makes this file. Do not edit it. Run the importer again to make it new.\n\n\
         ## Summary\n\n\
         | Item | Count |\n| --- | --- |\n\
         | Rooms | {} |\n\
         | Room descriptions that are not imported | {} |\n\
         | Exits | {exits} |\n\
         | Exits from hints | {} |\n\
         | Message exits (only show a message) | {} |\n\
         | Other exits that are not imported | {} |\n\
         | Things | {} |\n\
         | Things in a different place | {} |\n\
         | Room and object routines (not ported) | {} |\n\
         | Interrupt routines (not ported) | {} |",
        world.rooms.len(),
        r.descriptions.len(),
        r.hinted_exits.len(),
        r.message_exits.len(),
        r.exits.len(),
        world.things.len(),
        r.placement.len(),
        r.routines.len(),
        r.daemons.len(),
    );
    if !r.errors.is_empty() {
        table(
            &mut out,
            "Files that are not read",
            "The reader can not read these files. Their rooms and objects are missing.",
            ("File", "Problem"),
            &r.errors,
        );
    }
    table(
        &mut out,
        "Room and object routines",
        "These routines contain the behaviour of the game: puzzles, characters and special responses. \
         The importer does not port them. Write rules for them by hand in `rules/`.",
        ("Owner", "Routine"),
        &r.routines,
    );
    table(
        &mut out,
        "Room descriptions that are not imported",
        "The original prints these descriptions from code that the importer can not translate. \
         The room has a placeholder text.",
        ("Room", "Reason"),
        &r.descriptions,
    );
    table(
        &mut out,
        "Message exits",
        "In the original, these directions show a message and do not move the player. \
         The importer makes them as message exits: exits with `blocked` and no `to`.",
        ("Room", "Direction and message"),
        &r.message_exits,
    );
    table(
        &mut out,
        "Exits that are not imported",
        "The importer can not convert these exits.",
        ("Room", "Exit"),
        &r.exits,
    );
    table(
        &mut out,
        "Exits from hints",
        "A routine decides these exits in the original. The importer uses hints from `hints.rs` for them.",
        ("Room", "Exit"),
        &r.hinted_exits,
    );
    table(
        &mut out,
        "Things in a different place",
        "A lantern thing is in one room, or nowhere. These things are in a container, \
         in more than one room, or not visible in the original.",
        ("Thing", "Note"),
        &r.placement,
    );
    let _ = writeln!(
        out,
        "\n## Interrupt routines\n\n\
         These routines run on a timer or each turn: the lamp battery, the thief, combat and others. \
         The importer does not port them.\n"
    );
    for d in &r.daemons {
        let _ = writeln!(out, "- `{d}`");
    }
    out
}
