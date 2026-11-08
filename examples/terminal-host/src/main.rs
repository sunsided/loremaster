//! A terminal host of the Session API. See README.md for what a host must do.
//!
//! Lines that start with `:` belong to the host, so they can not collide with
//! the words of a story: `:save <file>`, `:load <file>`, `:quit`.

use loremaster_model::Value;
use loremaster_project::{load, read_dir};
use loremaster_runtime::{
    AwaitRequest, Awaiting, Block, BlockKind, Effect, Input, RESTORE, Session, Status, Step, World,
};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let own_dice = args.iter().any(|a| a == "--own-dice");
    if let Some(flag) = args
        .iter()
        .find(|a| a.starts_with("--") && *a != "--own-dice")
    {
        eprintln!("unknown option {flag}");
        return ExitCode::FAILURE;
    }
    let Some(dir) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: terminal-host <story folder> [--own-dice]");
        return ExitCode::FAILURE;
    };
    let world = match open(Path::new(dir)) {
        Ok(world) => world,
        Err(e) => {
            eprintln!("{dir}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if play(world, own_dice) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn open(dir: &Path) -> Result<Arc<World>, String> {
    let files = read_dir(dir).map_err(|e| e.to_string())?;
    let project = load(files).map_err(|errors| format!("{errors:?}"))?;
    let world = World::compile(project.story).map_err(|e| e.to_string())?;
    Ok(Arc::new(world))
}

/// The loop of a host: show the step, ask what its status asks, send the answer.
/// False when the host met something it can not handle.
fn play(world: Arc<World>, own_dice: bool) -> bool {
    let mut session = Session::new(world.clone());
    session.set_host_rolls(own_dice);
    let mut step = session.start();
    show(&step);
    loop {
        // Only the status says what may come next. Effects are just shown.
        let awaiting = match &step.status {
            Status::Ready => None,
            Status::Await { awaiting } => {
                ask(&awaiting.request);
                Some(awaiting.clone())
            }
            Status::Ended { ending } => {
                // A restored step has no ending effect: show the ending here.
                let shown = step
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::Ending { .. }));
                match ending {
                    None => println!("The story ended in a way it no longer knows."),
                    Some(e) if !shown => println!("\n*** {} ({:?}) ***", e.title, e.kind),
                    Some(_) => {}
                }
                return true;
            }
            // Status is non_exhaustive: a newer runtime can add a variant.
            other => {
                eprintln!("this host does not know the status {other:?}");
                return false;
            }
        };
        print!("\n> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(e) => {
                eprintln!("can not read the input: {e}");
                return false;
            }
        }
        let line = line.trim();
        let next = match line.split_once(' ').unwrap_or((line, "")) {
            ("", _) => continue,
            (":quit", _) => return true,
            (":save", file) => {
                match save(&session, file) {
                    Ok(()) => println!("Saved."),
                    Err(e) => println!("Can not save: {e}"),
                }
                continue;
            }
            (":load", file) => match restore_file(world.clone(), own_dice, file) {
                Ok((restored, first)) => {
                    session = restored;
                    show(&first);
                    // A save made in a talk: the restored step does not list the answers.
                    if let Some(talk) = session.talk() {
                        println!("\n(Still talking: {}. Type a number.)", talk.dialogue);
                        for choice in session.choices() {
                            println!("{}. {}", choice.index + 1, choice.text);
                        }
                    }
                    step = first;
                    continue;
                }
                Err(e) => {
                    println!("Can not load: {e}");
                    continue;
                }
            },
            (word, _) if word.starts_with(':') => {
                println!("Unknown host command {word}. Use :save, :load or :quit.");
                continue;
            }
            _ => match awaiting {
                Some(awaiting) => match answer_await(&mut session, &awaiting, line) {
                    Ok(step) => step,
                    Err(e) => {
                        println!("{e}");
                        continue;
                    }
                },
                None => session.submit(input(&session, line)),
            },
        };
        step = next;
        show(&step);
    }
}

/// In a talk a number picks an answer. All other text goes to the parser,
/// which also knows `undo` and the words that leave a talk.
fn input(session: &Session, line: &str) -> Input {
    match line.parse::<usize>() {
        Ok(n) if n > 0 && session.talk().is_some() => Input::Choice(n - 1),
        _ => Input::Text(line.to_string()),
    }
}

fn answer_await(session: &mut Session, awaiting: &Awaiting, line: &str) -> Result<Step, String> {
    let value = answer(&awaiting.request, line)?;
    // The session can not read a save file: `restore` is the host's to do.
    if matches!(awaiting.request, AwaitRequest::DeathPrompt { .. })
        && value == Value::Text(RESTORE.into())
    {
        return Err("Type :load <file>.".into());
    }
    session.resume(&awaiting.id, value)
}

