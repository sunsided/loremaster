//! Play the imported Zork I with the thief, a behavior in Rust.
//!
//! `cargo run -p loremaster-runtime --example zork`, then type commands. An
//! empty line or the end of input stops the game.

#[path = "../../../examples/zork1/thief.rs"]
mod thief;

use loremaster_project::{load, read_dir};
use loremaster_runtime::{Input, Session, Status, World};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/zork1");
    let project = load(read_dir(&root).expect("read examples/zork1")).expect("load Zork I");
    let mut world = World::compile(project.story).expect("compile Zork I");
    world.register(thief::Thief::zork());
    let mut session = Session::new(Arc::new(world));
    println!("{}", session.start().plain_text());
    let stdin = std::io::stdin();
    loop {
        print!("\n> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
            break;
        }
        let step = session.submit(Input::Text(line.trim().into()));
        println!("{}", step.plain_text());
        // The death prompt waits as `Status::Await` after a death ending:
        // the player answers it with `undo` or `restart` on the next line.
        if matches!(step.status, Status::Ended { .. }) {
            break;
        }
    }
}
