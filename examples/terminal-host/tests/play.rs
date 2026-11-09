//! Plays stories through the binary, over piped input, as a player would.

use loremaster_model::WalkStep;
use loremaster_project::{load, read_dir};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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

#[test]
fn a_sound_shows_as_its_caption() {
    let lines = ["pull rope", "pull rope", "open gate"].map(String::from);
    let out = host("gatehouse", &[], &lines);
    assert!(out.contains("[The gate creaks on its hinges.]"), "{out}");
}

/// A host on a private copy of a story, driven one prompt at a time: the host
/// polls before each prompt, so an edit made between two prompts is seen at
/// the next one, whatever the timing.
struct Watched {
    story: PathBuf,
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

impl Watched {
    /// Start on a copy of the lighthouse. Returns the host at its first prompt.
    fn start(name: &str) -> (Self, String) {
        let story = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&story);
        copy_dir(&example("lighthouse"), &story);
        let mut child = Command::new(env!("CARGO_BIN_EXE_terminal-host"))
            .arg(&story)
            .arg("--watch")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut host = Self {
            story,
            child,
            stdin,
            stdout,
        };
        let out = host.until_prompt();
        (host, out)
    }

    /// Everything the host prints up to its next prompt.
    fn until_prompt(&mut self) -> String {
        let mut out = Vec::new();
        let mut buf = [0; 4096];
        while !out.ends_with(b"\n> ") {
            let n = self.stdout.read(&mut buf).unwrap();
            let so_far = String::from_utf8_lossy(&out);
            assert!(n > 0, "the host ended before a prompt: {so_far}");
            out.extend_from_slice(&buf[..n]);
        }
        String::from_utf8(out).unwrap()
    }

    fn send(&mut self, line: &str) -> String {
        writeln!(self.stdin, "{line}").unwrap();
        self.until_prompt()
    }

    fn edit(&self, file: &str, change: impl FnOnce(String) -> String) {
        let path = self.story.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, change(text)).unwrap();
    }

    fn quit(mut self) {
        writeln!(self.stdin, ":quit").unwrap();
        assert!(self.child.wait().unwrap().success());
    }
}

const COTTAGE: &str = "One room, smelling of tar and pipe smoke.";
const NEW_COTTAGE: &str = "One room, smelling of fresh paint.";

#[test]
fn watch_shows_an_edit_and_keeps_the_place() {
    // Away from the start room, which a lost place would fall back to.
    let (mut host, _) = Watched::start("watch-edit");
    let moved = host.send("n");
    assert!(moved.contains(COTTAGE), "{moved}");

    host.edit("rooms/cottage.yaml", |t| t.replace(COTTAGE, NEW_COTTAGE));
    let out = host.send("look");
    assert!(out.contains("story reloaded"), "{out}");
    let reloaded = out.split("story reloaded").nth(1).unwrap();
    assert!(reloaded.contains("== Keeper's Cottage =="), "{out}");
    assert!(reloaded.contains(NEW_COTTAGE), "{out}");

    // One reload for one edit, and the new world stays.
    let again = host.send("look");
    assert!(!again.contains("story reloaded"), "{again}");
    assert!(again.contains(NEW_COTTAGE), "{again}");
    host.quit();
}

#[test]
fn watch_reports_a_broken_file_and_plays_on() {
    let (mut host, _) = Watched::start("watch-broken");
    host.send("n");

    host.edit("rooms/cottage.yaml", |_| "name: [\n".to_string());
    let out = host.send("look");
    assert!(!out.contains("story reloaded"), "{out}");
    assert!(out.contains("The story has errors"), "{out}");
    assert!(out.contains("cottage.yaml"), "{out}");

    // Still the old world, and the errors are not repeated.
    let next = host.send("look");
    assert!(next.contains(COTTAGE), "{next}");
    assert!(!next.contains("The story has errors"), "{next}");
    host.quit();
}
