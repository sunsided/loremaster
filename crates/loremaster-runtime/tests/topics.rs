//! `ask <person> about <topic>` and `show <thing> to <person>` (#169): the
//! parser fills the topic slot of the Command record, and rules match it.

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, World, authoring};
use std::sync::Arc;

const RULES: &str = r#"
- id: lantern
  on: { verb: ask, direct: bert, topic: lantern }
  then: [{ say: "Two lights on the gate, sir." }]
- id: lantern_story
  on: { verb: ask, direct: bert, topic: "two lights" }
  then: [{ say: "Then one." }]
- id: shown
  on: { verb: show, direct: boots, indirect: bert }
  then: [{ say: "Those are Mr Marsh's, sir." }]
"#;

fn files() -> Vec<SourceFile> {
    [
        ("story.yaml", "title: Topics\nstart: hut\n"),
        ("rooms/hut.yaml", "name: Hut\ndescription: [\"A hut.\"]\n"),
        ("things/bert.yaml", "name: Bert\nlocation: { room: hut }\n"),
        (
            "things/boots.yaml",
            "name: boots\nlocation: { room: hut }\nportable: true\n",
        ),
        (
            "verbs/ask.yaml",
            "label: Ask\nwords: [ask]\ndefault: \"Bert shrugs.\"\n",
        ),
        (
            "verbs/show.yaml",
            "label: Show\nwords: [show]\nslots: two\npreposition: to\ndefault: \"Bert shrugs.\"\n",
        ),
        ("rules/rules.yaml", RULES),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into()
}

fn say(game: &mut Session, text: &str) -> String {
    authoring::play(game, Input::Text(text.into())).plain_text()
}

#[test]
fn ask_about_and_show_to_are_playable() {
    let project = load(files()).unwrap();
    let found = check(&project);
    assert!(found.is_empty(), "{found:#?}");
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world.clone());
    authoring::start(&mut game);
    let lantern = say(&mut game, "ask Bert about the Lantern");
    assert!(lantern.contains("Two lights"), "{lantern}");
    let story = say(&mut game, "ask bert about two lights");
    assert!(
        story.contains("Then one.") && !story.contains("Two lights"),
        "{story}"
    );
    let other = say(&mut game, "ask bert about the weather");
    assert!(other.contains("Bert shrugs."), "{other}");
    let shown = say(&mut game, "show the boots to bert");
    assert!(shown.contains("Mr Marsh's"), "{shown}");

    // The explorer asks about the topics that the rules name.
    let found = explore(world, Budget::default());
    assert!(found.never_fired.is_empty(), "{found:#?}");
}

#[test]
fn a_topic_that_the_parser_never_gives_is_reported() {
    let mut files = files();
    files
        .iter_mut()
        .find(|f| f.path == "rules/rules.yaml")
        .unwrap()
        .text = RULES.replace("topic: lantern", "topic: \"the lantern\"");
    let found = check(&load(files).unwrap());
    assert!(
        found
            .iter()
            .any(|d| d.to_string().contains("write `topic: lantern`")),
        "{found:#?}"
    );
}

#[test]
fn topics_that_no_command_can_have_are_reported() {
    for (topic, rule, message) in [
        ("lantern", "verb: look", "`look` takes no topic"),
        (
            "the",
            "verb: ask, direct: bert",
            "never gives an empty topic",
        ),
        ("Über", "verb: ask, direct: bert", "write `topic: über`"),
    ] {
        let mut files = files();
        files
            .iter_mut()
            .find(|f| f.path == "rules/rules.yaml")
            .unwrap()
            .text = format!("- id: r\n  on: {{ {rule}, topic: \"{topic}\" }}\n  then: []\n");
        let found = check(&load(files).unwrap());
        assert!(
            found.iter().any(|d| d.to_string().contains(message)),
            "{topic}: {found:#?}"
        );
    }
}
