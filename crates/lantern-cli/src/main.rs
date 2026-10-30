//! `lantern new <dir> [--title T]`, `lantern play <dir>`, `lantern check <dir> [--todos]`,
//! `lantern explore <dir>` and
//! `lantern mcp <dir>` and `lantern serve <dir> [--port N]`.

use lantern_project::{Project, Severity, check, load, open_todos, read_dir};
use lantern_runtime::{BlockKind, Budget, Game, Turn, World};
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

const USAGE: &str = "usage: lantern <play|check|explore|mcp> <project-folder>\n       lantern new <project-folder> [--title \"The Title\"]\n       lantern check <project-folder> --todos\n       lantern serve <project-folder> [--port N]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if cmd == "new" {
        let title = match args.get(2).map(String::as_str) {
            None => "Untitled",
            Some("--title") if args.len() == 4 => args[3].as_str(),
            Some(_) => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        };
        return new_story(Path::new(dir), title);
    }
    // The MCP server owns stdout and also serves a project with errors, so it skips `open`.
    if cmd == "mcp" {
        return match lantern_mcp::serve_stdio(Path::new(dir)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if cmd == "serve" {
        let port = match args.get(2).map(String::as_str) {
            None => lantern_mcp::http::DEFAULT_PORT,
            Some("--port") => match args.get(3).and_then(|p| p.parse().ok()) {
                Some(p) => p,
                None => {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
            },
            Some(_) => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        };
        return match lantern_mcp::http::serve_http(Path::new(dir), port) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let Some(project) = open(Path::new(dir)) else {
        return ExitCode::FAILURE;
    };
    match cmd.as_str() {
        "check" if args.get(2).is_some_and(|a| a == "--todos") => {
            let todos = open_todos(&project);
            for t in &todos {
                let line = t.line.map(|l| format!(":{l}")).unwrap_or_default();
                println!("{}{line} ({}): {}", t.file, t.at, t.note.text);
            }
            println!("{} open TODO(s).", todos.len());
            ExitCode::SUCCESS
        }
        "check" => {
            let n = project.story.rooms.len();
            println!(
                "{}: {n} rooms, {} things, {} rules, no problems.",
                project.story.title,
                project.story.things.len(),
                project.story.rules.len()
            );
            ExitCode::SUCCESS
        }
        "play" => play(project),
        "explore" => explore(project),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Write the files of a new story into a folder. The folder must not hold a story yet.
fn new_story(dir: &Path, title: &str) -> ExitCode {
    if dir.join("story.yaml").exists() {
        eprintln!("error: {} has a story.yaml already", dir.display());
        return ExitCode::FAILURE;
    }
    for file in lantern_project::scaffold(title) {
        let path = dir.join(&file.path);
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, &file.text));
        if let Err(e) = written {
            eprintln!("error: cannot write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    println!(
        "New story in {}. Play it with `lantern play {}`.",
        dir.display(),
        dir.display()
    );
    ExitCode::SUCCESS
}

/// Load and check a project, printing every problem.
fn open(dir: &Path) -> Option<Project> {
    let files = match read_dir(dir) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", dir.display());
            return None;
        }
    };
    let project = match load(files) {
        Ok(p) => p,
        Err(diags) => {
            diags.iter().for_each(|d| eprintln!("{d}"));
            return None;
        }
    };
    let diags = check(&project);
    diags.iter().for_each(|d| eprintln!("{d}"));
    if diags.iter().any(|d| d.severity == Severity::Error) {
        return None;
    }
    Some(project)
}

fn play(project: Project) -> ExitCode {
    let world = match World::compile(project.story) {
        Ok(w) => Arc::new(w),
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let style = std::io::stdout().is_terminal();
    let mut game = Game::new(world.clone());
    println!("{}\n", bold(&world.story.title, style));
    print_turn(&game.start(), style);
    let stdin = std::io::stdin();
    loop {
        print!("\n> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        let line = line.trim();
        if matches!(line, "quit" | "q" | "exit") {
            break;
        }
        if line == "time" {
            let s = game.scene();
            println!("Day {}, {} ({}), turn {}.", s.day, s.time, s.phase, s.turns);
            continue;
        }
        let turn = game.input(line);
        println!();
        print_turn(&turn, style);
        if turn.ended {
            println!("\n*** The End ***");
            break;
        }
    }
    ExitCode::SUCCESS
}

/// Walk all reachable states and print what the walk found.
fn explore(project: Project) -> ExitCode {
    let world = match World::compile(project.story) {
        Ok(w) => Arc::new(w),
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = lantern_runtime::explore(&Game::new(world), Budget::default());
    let more = if result.truncated {
        " (budget reached; results are incomplete)"
    } else {
        ""
    };
    println!("{} states explored{more}.", result.states);
    for room in &result.rooms {
        let state = if room.reached {
            format!("{} text variants", room.variants.len())
        } else {
            "NOT REACHED".into()
        };
        println!("  {} ({}): {state}", room.name, room.id);
    }
    for ending in &result.endings {
        println!(
            "Ending in {} commands: {}",
            ending.path.len(),
            ending.path.join(", ")
        );
    }
    if result.stuck > 0 {
        println!(
            "{} states cannot reach an ending. Shortest way in: {}",
            result.stuck,
            result.stuck_path.join(", ")
        );
    }
    for f in &result.never_shown {
        println!("Never shown: {f}");
    }
    for r in &result.never_fired {
        println!("Never fired: {r}");
    }
    let problems = result.stuck > 0 || result.rooms.iter().any(|r| !r.reached);
    if problems {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_turn(turn: &Turn, style: bool) {
    let mut first = true;
    for block in &turn.blocks {
        if block.kind == BlockKind::Command {
            continue;
        }
        if !first {
            println!();
        }
        first = false;
        let text = emphasis(&block.plain_text(), style);
        match block.kind {
            BlockKind::Title => println!("{}", bold(&text, style)),
            _ => println!("{}", wrap(&text, 78)),
        }
    }
}

fn bold(text: &str, style: bool) -> String {
    if style {
        format!("\x1b[1m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// `*word*` in story text is emphasis.
fn emphasis(text: &str, style: bool) -> String {
    let mut out = String::new();
    for (i, part) in text.split('*').enumerate() {
        if i % 2 == 1 && style {
            out.push_str(&format!("\x1b[3m{part}\x1b[0m"));
        } else {
            out.push_str(part);
        }
    }
    out
}

fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut col = 0;
    for word in text.split(' ') {
        let len = word.chars().filter(|c| !c.is_control()).count();
        if col > 0 && col + 1 + len > width {
            out.push('\n');
            col = 0;
        } else if col > 0 {
            out.push(' ');
            col += 1;
        }
        out.push_str(word);
        col += len;
    }
    out
}
