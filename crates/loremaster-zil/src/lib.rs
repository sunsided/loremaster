//! Import of ZIL games (Infocom's Zork Implementation Language) into loremaster projects.
//!
//! The importer reads rooms, exits and objects, which are data in ZIL. It translates a few
//! common code patterns, such as room descriptions that depend on a flag. It does not port
//! the behaviour in routines; the import report lists it.

pub mod convert;
pub mod hints;
pub mod out;
pub mod reader;
pub mod report;
pub mod source;
pub mod translate;

pub use convert::{World, convert};
pub use out::OutFile;
pub use source::Source;

/// The files of a loremaster project folder for a converted world.
pub fn files(world: &World) -> Vec<OutFile> {
    let mut out = vec![
        OutFile {
            path: "story.yaml".into(),
            text: world.story.to_yaml(),
        },
        OutFile {
            path: "IMPORT.md".into(),
            text: report::render(world),
        },
    ];
    out.extend(world.rooms.iter().map(|r| OutFile {
        path: format!("rooms/{}.yaml", r.id),
        text: r.to_yaml(),
    }));
    out.extend(world.things.iter().map(|t| OutFile {
        path: format!("things/{}.yaml", t.id),
        text: t.to_yaml(),
    }));
    out
}
