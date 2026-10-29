//! `lantern play <dir>` and `lantern check <dir>`.

use lantern_project::{Project, Severity, check, load, read_dir};
use lantern_runtime::{BlockKind, Game, Turn, World};
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

const USAGE: &str = "usage: lantern <play|check> <project-folder>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let Some(project) = open(Path::new(dir)) else {
        return ExitCode::FAILURE;
    };
    match cmd.as_str() {
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
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
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
