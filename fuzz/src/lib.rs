//! Shared parts of the fuzz targets: the example worlds, compiled once.

use loremaster_project::{load, read_dir};
use loremaster_runtime::{Session, World};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// The folder of an example.
pub fn example_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples")
        .join(name)
}

fn compile(name: &str) -> Arc<World> {
    let project = load(read_dir(&example_dir(name)).unwrap()).unwrap();
    Arc::new(World::compile(project.story).unwrap())
}

/// The Lighthouse: rooms, things, talks and timers.
pub fn lighthouse() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD.get_or_init(|| compile("lighthouse")).clone()
}

/// The Salt Tower: rolls, awaits, a combat and stats with limits.
pub fn salt_tower() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD.get_or_init(|| compile("salt-tower")).clone()
}

/// Play typed commands; an await gets its first option, or the lowest total.
pub fn play<'a>(game: &mut Session, commands: impl IntoIterator<Item = &'a str>) {
    use loremaster_runtime::{AwaitRequest, Input, Status, Value};
    for c in commands {
        if let Status::Await { awaiting } = game.status() {
            let answer = match &awaiting.request {
                AwaitRequest::Choice { options } if !options.is_empty() => {
                    Value::Text(options[0].id.to_string())
                }
                AwaitRequest::Roll { min, .. } => Value::Int(*min),
                _ => Value::Text("arrived".into()),
            };
            let _ = game.resume(&awaiting.id, answer);
        }
        game.submit(Input::Text(c.into()));
    }
}
