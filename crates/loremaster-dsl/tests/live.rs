//! `LiveStory`: a changed YAML file gives a new world, a broken one its
//! diagnostics, once.

use loremaster_dsl::prelude::*;
use std::fs;
use std::path::PathBuf;

/// A fresh story folder: story.yaml and one room.
fn story_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("loremaster-live-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("rooms")).unwrap();
    fs::write(dir.join("story.yaml"), "title: Live\nstart: hall\n").unwrap();
    room(&dir, "An empty hall.");
    dir
}

fn room(dir: &std::path::Path, text: &str) {
    fs::write(
        dir.join("rooms/hall.yaml"),
        format!("name: Hall\ndescription:\n  - \"{text}\"\n"),
    )
    .unwrap();
}

fn text(world: &loremaster_runtime::World) -> String {
    format!("{:?}", world.story.rooms)
}

#[test]
fn reports_each_change_once() {
    let dir = story_dir("change");
    let mut live = LiveStory::new(Loremaster::new().yaml(&dir));
    assert!(live.poll().is_none());

    room(&dir, "A hall full of echoes.");
    let rebuilt = live.poll().expect("a change").expect("it builds");
    assert!(text(&rebuilt.world).contains("A hall full of echoes."));

    fs::write(dir.join("rooms/hall.yaml"), "name: [unclosed\n").unwrap();
    let diags = live.poll().expect("a change").unwrap_err();
    assert_eq!(diags[0].file, "rooms/hall.yaml", "{diags:?}");
    assert!(
        diags[0].line.is_some() && !diags[0].message.contains("cannot read"),
        "the parse error: {diags:?}"
    );
    assert!(live.poll().is_none(), "the same broken files report once");

    room(&dir, "A quiet hall.");
    let rebuilt = live.poll().expect("a change").expect("the fix builds");
    assert!(text(&rebuilt.world).contains("A quiet hall."));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn carries_walkthroughs_and_warnings() {
    let dir = story_dir("carried");
    let mut live = LiveStory::new(Loremaster::new().yaml(&dir));
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join("rules")).unwrap();
    fs::write(
        dir.join("tests/smoke.yaml"),
        "name: Smoke\nsteps:\n  - look\n",
    )
    .unwrap();
    fs::write(
        dir.join("rules/ticks.yaml"),
        "on: { event: turn }\nfree: true\nthen: []\n",
    )
    .unwrap();

    let rebuilt = live.poll().expect("a change").expect("it builds");
    assert_eq!(rebuilt.walkthroughs.len(), 1);
    assert_eq!(rebuilt.walkthroughs[0].0, "tests/smoke.yaml");
    assert!(
        rebuilt.warnings.iter().any(|d| d.at.contains("free")),
        "{:?}",
        rebuilt.warnings
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reports_an_unreadable_folder_once() {
    let dir = story_dir("unreadable").join("no-such-folder");
    let mut live = LiveStory::new(Loremaster::new().yaml(&dir));
    let diags = live.poll().expect("reported").unwrap_err();
    assert!(
        diags[0].message.contains("cannot read the YAML folder"),
        "{diags:?}"
    );
    assert!(live.poll().is_none());
}

#[test]
fn keeps_the_folder_in_paths_with_two_sources() {
    let first = story_dir("two-a");
    let second = story_dir("two-b");
    let mut live = LiveStory::new(Loremaster::new().yaml(&first).yaml(&second));
    fs::write(second.join("rooms/hall.yaml"), "name: [unclosed\n").unwrap();
    let diags = live.poll().expect("a change").unwrap_err();
    assert_eq!(
        diags[0].file,
        format!("{}/rooms/hall.yaml", second.display())
    );
    fs::remove_dir_all(&first).unwrap();
    fs::remove_dir_all(&second).unwrap();
}

#[derive(Debug)]
struct Noop(&'static str);

impl Behavior for Noop {
    fn id(&self) -> &str {
        self.0
    }
    fn handle(&self, _: &Event, _: &mut Cx<'_>) -> Flow {
        Flow::Continue
    }
}

#[test]
fn registers_pack_and_host_behaviors() {
    let dir = story_dir("behaviors");
    // A behavior the YAML names and the host provides.
    fs::create_dir_all(dir.join("behaviors")).unwrap();
    fs::write(dir.join("behaviors/host.yaml"), "").unwrap();
    let mut pack = Pack::new("rust");
    pack.register(Noop("from_pack"));
    let builder = Loremaster::new().pack(pack).yaml(&dir);

    let mut bare = LiveStory::new(builder.clone());
    let mut live = LiveStory::new(builder).behaviors(|w| {
        w.register(Noop("host"));
    });
    room(&dir, "A hall with gears.");

    let diags = bare.poll().expect("a change").unwrap_err();
    assert!(
        diags.iter().any(|d| d.message.contains("`host`")),
        "{diags:?}"
    );

    let rebuilt = live.poll().expect("a change").expect("it builds");
    let ids = &rebuilt.world.story.behaviors;
    assert!(ids.iter().any(|id| id.as_str() == "from_pack"), "{ids:?}");
    assert!(rebuilt.world.missing_behaviors().is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn can_move_to_a_watcher_thread() {
    fn send<T: Send>() {}
    send::<LiveStory>();
}
