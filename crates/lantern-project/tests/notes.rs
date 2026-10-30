use assert2::check;
use lantern_project::{Severity, SourceFile, check, load, open_todos, threads};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

fn project(extra: &[(&str, &str)]) -> lantern_project::Project {
    let mut files = vec![
        file(
            "story.yaml",
            "title: Notes\nstart: shore\nnotes:\n  - The premise: a lighthouse without a keeper.\n",
        ),
        file(
            "rooms/shore.yaml",
            r#"name: The Shore
notes:
  - Keep it eerie.
  - todo: Decide if the ferryman comes back.
  - { todo: Name the gull., done: true }
description:
  - id: gull
    text: "A gull watches you."
    notes:
      - { text: First hint at the key., thread: lost_key, role: setup }
"#,
        ),
    ];
    files.extend(extra.iter().map(|(p, t)| file(p, t)));
    load(files).expect("loads")
}

const THREAD: &str = "name: The lost key\nsummary: Hal hid the key.\nnotes:\n  - todo: A second hint on the cliff.\n";

#[test]
fn notes_load_on_story_rooms_and_fragments() {
    let p = project(&[("threads/lost_key.yaml", THREAD)]);
    check!(p.story.notes.len() == 1);
    check!(p.story.rooms[0].notes.len() == 3);
    check!(p.story.rooms[0].description[0].notes.len() == 1);
    check!(p.story.threads[0].id.as_str() == "lost_key");
}

#[test]
fn open_todos_skip_done_ones() {
    let p = project(&[("threads/lost_key.yaml", THREAD)]);
    let todos = open_todos(&p);
    let texts: Vec<_> = todos.iter().map(|n| n.note.text.as_str()).collect();
    check!(
        texts
            == [
                "Decide if the ferryman comes back.",
                "A second hint on the cliff."
            ]
    );
    check!(todos[0].file == "rooms/shore.yaml");
    check!(todos[0].at == "notes[1]");
    check!(todos[0].line == Some(4));
}

#[test]
fn a_thread_collects_its_beats() {
    let p = project(&[("threads/lost_key.yaml", THREAD)]);
    let t = &threads(&p)[0];
    check!(t.setups == 1);
    check!(t.payoffs == 0);
    check!(t.open_todos == 1);
    check!(
        t.beats
            .iter()
            .any(|b| b.fragment_id.as_deref() == Some("gull"))
    );
}

#[test]
fn an_open_thread_without_payoff_warns() {
    let p = project(&[("threads/lost_key.yaml", THREAD)]);
    let diags = check(&p);
    check!(
        diags.iter().all(|d| d.severity == Severity::Warning),
        "{diags:?}"
    );
    check!(
        diags.iter().any(|d| d.message.contains("no payoff")),
        "{diags:?}"
    );
}

#[test]
fn a_dropped_thread_does_not_warn() {
    let dropped = THREAD.replace("name:", "status: dropped\nname:");
    let p = project(&[("threads/lost_key.yaml", &dropped)]);
    check!(check(&p).is_empty());
}

#[test]
fn an_unknown_thread_is_an_error() {
    let p = project(&[]);
    let diags = check(&p);
    check!(
        diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message == "unknown thread `lost_key`"),
        "{diags:?}"
    );
}

#[test]
fn a_bad_note_does_not_load() {
    let files = vec![
        file("story.yaml", "title: x\nstart: a\n"),
        file(
            "rooms/a.yaml",
            "name: A\nnotes:\n  - { text: a, todo: b }\n",
        ),
    ];
    check!(load(files).is_err());
}
