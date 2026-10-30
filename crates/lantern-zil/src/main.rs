//! `lantern-zil <zil-folder> <project-folder>`: import a ZIL game into a lantern project folder.
//!
//! The importer writes `story.yaml`, `IMPORT.md`, `rooms/` and `things/`. It removes the room and
//! thing files of an earlier import, but keeps all other files, such as hand-written `rules/`.

use lantern_zil::{Source, convert, files};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: lantern-zil <zil-folder> <project-folder>";
/// The first line of each room and thing file that the importer writes.
const MARK: &str = "# Imported from ZIL:";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [zil, project] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(Path::new(zil), Path::new(project)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(zil: &Path, project: &Path) -> std::io::Result<()> {
    let mut names: Vec<_> = fs::read_dir(zil)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zil")))
        .collect();
    names.sort();
    let mut src = Source::default();
    for path in &names {
        let bytes = fs::read(path)?;
        let text = String::from_utf8_lossy(&bytes).replace('\r', "");
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        src.add_file(&name, &text);
    }

    let mut rules = String::new();
    if let Ok(entries) = fs::read_dir(project.join("rules")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|x| x == "yaml" || x == "yml") {
                rules.push_str(&fs::read_to_string(&path)?);
            }
        }
    }

    let world = convert(&src, &rules);
    for dir in ["rooms", "things"] {
        let dir = project.join(dir);
        fs::create_dir_all(&dir)?;
        for entry in fs::read_dir(&dir)?.filter_map(Result::ok) {
            if fs::read_to_string(entry.path()).is_ok_and(|t| t.starts_with(MARK)) {
                fs::remove_file(entry.path())?;
            }
        }
    }
    for file in files(&world) {
        fs::write(project.join(&file.path), file.text)?;
    }

    let r = &world.report;
    println!(
        "{}: {} rooms, {} things, {} routines and {} interrupt routines not ported. See IMPORT.md.",
        world.story.title,
        world.rooms.len(),
        world.things.len(),
        r.routines.len(),
        r.daemons.len()
    );
    for (file, e) in &src.errors {
        eprintln!("warning: {file}: {e}");
    }
    Ok(())
}
