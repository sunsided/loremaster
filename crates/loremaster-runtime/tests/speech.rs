//! Speech segments (P3-15, #111): a line is a text or a list of narration and
//! speech. The renderer quotes speech, joins the segments with one space and
//! gives each speech run its speaker.

use loremaster_project::{Project, Severity, SourceFile, check, load};
use loremaster_runtime::authoring;
use loremaster_runtime::{BlockKind, Input, Run, Session, Turn, World};
use std::sync::Arc;

const STORY: &str = r#"
title: Speech
start: kitchen
vars:
  - { id: late, type: bool, initial: false }
"#;

const KITCHEN: &str = r#"
name: Kitchen
description: ["A warm kitchen."]
"#;

const KEEPER: &str = r#"
name: lock-keeper
aliases: [keeper]
location: { room: kitchen }
rules:
  - id: accuse
    on: { verb: [push, keeper] }
    instead: true
    then:
      - say: ["The lock-keeper shrugs.", { says: "Prove it,", by: keeper }, { narrate: "he says." }]
"#;

const BELL: &str = r#"
name: bell
location: { room: kitchen }
rules:
  - id: ring
    on: { verb: [push, bell] }
    instead: true
    then:
      - set: { late: true }
      - say: "Ding."
"#;

const RULES: &str = r#"
- id: quiet
  on: { verb: [wait] }
  instead: true
  then:
    - line:
        - { narrate: "The kitchen is quiet.", when: not late }
        - { narrate: "It is late.", when: late }
        - { says: "Go to bed.", by: keeper, when: late }
        - { says: "{late ? | }" }
"#;

const DIALOGUE: &str = r#"
with: keeper
start: [hi]
nodes:
  hi:
    say: ["He looks up.", { says: "Yes?" }]
    options:
      - line: [{ says: "Who are you?" }]
        go: name
      - say: "Leave."
        go: end
  name:
    say: [{ says: "The keeper." }, { narrate: "He points at the bell." }, { says: "Ring it,", by: player }]
"#;

fn files(extra: &[(&str, &str)]) -> Vec<SourceFile> {
    [
        ("story.yaml", STORY),
        ("rooms/kitchen.yaml", KITCHEN),
        ("things/keeper.yaml", KEEPER),
        ("things/bell.yaml", BELL),
        ("rules/quiet.yaml", RULES),
        ("dialogues/keeper.yaml", DIALOGUE),
    ]
    .iter()
    .chain(extra)
    .map(|(path, text)| SourceFile {
        path: (*path).into(),
        text: (*text).into(),
    })
    .collect()
}

