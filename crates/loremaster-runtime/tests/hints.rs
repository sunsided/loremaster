//! The hint ladder of the lighthouse (CK-6, UI-1): hints come from the steps
//! of `tests/win.yaml`, per room, one level at a time.

use loremaster_project::{load, read_dir};
use loremaster_runtime::{Effect, HintBook, Input, Session, Step, World};
use std::path::Path;
use std::sync::Arc;

fn lighthouse() -> Session {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&dir).unwrap()).unwrap();
    let world = Arc::new(World::compile(project.story).unwrap());
    let mut session = Session::new(world.clone());
    session.set_hints(HintBook::new(world, &project.walkthroughs));
    session.start();
    session
}

fn say(session: &mut Session, text: &str) -> String {
    session.submit(Input::Text(text.into())).plain_text()
}

fn note(step: &Step) -> String {
    step.effects
        .iter()
        .find_map(|e| match e {
            Effect::Text { block } => Some(block.plain_text()),
            _ => None,
        })
        .unwrap_or_default()
}

fn open_steps(session: &Session) -> Vec<String> {
    session.hints().into_iter().map(|h| h.step).collect()
}

#[test]
fn the_shore_has_no_hint_before_the_key() {
    let mut s = lighthouse();
    // The door needs the key: no hint for it yet.
    assert!(s.hints().is_empty());
    assert_eq!(say(&mut s, "hint"), "There is no hint for this place now.");
}

#[test]
fn hints_come_one_level_at_a_time_then_the_command() {
    let mut s = lighthouse();
    say(&mut s, "n");
    // Only the steps of the cottage whose needs are done.
    assert_eq!(open_steps(&s), ["drawer", "lamp", "matches"]);
    let first = s.hints().remove(0);
    assert_eq!(first.levels, 3);
    assert!(first.shown.is_empty());

    assert_eq!(
        note(&s.hint(None)),
        "Hint 1 of 3: Hal kept his few things in the cottage. Look at the furniture."
    );
    assert_eq!(
        say(&mut s, "hint"),
        "Hint 2 of 3: The dresser has a deep drawer."
    );
    assert_eq!(say(&mut s, "hint"), "Hint 3 of 3: type \"open drawer\".");
    let first = s.hints().remove(0);
    assert_eq!(first.shown.len(), 2);
    assert_eq!(first.command.as_deref(), Some("open drawer"));
    // The drawer is done: the next hint is of the next step.
    assert_eq!(
        say(&mut s, "hint"),
        "Hint 1 of 2: It will be dark where you are going. Something on the table could show you the way."
    );
    // A hint is no turn.
    assert_eq!(s.state().turns, 1);
}

#[test]
fn a_step_is_done_by_its_command_also_in_other_words() {
    let mut s = lighthouse();
    say(&mut s, "n");
    say(&mut s, "open the drawer");
    // The key needs the drawer: it is open now.
    assert_eq!(open_steps(&s), ["key", "lamp", "matches"]);
    say(&mut s, "get iron key");
    say(&mut s, "take lamp");
    say(&mut s, "take box");
    assert_eq!(open_steps(&s), ["lit"]);
    say(&mut s, "light lamp");
    assert!(s.hints().is_empty());
    // Undo puts the step back.
    say(&mut s, "undo");
    assert_eq!(open_steps(&s), ["lit"]);
    say(&mut s, "light lamp");
    say(&mut s, "s");
    assert_eq!(open_steps(&s), ["door"]);
}

#[test]
fn the_steps_done_survive_a_save() {
    let mut s = lighthouse();
    say(&mut s, "n");
    say(&mut s, "open drawer");
    let save = s.save();
    let world = Arc::new(s.world().clone());
    let (mut loaded, _) = Session::from_save(world.clone(), &save).unwrap();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&dir).unwrap()).unwrap();
    loaded.set_hints(HintBook::new(world, &project.walkthroughs));
    assert_eq!(open_steps(&loaded), ["key", "lamp", "matches"]);
}

#[test]
fn a_story_without_hints_says_so() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&dir).unwrap()).unwrap();
    let mut s = Session::new(Arc::new(World::compile(project.story).unwrap()));
    s.start();
    assert_eq!(say(&mut s, "hint"), "This story has no hints.");
}
