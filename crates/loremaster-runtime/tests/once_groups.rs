//! Once-groups (#225): `once: <key>` lets every rule with that key fire at
//! most once in total. The first rule that fires closes the group.

use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::authoring::{Budget, explore};
use loremaster_runtime::{Input, Session, World, authoring};
use std::sync::Arc;

const RULES: &str = r#"
- id: greet_day
  on: { verb: [examine, keeper] }
  when: "!late"
  once: greet
  then: [{ say: "Good day, stranger." }]
- id: greet_night
  on: { verb: [examine, keeper] }
  when: late
  once: greet
  then: [{ say: "Late for a visit." }]
- id: dusk
  on: { verb: [wait] }
  then: [{ set: { late: true } }]
"#;

fn files(rules: &str) -> Vec<SourceFile> {
    [
        (
            "story.yaml",
            "title: Greet\nstart: hall\nvars: [{ id: late, type: bool, initial: false }]\n",
        ),
        (
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        (
            "things/keeper.yaml",
            "name: keeper\nlocation: { room: hall }\n",
        ),
        ("rules/rules.yaml", rules),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    })
    .into()
}

fn world(rules: &str) -> Arc<World> {
    let project = load(files(rules)).unwrap();
    assert!(check(&project).is_empty(), "{:#?}", check(&project));
    Arc::new(World::compile(project.story).unwrap())
}

fn say(game: &mut Session, text: &str) -> String {
    authoring::play(game, Input::Text(text.into())).plain_text()
}

#[test]
fn two_variants_of_one_event_fire_once_in_total() {
    let world = world(RULES);
    let mut game = Session::new(world.clone());
    authoring::start(&mut game);

    assert!(say(&mut game, "examine keeper").contains("Good day"));
    say(&mut game, "wait");
    let later = say(&mut game, "examine keeper");
    assert!(!later.contains("Late for a visit"), "{later}");
    assert!(!later.contains("Good day"), "{later}");

    // The save keeps the closed group; a load does not open it again.
    let save = serde_json::to_value(game.save()).unwrap();
    assert_eq!(save["state"]["once_groups"], serde_json::json!(["greet"]));
    let (mut loaded, _) = Session::load(world, save).unwrap();
    assert!(!say(&mut loaded, "examine keeper").contains("Late"));
}

#[test]
fn the_other_variant_can_close_the_group() {
    let mut game = Session::new(world(RULES));
    authoring::start(&mut game);
    say(&mut game, "wait");
    assert!(say(&mut game, "examine keeper").contains("Late for a visit"));
    assert!(!say(&mut game, "examine keeper").contains("stranger"));
}

#[test]
fn an_old_save_without_groups_loads() {
    let world = world(RULES);
    let mut game = Session::new(world.clone());
    authoring::start(&mut game);
    let mut save = serde_json::to_value(game.save()).unwrap();
    save["state"].as_object_mut().unwrap().remove("once_groups");
    let (mut loaded, _) = Session::load(world, save).unwrap();
    assert!(say(&mut loaded, "examine keeper").contains("Good day"));
}

#[test]
fn the_explorer_reaches_each_variant() {
    // Talking first or waiting first are different states: both variants fire.
    let found = explore(world(RULES), Budget::default());
    assert!(found.never_fired.is_empty(), "{found:#?}");
}

#[test]
fn a_group_of_one_rule_is_a_warning() {
    let rules = "- id: lonely\n  on: { verb: [examine, keeper] }\n  once: solo\n  then: []\n";
    let found = check(&load(files(rules)).unwrap());
    assert!(
        found
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("once-group `solo`")),
        "{found:#?}"
    );
}