fn project(extra: &[(&str, &str)]) -> Project {
    load(files(extra)).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn game() -> Session {
    let project = project(&[]);
    let problems = check(&project);
    assert!(problems.is_empty(), "{problems:#?}");
    let mut game = Session::new(Arc::new(World::compile(project.story).unwrap()));
    authoring::start(&mut game);
    game
}

fn input(session: &mut Session, text: &str) -> Turn {
    authoring::play(session, Input::Text(text.into()))
}

/// The runs of the first block of a kind.
fn runs(turn: &Turn, kind: BlockKind) -> Vec<Run> {
    turn.blocks
        .iter()
        .find(|b| b.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} block: {turn:#?}"))
        .runs
        .clone()
}

/// The text of the runs, and each piece of speech with its speaker. A thing
/// name in a run becomes a run of its own (a link), with the same speaker,
/// so the runs of one speech join here.
fn spoken(runs: &[Run]) -> (String, Vec<(String, &str)>) {
    let text = runs.iter().map(|r| r.text.as_str()).collect();
    let mut speech: Vec<(String, &str)> = Vec::new();
    let mut last: Option<&str> = None;
    for r in runs {
        match r.speaker.as_deref() {
            Some(who) if last == Some(who) => speech.last_mut().unwrap().0.push_str(&r.text),
            Some(who) => speech.push((r.text.clone(), who)),
            None => {}
        }
        last = r.speaker.as_deref();
    }
    (text, speech)
}

fn said(pairs: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    pairs.iter().map(|(t, w)| ((*t).to_string(), *w)).collect()
}

#[test]
fn speech_gets_quotes_and_segments_join_with_one_space() {
    let mut game = game();
    let turn = input(&mut game, "push keeper");
    assert!(
        turn.plain_text()
            .contains(r#"The lock-keeper shrugs. "Prove it," he says."#),
        "{}",
        turn.plain_text()
    );
    let runs = runs(&turn, BlockKind::Narration);
    let (text, speech) = spoken(&runs);
    assert_eq!(text, r#"The lock-keeper shrugs. "Prove it," he says."#);
    assert_eq!(speech, said(&[("\"Prove it,\"", "keeper")]));
    // Each segment's run keeps the rule as its source.
    let quoted = runs.iter().find(|r| r.speaker.is_some()).unwrap();
    assert!(
        quoted
            .source
            .as_ref()
            .is_some_and(|s| s.owner == "rule:accuse")
    );
}

#[test]
fn a_plain_line_renders_as_before() {
    let mut game = game();
    let turn = input(&mut game, "push bell");
    let runs = runs(&turn, BlockKind::Narration);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "Ding.");
    assert_eq!(runs[0].speaker, None);
}

#[test]
fn a_segment_with_when_shows_only_while_it_holds() {
    let mut game = game();
    let early = input(&mut game, "wait").plain_text();
    assert!(early.ends_with("The kitchen is quiet."), "{early}");
    assert!(!early.contains("late") && !early.contains('"'), "{early}");
    input(&mut game, "push bell");
    let late = input(&mut game, "wait");
    assert!(
        late.plain_text().ends_with(r#"It is late. "Go to bed.""#),
        "{}",
        late.plain_text()
    );
    // The last segment renders to nothing: no empty quotes, no extra space.
    assert!(!late.plain_text().contains("\"\""));
}

#[test]
fn the_speaker_of_a_node_is_the_dialogues_thing_and_of_an_option_the_player() {
    let mut game = game();
    let talk = input(&mut game, "talk to keeper");
    assert_eq!(
        spoken(&runs(&talk, BlockKind::Speech)),
        (
            r#"He looks up. "Yes?""#.to_string(),
            said(&[("\"Yes?\"", "keeper")])
        )
    );
    let choices: Vec<&str> = talk.choices.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(choices, ["\"Who are you?\"", "Leave."]);

    let answer = input(&mut game, "1");
    assert_eq!(
        spoken(&runs(&answer, BlockKind::Said)).1,
        said(&[("\"Who are you?\"", "player")])
    );
    assert_eq!(
        spoken(&runs(&answer, BlockKind::Speech)),
        (
            r#""The keeper." He points at the bell. "Ring it,""#.to_string(),
            said(&[("\"The keeper.\"", "keeper"), ("\"Ring it,\"", "player")])
        )
    );
}

#[test]
fn the_speaker_is_left_out_of_json_when_there_is_none() {
    let mut game = game();
    let turn = input(&mut game, "push keeper");
    let runs = runs(&turn, BlockKind::Narration);
    let json = serde_json::to_value(&runs).unwrap();
    assert!(json[0].get("speaker").is_none(), "{json}");
    let quoted = runs.iter().position(|r| r.speaker.is_some()).unwrap();
    assert_eq!(json[quoted]["speaker"], "keeper", "{json}");
}

fn diagnostics(extra: &[(&str, &str)]) -> Vec<(Severity, String, String)> {
    check(&project(extra))
        .into_iter()
        .map(|d| (d.severity, d.at, d.message))
        .collect()
}

#[test]
fn a_bare_string_after_speech_gets_a_warning() {
    let problems = diagnostics(&[(
        "rules/more.yaml",
        r#"
- id: shrug
  on: { verb: [wait] }
  then:
    - say: [{ says: "Prove it," }, "he says.", { says: "Fine." }, { narrate: "he adds." }]
"#,
    )]);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    let (severity, at, message) = &problems[0];
    assert_eq!(*severity, Severity::Warning);
    assert!(at.ends_with("then[0].say[1]"), "{at}");
    assert!(message.contains("narrate"), "{message}");
}

#[test]
fn the_check_examines_each_segment() {
    let problems = diagnostics(&[
        (
            "rules/bad.yaml",
            r#"
- id: bad
  on: { verb: [wait] }
  then:
    - say:
        - "{nope}"
        - { says: "Hm.", by: keeper, when: lte }
        - { says: "No.", by: keper }
"#,
        ),
        (
            "dialogues/bell.yaml",
            r#"
with: bell
start: [ding]
nodes:
  ding:
    say: [{ says: "Ding.", by: nobody }]
    options:
      - say: [{ says: "{nope}" }]
"#,
        ),
    ]);
    let errors: Vec<(&str, &str)> = problems
        .iter()
        .filter(|p| p.0 == Severity::Error)
        .map(|p| (p.1.as_str(), p.2.as_str()))
        .collect();
    assert_eq!(errors.len(), 5, "{problems:#?}");
    let has = |at: &str, text: &str| {
        errors
            .iter()
            .any(|(a, m)| a.ends_with(at) && m.contains(text))
    };
    assert!(has("then[0].say[0]", "nope"), "{errors:#?}");
    assert!(has("then[0].say[1].when", "lte"), "{errors:#?}");
    assert!(
        has("then[0].say[2].by", "did you mean `keeper`"),
        "{errors:#?}"
    );
    assert!(
        has("nodes.ding.say[0].by", "unknown speaker `nobody`"),
        "{errors:#?}"
    );
    // Reported once, though the dialogue compiler finds it too.
    assert!(has("nodes.ding.options[0].say[0]", "nope"), "{errors:#?}");
}

#[test]
fn line_is_an_alias_of_say() {
    let project = project(&[]);
    let quiet = project
        .story
        .rules
        .iter()
        .find(|r| r.id.as_str() == "quiet")
        .unwrap();
    let json = serde_json::to_value(&quiet.then[0]).unwrap();
    // The model writes `say`, whichever key the file used.
    assert!(
        json.get("say").is_some() && json.get("line").is_none(),
        "{json}"
    );

    let both = files(&[(
        "rules/both.yaml",
        "- id: both\n  on: { verb: [wait] }\n  then:\n    - { say: \"A.\", line: \"B.\" }\n",
    )]);
    assert!(load(both).is_err());
}
