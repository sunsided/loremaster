//! The rules view shows behaviors in Rust with their manifest, and a `call`
//! effect in words.

use loremaster_model::Value;
use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Behavior, Cx, Event, EventKind, Flow, Manifest, World};
use loremaster_workbench::Workbench;
use std::sync::Arc;

/// Counts the events it gets in `seen`.
#[derive(Debug)]
struct Counter;

impl Behavior for Counter {
    fn id(&self) -> &str {
        "counter"
    }

    fn manifest(&self) -> Manifest {
        Manifest::new()
            .events([EventKind::Custom])
            .reads(["seen"])
            .writes(["seen"])
    }

    fn handle(&self, _: &Event, cx: &mut Cx<'_>) -> Flow {
        let seen = match cx.fact("seen") {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        cx.set("seen", Value::Int(seen + 1));
        Flow::Continue
    }
}

fn bench() -> Workbench {
    let files = [
        (
            "story.yaml",
            "title: Bell\nstart: hall\nbehaviors: [counter]\nvars:\n  - { id: seen, type: int, initial: 0 }\n",
        ),
        ("rooms/hall.yaml", "name: Hall\ndescription: [\"A hall.\"]\n"),
        ("verbs/ring.yaml", "label: Ring\nwords: [ring]\nslots: none\n"),
        (
            "rules/ring.yaml",
            "- id: ring\n  on: { verb: [ring] }\n  instead: true\n  then:\n    - emit: bell\n    - call: counter\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let mut world = World::compile(load(files.into()).unwrap().story).unwrap();
    world.register(Counter);
    let mut bench = Workbench::new(Arc::new(world));
    bench.start();
    bench
}

#[test]
fn the_rules_view_shows_the_manifest_of_a_behavior() {
    let rules = bench().rules();
    let view = rules.iter().find(|r| r.id == "behavior:counter").unwrap();
    assert!(
        view.when.contains("on custom; reads seen; writes seen"),
        "{}",
        view.when
    );
}

#[test]
fn the_rules_view_describes_a_call() {
    let rules = bench().rules();
    let ring = rules.iter().find(|r| r.id == "ring").unwrap();
    assert!(
        ring.then.iter().any(|t| t == "run the behavior counter"),
        "{:?}",
        ring.then
    );
}
