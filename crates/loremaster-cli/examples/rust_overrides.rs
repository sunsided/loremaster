//! The reference example of editor overrides: `examples/rust-overrides/`.
//! `cargo run -p loremaster --example rust_overrides --features dsl --
//! export-pack <dir>` writes its packs, as `loremaster export-pack` does.

#[path = "../../../examples/rust-overrides/world.rs"]
mod world;

use loremaster::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, dir] = args.as_slice()
        && cmd == "export-pack"
    {
        let paths = Loremaster::new()
            .pack(world::pack())
            .export_packs(dir)
            .expect("the packs can be written");
        println!("wrote {paths:?}");
    }
}