/// A number or an id for a request with a list of answers. Else a typed value.
fn answer(request: &AwaitRequest, line: &str) -> Result<Value, String> {
    let answers = request.answers();
    if answers.is_empty() {
        return Ok(match (request, line.parse()) {
            (AwaitRequest::Roll { .. }, Ok(total)) => Value::Int(total),
            (AwaitRequest::Roll { dice, .. }, Err(_)) => {
                return Err(format!("Type the total of {dice}."));
            }
            _ => Value::Text(line.to_string()),
        });
    }
    let by_number = line.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
    (by_number.and_then(|i| answers.get(i)))
        .or_else(|| answers.iter().find(|(v, _)| *v == Value::Text(line.into())))
        .map(|(value, _)| value.clone())
        .ok_or_else(|| format!("Type a number from 1 to {}.", answers.len()))
}

/// Show what a rule waits for. Each request needs its own way to answer.
fn ask(request: &AwaitRequest) {
    match request {
        AwaitRequest::Choice { options } => {
            let texts: Vec<_> = options.iter().map(|o| o.text.as_str()).collect();
            list(&texts);
        }
        AwaitRequest::Action {
            name,
            args,
            results,
        } => {
            // A game engine would do the action. Here the player reports it.
            println!("\n(The story asks for the action {name} {args:?}.)");
            let texts: Vec<_> = results.iter().map(|r| r.as_str()).collect();
            list(&texts);
        }
        AwaitRequest::Roll { dice, min, max } => {
            println!("\n(Roll {dice} and type the total, {min} to {max}.)");
        }
        AwaitRequest::WalkTo { room } => {
            // A terminal has no walking: the player says what happened.
            println!("\n(Walk to {room}.)");
            list(&[loremaster_runtime::ARRIVED, loremaster_runtime::REFUSED]);
        }
        AwaitRequest::DeathPrompt { ending, options } => {
            println!("\n(You reached {ending}.)");
            let texts: Vec<_> = options.iter().map(|o| o.as_str()).collect();
            list(&texts);
        }
        other => println!("\n(The story waits: {}.)", other.describe()),
    }
}

fn list(texts: &[&str]) {
    for (i, text) in texts.iter().enumerate() {
        println!("{}. {text}", i + 1);
    }
}

fn show(step: &Step) {
    for effect in &step.effects {
        match effect {
            Effect::Text { block } => show_block(block),
            // Changes of state. A host with a map or an inventory panel uses them.
            Effect::Notice { .. } => {}
            Effect::Choices { talk, choices } => {
                println!("\n(Talking to {}. Type a number.)", talk.name);
                for choice in choices {
                    println!("{}. {}", choice.index + 1, choice.text);
                }
            }
            Effect::Roll { roll } => {
                let who = roll.who.as_deref().unwrap_or("Roll");
                print!("\n[{who}: {} = {}", roll.dice, roll.total);
                if let Some(target) = roll.target {
                    print!(" against {target}");
                }
                if let Some(result) = &roll.result {
                    print!(", {result}");
                }
                println!("]");
            }
            Effect::Ending { ending } => {
                println!("\n*** {} ({:?}) ***", ending.title, ending.kind);
            }
            // Effect is non_exhaustive. A new effect must not stop the game.
            other => eprintln!("this host does not know the effect {other:?}"),
        }
    }
}

fn show_block(block: &Block) {
    let mut text = String::new();
    let mut speaker = None;
    for run in &block.runs {
        // Runs of speech name their speaker. The text has the quotes already.
        if run.speaker.is_some() && run.speaker != speaker {
            text.push_str(&format!("{}: ", run.speaker.as_deref().unwrap_or_default()));
        }
        // Narration between two lines of one speaker names the speaker again.
        speaker.clone_from(&run.speaker);
        text.push_str(&run.text);
    }
    match block.kind {
        // The echo of the player's own line: the terminal has it already.
        BlockKind::Command => {}
        BlockKind::Title => println!("\n== {text} =="),
        BlockKind::Said => println!("\n> {text}"),
        BlockKind::Note
        | BlockKind::Prose
        | BlockKind::Objects
        | BlockKind::Exits
        | BlockKind::Response
        | BlockKind::Narration
        | BlockKind::End
        | BlockKind::Speech
        | BlockKind::Author => println!("\n{text}"),
    }
}

fn save(session: &Session, file: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&session.save()).map_err(|e| e.to_string())?;
    std::fs::write(file, json).map_err(|e| e.to_string())
}

fn restore_file(world: Arc<World>, own_dice: bool, file: &str) -> Result<(Session, Step), String> {
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let json = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let (mut session, loaded) = Session::load(world, json).map_err(|e| e.to_string())?;
    if loaded.story_changed {
        println!("The story has changed. Dropped: {:?}", loaded.dropped);
    }
    // The setting belongs to the session, not to the save.
    session.set_host_rolls(own_dice);
    // `load` returns no step. Restoring its own state shows the room, without a turn.
    let snapshot = session.snapshot();
    let step = session.restore(&snapshot).map_err(|e| e.to_string())?;
    Ok((session, step))
}
