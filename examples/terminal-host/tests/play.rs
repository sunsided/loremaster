//! Plays stories through the binary, over piped input, as a player would.

use loremaster_model::WalkStep;
use loremaster_project::{load, read_dir};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

/// Run the host on a story with these lines as input. Gives stdout.
fn host(story: &str, flags: &[&str], lines: &[String]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_terminal-host"))
        .arg(example(story))
        .args(flags)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(lines.join("\n").as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

fn lighthouse_walkthrough() -> Vec<String> {
    let files = read_dir(&example("lighthouse")).unwrap();
    let project = load(files).unwrap();
    let (_, win) = project
        .walkthroughs
        .iter()
        .find(|(name, _)| name.contains("win"))
        .unwrap();
    let commands = win.steps.iter().map(|s| match s {
        WalkStep::Command(c) | WalkStep::Checked { command: c, .. } => c.clone(),
    });
    commands.collect()
}

const ENDING: &str = "*** The beacon is lit";

#[test]
fn the_walkthrough_reaches_the_ending() {
    let out = host("lighthouse", &[], &lighthouse_walkthrough());
    assert!(out.contains(ENDING), "{out}");
}

#[test]
fn a_save_file_carries_the_game_to_another_process() {
    let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("terminal-host.save.json");
    let file = file.to_str().unwrap();
    let mut lines = lighthouse_walkthrough();
    // Save after the matches, before the beacon is lit.
    let at = lines
        .iter()
        .position(|c| c == "take matches")
        .expect("the walkthrough takes the matches");
    let rest = lines.split_off(at + 1);

    lines.push(format!(":save {file}"));
    let first = host("lighthouse", &[], &lines);
    assert!(first.contains("Saved."), "{first}");
    assert!(!first.contains(ENDING), "{first}");

    let mut lines = vec![format!(":load {file}")];
    lines.extend(rest);
    let second = host("lighthouse", &[], &lines);
    assert!(second.contains("Restored."), "{second}");
    assert!(second.contains(ENDING), "{second}");
}

#[test]
fn a_bad_load_keeps_the_game() {
    let lines = [":load /no/such/file".to_string(), "look".to_string()];
    let out = host("lighthouse", &[], &lines);
    assert!(out.contains("Can not load"), "{out}");
    assert!(out.matches("== ").count() >= 2, "{out}");
}

#[test]
fn the_player_answers_a_roll_with_own_dice() {
    let lines = ["roll your character", "nine", "3", "7", "3"].map(String::from);
    let out = host("salt-tower", &["--own-dice"], &lines);
    assert!(
        out.contains("(Roll 1d6 and type the total, 1 to 6.)"),
        "{out}"
    );
    assert!(out.contains("Type the total of 1d6."), "{out}");
    assert!(out.contains("SKILL 9, STAMINA 19, LUCK 9."), "{out}");
}

#[test]
fn the_player_answers_a_choice_and_an_action() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lever");
    for sub in ["rooms", "things", "verbs", "rules"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let story = [
        (
            "story.yaml",
            "title: Lever\nstart: hall\nvars:\n  - { id: path, type: { enum: [none, left, right] }, initial: none }\n",
        ),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall with a lever.\"]\n",
        ),
        (
            "things/lever.yaml",
            "name: lever\nlocation: { room: hall }\n",
        ),
        ("verbs/pull.yaml", "label: Pull\nwords: [pull]\n"),
        (
            "rules/lever.yaml",
            "- id: pull_lever\n  on: { verb: [pull, lever] }\n  instead: true\n  then:\n    - await: { choice: [left, right], into: path }\n    - await: { action: open_sluice, results: [done] }\n    - say: \"You go {path}.\"\n",
        ),
    ];
    for (path, text) in story {
        std::fs::write(dir.join(path), text).unwrap();
    }
    let lines = ["pull lever", "9", "2", "done"].map(String::from);
    let out = host(dir.to_str().unwrap(), &[], &lines);
    assert!(out.contains("Type a number from 1 to 2."), "{out}");
    assert!(out.contains("the action open_sluice"), "{out}");
    assert!(out.contains("You go right."), "{out}");
}

#[test]
fn the_host_stays_on_the_public_session_api() {
    let source = include_str!("../src/main.rs").to_lowercase();
    for word in ["authoring", "unstable", "::game", "game::"] {
        assert!(!source.contains(word), "the host uses {word}");
    }
}
