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
fn a_missing_node_is_an_error_from_the_graph_validator() {
    let d = diagnostics(&GOOD.replace("go: names, relation", "go: nmes, relation"));
    check!(d.len() == 1, "{d:#?}");
    check!(d[0].severity == Severity::Error);
    check!(d[0].file == "dialogues/bb.yaml");
    check!(d[0].at == "nodes.meet");
    check!(d[0].message.contains("targets nonexistent node 'nmes'"));
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
