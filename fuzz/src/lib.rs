//! Shared parts of the fuzz targets: the example worlds, compiled once.

use loremaster_lang::Scope;
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

/// The names of the Lighthouse, for conditions and markup.
pub fn lighthouse_scope() -> &'static Scope {
    static SCOPE: OnceLock<Scope> = OnceLock::new();
    SCOPE.get_or_init(|| Scope::from_story(&lighthouse().story))
}

/// The Salt Tower: rolls, awaits, a combat and stats with limits.
pub fn salt_tower() -> Arc<World> {
    static WORLD: OnceLock<Arc<World>> = OnceLock::new();
    WORLD.get_or_init(|| compile("salt-tower")).clone()
}

/// Play typed commands. An await gets its first answer, or the lowest total
/// of a roll. When the session refuses that answer (a save can hold an await
/// that no longer fits the story), the rest of the commands would only meet
/// the same await, so the play stops.
pub fn play<'a>(game: &mut Session, commands: impl IntoIterator<Item = &'a str>) {
    use loremaster_runtime::{AwaitRequest, Input, Status, Value};
    for c in commands {
        if let Status::Await { awaiting } = game.status() {
            let answer = match &awaiting.request {
                AwaitRequest::Roll { min, .. } => Value::Int(*min),
                request => match request.answers().into_iter().next() {
                    Some((answer, _)) => answer,
                    None => Value::Text("arrived".into()),
                },
            };
            if game.resume(&awaiting.id, answer).is_err() {
                return;
            }
        }
        game.submit(Input::Text(c.into()));
    }
}
