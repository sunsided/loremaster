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

fn play(files: Vec<SourceFile>, cmd: &str) -> String {
    let project = load(files).unwrap();
    let found = check(&project);
    assert!(found.is_empty(), "{found:#?}");
    let mut game = Session::new(Arc::new(World::compile(project.story).unwrap()));
    authoring::start(&mut game);
    say(&mut game, cmd)
}

#[test]
fn an_exact_rule_that_already_passed_does_not_hold_a_star_rule_back() {
    // `exact` comes first and does not hold; `light` makes it hold, too late.
    let rules = r#"
- id: exact
  on: { verb: [push, darkness] }
  when: lit
  instead: true
  priority: 3
  then: [{ say: "Exact." }]
- id: light
  on: { verb: ["*", darkness] }
  instead: true
  priority: 2
  then: [{ set: { lit: true } }]
- id: late
  on: { verb: ["*", darkness] }
  instead: true
  priority: 1
  then: [{ say: "Late." }]
"#;
    let mut files = files(rules);
    files[0].text =
        "title: Dark\nstart: cellar\nvars:\n  - { id: lit, type: bool, initial: false }\n".into();
    let text = play(files, "push darkness");
    assert!(text.contains("Late.") && !text.contains("Exact."), "{text}");
}

#[test]
fn a_machine_step_that_cannot_run_does_not_hold_a_star_rule_back() {
    // The machine takes one step per event: after `dim`, `bright` holds but
    // does not run. `late` sorts after the machine's steps, so this covers
    // an exact step ahead of it; the `stepped` guard is for an order with the
    // step after the `*` rule, which only the builder can give.
    let machine = r#"
start: dark
states: [{ name: dark }, { name: dim }, { name: bright }]
steps:
  - { from: dark, on: { verb: ["*", darkness] }, to: dim, instead: true, say: "Dim." }
  - { from: dim, on: { verb: [push, darkness] }, to: bright, instead: true, say: "Bright." }
"#;
    let rules = r#"
- id: late
  on: { verb: ["*", darkness] }
  instead: true
  priority: -1
  then: [{ say: "Late." }]
"#;
    let mut files = files(rules);
    files.push(SourceFile {
        path: "machines/glow.yaml".into(),
        text: machine.into(),
    });
    let text = play(files, "push darkness");
    assert!(
        text.contains("Dim.") && text.contains("Late.") && !text.contains("Bright."),
        "{text}"
    );
}

#[test]
fn a_story_verb_cannot_be_named_star() {
    let mut files = files(RULES);
    files.push(SourceFile {
        path: "verbs/star.yaml".into(),
        text: "id: \"*\"\nlabel: Star\nwords: [star]\n".into(),
    });
    let found = check(&load(files).unwrap());
    assert!(
        found
            .iter()
            .any(|d| d.to_string().contains("rules that match any verb")),
        "{found:#?}"
    );
}
