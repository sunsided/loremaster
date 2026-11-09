//! The wildcard verb in rule patterns (#192): `on: { verb: ["*", darkness] }`
//! matches any verb on the thing, and an exact-verb rule that fires in the
//! same scope and phase holds the `*` rule back.

use loremaster_project::{SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, World, authoring};
use std::sync::Arc;

const RULES: &str = r#"
- id: any_darkness
  on: { verb: ["*", darkness] }
  instead: true
  then: [{ say: "The darkness swallows it." }]
- id: examine_darkness
  on: { verb: [examine, darkness] }
  instead: true
  then: [{ say: "It is very dark." }]
- id: once_on_lamp
  on: { verb: ["*", lamp] }
  once: true
  then: [{ say: "The lamp flickers." }]
"#;

fn files(rules: &str) -> Vec<SourceFile> {
    [
        ("story.yaml", "title: Dark\nstart: cellar\n"),
        (
            "rooms/cellar.yaml",
            "name: Cellar\ndescription: [\"A cellar.\"]\n",
        ),
        (
            "things/darkness.yaml",
            "name: darkness\nlocation: { room: cellar }\n",
        ),
        (
            "things/lamp.yaml",
            "name: lamp\nlocation: { room: cellar }\nportable: true\n",
        ),
        ("rules/rules.yaml", rules),
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
fn a_star_rule_matches_any_verb_unless_an_exact_rule_fires() {
    let project = load(files(RULES)).unwrap();
    assert!(check(&project).is_empty(), "{:#?}", check(&project));
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut game = Session::new(world.clone());
    authoring::start(&mut game);

    for cmd in ["push darkness", "take darkness", "open darkness"] {
        let text = say(&mut game, cmd);
        assert!(text.contains("swallows it"), "{cmd}: {text}");
    }
    let examined = say(&mut game, "examine darkness");
    assert!(examined.contains("very dark"), "{examined}");
    assert!(!examined.contains("swallows"), "{examined}");

    // `once` spans every verb of the one rule.
    let first = say(&mut game, "push lamp");
    assert!(first.contains("flickers"), "{first}");
    let second = say(&mut game, "take lamp");
    assert!(!second.contains("flickers"), "{second}");

    // The explorer tries the verbs of a `*` rule.
    let found = explore(world, Budget::default());
    assert!(found.never_fired.is_empty(), "{found:#?}");
}

#[test]
fn an_agenda_step_cannot_run_any_verb() {
    let mut files = files(RULES);
    files.push(SourceFile {
        path: "things/keeper.yaml".into(),
        text: "name: keeper\nlocation: { room: cellar }\n".into(),
    });
    files.push(SourceFile {
        path: "agendas/keeper.yaml".into(),
        text: "actor: keeper\nsteps:\n  - { at: \"00:00\", do: [\"*\", lamp] }\n".into(),
    });
    let found = check(&load(files).unwrap());
    assert!(
        found
            .iter()
            .any(|d| d.to_string().contains("`*` is for the events of rules")),
        "{found:#?}"
    );
}
