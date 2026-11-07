use assert2::check;
use loremaster_project::{Severity, SourceFile, check, load};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

fn diagnostics(dialogue: &str) -> Vec<loremaster_project::Diagnostic> {
    let files = vec![
        file(
            "story.yaml",
            "title: Talk\nstart: bar\nrelations: [trust]\nvars:\n  - { id: known, type: bool, initial: false }\n",
        ),
        file("rooms/bar.yaml", "name: Bar\n"),
        file("things/bb.yaml", "name: man\nlocation: { room: bar }\n"),
        file("dialogues/bb.yaml", dialogue),
    ];
    check(&load(files).expect("loads"))
}

const GOOD: &str = r#"with: bb
start: [again, meet]
nodes:
  meet:
    say: "Sit."
    options:
      - { say: "Who?", go: names, relation: { trust: 1 } }
      - { say: "Bye." }
  names:
    say: "Blackbeard."
    set: { known: true }
    options:
      - { say: "Back.", go: back }
  again:
    when: bb.talked > 1
    say: "Again."
    options:
      - { say: "Who?", go: names }
"#;

#[test]
fn a_good_dialogue_has_no_problems() {
    check!(diagnostics(GOOD) == vec![]);
}

#[test]
fn a_missing_node_is_an_error_from_the_dialogue_check() {
    // dramaturge-syntax's check finds it, at the `go` of the option.
    let d = diagnostics(&GOOD.replace("go: names, relation", "go: nmes, relation"));
    check!(d.len() == 1, "{d:#?}");
    check!(d[0].severity == Severity::Error);
    check!(d[0].file == "dialogues/bb.yaml");
    check!(d[0].at == "nodes.meet.options[0].go");
    check!(d[0].message == "no node `nmes`");
}

#[test]
fn an_unknown_speaker_of_a_node_is_an_error() {
    let d =
        diagnostics(&GOOD.replace("    say: \"Sit.\"", "    speaker: ghost\n    say: \"Sit.\""));
    check!(d.len() == 1, "{d:#?}");
    check!(d[0].at == "nodes.meet.speaker");
    check!(d[0].message == "unknown thing `ghost`");
}

#[test]
fn a_terminal_node_does_not_hide_other_problems() {
    // A node without options ends the talk; the other checks still run.
    let terminal = GOOD.replace("    options:\n      - { say: \"Back.\", go: back }\n", "");
    assert!(terminal != GOOD, "the replacement must apply");
    let d = diagnostics(&terminal.replace("bb.talked > 1", "bb.trsut > 1"));
    check!(d.len() == 1, "{d:#?}");
    check!(d[0].at == "nodes.again.when");
}

#[test]
fn conditions_and_outcomes_are_checked() {
    let d = diagnostics(&GOOD.replace("bb.talked > 1", "bb.trsut > 1"));
    check!(d[0].at == "nodes.again.when");
    check!(
        d[0].message.contains("did you mean `bb.trust`?"),
        "{}",
        d[0].message
    );
    let d = diagnostics(&GOOD.replace("set: { known: true }", "set: { known: 3 }"));
    check!(d[0].message == "cannot set `known` to `3`");
    let d = diagnostics(&GOOD.replace("relation: { trust: 1 }", "relation: { fear: 1 }"));
    check!(d[0].message.starts_with("unknown relation `fear`"));
}

#[test]
fn a_talk_that_loops_back_is_no_problem() {
    // meet -> names -> meet is a cycle; talks loop on purpose.
    let looping = GOOD.replace(
        "{ say: \"Back.\", go: back }",
        "{ say: \"Back.\", go: meet }",
    );
    assert!(looping != GOOD, "the replace must build the loop");
    let d = diagnostics(&looping);
    check!(d == vec![], "{d:#?}");
}

#[test]
fn a_node_that_no_start_reaches_is_reported_but_a_start_is_not() {
    let orphan = format!("{GOOD}  lost:\n    say: \"Nobody comes here.\"\n");
    let d = diagnostics(&orphan);
    check!(d.len() == 1, "{d:#?}");
    check!(d[0].at == "nodes.lost");
    check!(d[0].message.contains("unreachable"), "{}", d[0].message);
}
