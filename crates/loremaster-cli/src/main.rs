//! `loremaster new <dir> [--title T] [--idea I]`, `loremaster play <dir>`, `loremaster check <dir> [--todos]`,
//! `loremaster explore <dir>`, `loremaster mcp <dir>` and `loremaster serve <dir | --bootstrap> [--port N] [--allow-origin URL]... [--no-open]`.

use loremaster_project::{Project, Severity, check, load, open_todos, read_dir};
use loremaster_runtime::{BlockKind, Budget, Game, Turn, World};
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

const USAGE: &str = "usage: loremaster <play|check|explore|mcp> <project-folder>\n       loremaster new <project-folder> [--title \"The Title\"] [--idea \"An idea for the AI\"]\n       loremaster check <project-folder> --todos\n       loremaster serve <project-folder | --bootstrap> [--port N] [--allow-origin URL]... [--no-open]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if cmd == "new" {
        let (mut title, mut idea) = ("Untitled", None);
        let mut rest = args[2..].chunks(2);
        for pair in rest.by_ref() {
            match pair {
                [flag, value] if flag == "--title" => title = value,
                [flag, value] if flag == "--idea" => idea = Some(value.as_str()),
                _ => {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
        let made = new_story(Path::new(dir), title);
        return match idea {
            Some(idea) if made == ExitCode::SUCCESS => seed(Path::new(dir), idea),
            _ => made,
        };
    }
    // The MCP server owns stdout and also serves a project with errors, so it skips `open`.
    if cmd == "mcp" {
        return match loremaster_mcp::serve_stdio(Path::new(dir)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if cmd == "serve" {
        let mut options = loremaster_mcp::http::Options::default();
        let mut rest = args[2..].iter();
        while let Some(flag) = rest.next() {
            match flag.as_str() {
                "--port" => match rest.next().and_then(|p| p.parse().ok()) {
                    Some(p) => options.port = p,
                    None => {
                        eprintln!("{USAGE}");
                        return ExitCode::from(2);
                    }
                },
                "--allow-origin" => match rest
                    .next()
                    .map(|u| (u, loremaster_mcp::http::origin_of(u)))
                {
                    Some((_, Some(o))) => options.allow_origins.push(o),
                    Some((u, None)) => {
                        eprintln!("error: {u} is not an origin, for example https://example.com");
                        return ExitCode::from(2);
                    }
                    None => {
                        eprintln!("{USAGE}");
                        return ExitCode::from(2);
                    }
                },
                "--no-open" => options.open = false,
                _ => {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
            }
        }
        // `--bootstrap`: a new folder in the temporary directory, for a story that starts in the editor.
        let folder = if dir == "--bootstrap" {
            std::env::temp_dir().join(format!("loremaster-bootstrap-{}", std::process::id()))
        } else {
            Path::new(dir).to_path_buf()
        };
        if let Err(e) = std::fs::create_dir_all(&folder) {
            eprintln!("error: cannot make {}: {e}", folder.display());
            return ExitCode::FAILURE;
        }
        return match loremaster_mcp::http::serve_http(&folder, &options) {
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
    for file in loremaster_project::scaffold(title) {
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
        "New story in {}. Play it with `loremaster play {}`.",
        dir.display(),
        dir.display()
    );
    ExitCode::SUCCESS
}

/// Let the AI expand an idea into the first rooms of the new story in `dir`, as a proposal.
fn seed(dir: &Path, idea: &str) -> ExitCode {
    println!("✦ Drafting the first rooms with AI…");
    let job = loremaster_mcp::seed_folder(dir, idea, |e| {
        if e.kind != "text" {
            println!("  {}: {}", e.kind, e.text);
        }
    });
    match job {
        Ok(job) if !job.proposals.is_empty() => {
            if let Some(result) = &job.result {
                println!("\n{}", result.trim());
            }
            if let Some(cost) = job.cost_usd {
                println!("\nCost: about ${cost:.2} (an estimate with a Claude plan login).");
            }
            println!(
                "\nProposal {} is ready. Start `loremaster serve {}` and accept it in the editor.",
                job.proposals.join(", "),
                dir.display()
            );
            ExitCode::SUCCESS
        }
        Ok(job) => {
            eprintln!(
                "error: the AI made no proposal ({:?}). The story stays empty.",
                job.status
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
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

/// Walk all reachable states and print what the walk found.
fn explore(project: Project) -> ExitCode {
    let world = match World::compile(project.story) {
        Ok(w) => Arc::new(w),
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = loremaster_runtime::explore(&Game::new(world), Budget::default());
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
    for t in &result.threads {
        if !t.paid_off {
            println!(
                "Thread {}: no explored path shows its payoff ({}).",
                t.thread,
                t.payoffs.join(", ")
            );
        } else if !t.payoff_before_setup.is_empty() {
            println!(
                "Thread {}: the payoff shows before any setup after {}.",
                t.thread,
                t.payoff_before_setup.join(", ")
            );
        }
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
            BlockKind::Said => println!("{}", wrap(&format!("> {text}"), 78)),
            _ => println!("{}", wrap(&text, 78)),
        }
    }
    if !turn.choices.is_empty() {
        println!();
        for c in &turn.choices {
            println!(
                "{}",
                wrap(
                    &format!("{}. {}", c.index + 1, emphasis(&c.text, style)),
                    78
                )
            );
        }
        println!("(Type a number, or \"leave\".)");
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
