//! With the feature `embed-ui`, `src/ui.rs` embeds the web editor from `ui/`.
//! `ui/` is a copy of `web/build`. `task release:ui` makes it.

use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=ui");
    if std::env::var_os("CARGO_FEATURE_EMBED_UI").is_none() {
        return;
    }
    let index = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("ui/index.html");
    if !index.is_file() {
        panic!(
            "\n\nThe feature `embed-ui` needs the built web editor in crates/loremaster-mcp/ui/.\n\
             Run `task release:ui` to build it and copy it there.\n\
             Or build without the editor: `cargo build --no-default-features`.\n\n"
        );
    }
}
