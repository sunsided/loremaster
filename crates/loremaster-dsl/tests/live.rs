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
    assert!(!live.poll().expect("a change").unwrap_err().is_empty());
    assert!(live.poll().is_none(), "the same broken files report once");

    room(&dir, "A quiet hall.");
    let rebuilt = live.poll().expect("a change").expect("the fix builds");
    assert!(text(&rebuilt.world).contains("A quiet hall."));
    fs::remove_dir_all(&dir).unwrap();
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
