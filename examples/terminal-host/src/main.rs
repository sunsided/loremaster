//! A terminal host of the Session API. See README.md for what a host must do.
//!
//! Lines that start with `:` belong to the host, so they can not collide with
//! the words of a story: `:save <file>`, `:load <file>`, `:quit`.

use loremaster_dsl::{LiveStory, Loremaster};
use loremaster_model::{Value, Walkthrough};
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
    let watched = args.iter().any(|a| a == "--watch");
    if let Some(flag) = args
        .iter()
        .find(|a| a.starts_with("--") && *a != "--own-dice" && *a != "--watch")
    {
        eprintln!("unknown option {flag}");
        return ExitCode::FAILURE;
    }
    let Some(dir) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: terminal-host <story folder> [--own-dice] [--watch]");
        return ExitCode::FAILURE;
    };
    let opened = if watched {
        open_watched(Path::new(dir)).map(|(world, watch)| (world, Some(watch)))
    } else {
        open(Path::new(dir)).map(|world| (world, None))
    };
    let (world, watch) = match opened {
        Ok(opened) => opened,
        Err(e) => {
            eprintln!("{dir}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if play(world, own_dice, watch) {
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

/// A running `--watch`: the source of new worlds, and the hints of the current one.
struct Watch {
    live: LiveStory,
    walkthroughs: Vec<(String, Walkthrough)>,
}

/// Build the first world as `LiveStory::poll` builds the later ones: with the
/// check, so a story that a reload would refuse does not start either.
fn open_watched(dir: &Path) -> Result<(Arc<World>, Watch), String> {
    // Read first: an edit made during the build then shows at the first poll.
    let sources = Loremaster::new().yaml(dir);
    let live = LiveStory::new(sources.clone());
    let (project, warnings) = sources.check().map_err(|found| {
        let lines: Vec<_> = found.iter().map(ToString::to_string).collect();
        lines.join("\n")
    })?;
    for warning in &warnings {
        println!("{warning}");
    }
    let world = World::compile(project.story).map_err(|e| e.to_string())?;
    let missing: Vec<_> = (world.missing_behaviors().iter())
        .map(ToString::to_string)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "--watch can not live-reload this story: this host registers no behaviors, and the story needs {}",
            missing.join(", ")
        ));
    }
    let walkthroughs = project.walkthroughs;
    Ok((Arc::new(world), Watch { live, walkthroughs }))
}

/// Take up a changed story, if there is one. Gives the step that shows the
/// current room again when the session moved to a new world.
fn reload(session: &mut Session, world: &mut Arc<World>, watch: &mut Watch) -> Option<Step> {
    let rebuilt = match watch.live.poll()? {
        Ok(rebuilt) => rebuilt,
        Err(diagnostics) => {
            println!("\nThe story has errors. Still playing the last good version:");
            for diagnostic in &diagnostics {
                println!("{diagnostic}");
            }
            return None;
        }
    };
    let reloaded = match session.reload(rebuilt.world.clone(), &rebuilt.walkthroughs) {
        Ok(reloaded) => reloaded,
        Err(e) => {
            println!("\nThe story needs a rebuild of the host: {e}");
            return None;
        }
    };
    *world = rebuilt.world;
    watch.walkthroughs = rebuilt.walkthroughs;
    println!("\nstory reloaded");
    for warning in &rebuilt.warnings {
        println!("{warning}");
    }
    if !reloaded.dropped.is_empty() {
        println!("Dropped: {}", reloaded.dropped.join(", "));
    }
    // Restoring its own state shows the room without a turn, as after a load.
    let snapshot = session.snapshot();
    match session.restore(&snapshot) {
        Ok(step) => Some(step),
        Err(e) => {
            println!("Can not show the room again: {e}");
            None
        }
    }
}

/// The loop of a host: show the step, ask what its status asks, send the answer.
/// False when the host met something it can not handle.
fn play(mut world: Arc<World>, own_dice: bool, mut watch: Option<Watch>) -> bool {
    let mut session = Session::new(world.clone());
    session.set_host_rolls(own_dice);
    if let Some(watch) = &watch {
        session.set_hints_from(&watch.walkthroughs);
    }
    let mut step = session.start();
    show(&step);
    loop {
        // Only before a prompt: an edit shows with the next command.
        if !matches!(step.status, Status::Ended { .. })
            && let Some(first) = watch
                .as_mut()
                .and_then(|watch| reload(&mut session, &mut world, watch))
        {
            show(&first);
            show_talk(&session);
            step = first;
        }
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
                    // A load gives a session without hints.
                    if let Some(watch) = &watch {
                        session.set_hints_from(&watch.walkthroughs);
                    }
                    show(&first);
                    show_talk(&session);
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

/// A restored step does not list the answers of a talk that is still open.
fn show_talk(session: &Session) {
    if let Some(talk) = session.talk() {
        println!("\n(Still talking: {}. Type a number.)", talk.dialogue);
        for choice in session.choices() {
            println!("{}. {}", choice.index + 1, choice.text);
        }
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
        AwaitRequest::Roll {
            dice,
            min,
            max,
            who,
            ..
        } => {
            let whom = who
                .as_ref()
                .map_or_else(String::new, |w| format!(" for {w}"));
            println!("\n(Roll {dice}{whom} and type the total, {min} to {max}.)");
        }
        AwaitRequest::WalkTo { .. } => {
            // A terminal has no walking: the player says what happened.
            println!("\n({}.)", request.describe());
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
            // This host plays no audio: it shows the caption of a sound.
            Effect::Sound { sound } => {
                if let Some(caption) = &sound.caption {
                    println!("\n{caption}");
                }
            }
            // Effect is non_exhaustive. A new effect must not stop the game.
            other => eprintln!("this host does not know the effect {other:?}"),
        }
    }
}

fn show_block(block: &Block) {
    let mut text = String::new();
    let mut speaker = None;
    let mut emphasis = false;
    for run in &block.runs {
        // Runs of speech name their speaker. The text has the quotes already.
        if run.speaker.is_some() && run.speaker != speaker {
            text.push_str(&format!("{}: ", run.speaker.as_deref().unwrap_or_default()));
        }
        // Narration between two lines of one speaker names the speaker again.
        speaker.clone_from(&run.speaker);
        // A plain terminal has no italics: emphasis keeps its stars, one
        // pair for runs that a link split.
        if run.emphasis != emphasis {
            text.push('*');
            emphasis = run.emphasis;
        }
        text.push_str(&run.text);
    }
    if emphasis {
        text.push('*');
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
    // A story with `reload_rolls: fresh` rolls anew after a load. The value must be
    // fresh and unpredictable on each load; `RandomState` has new OS-seeded keys each time.
    session.reseed(std::hash::BuildHasher::hash_one(
        &std::collections::hash_map::RandomState::new(),
        0,
    ));
    Ok((session, step))
}
