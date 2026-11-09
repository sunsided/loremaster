//! The MCP server (`loremaster mcp`, and `/mcp` of `loremaster serve`): each
//! line of the input is one JSON-RPC message to a new server of a copy of
//! the Lighthouse. Over MCP the server writes only its proposals, in
//! `.loremaster/`; each input starts without them.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_mcp::Server;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The copy of the Lighthouse, made once per process.
fn folder() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("loremaster-fuzz-mcp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy(&loremaster_fuzz::example_dir("lighthouse"), &dir);
        dir
    })
}

fuzz_target!(|text: &str| {
    let dir = folder();
    let _ = std::fs::remove_dir_all(dir.join(".loremaster"));
    let mut server = Server::new(dir.clone()).unwrap();
    for line in text.lines().take(16) {
        server.handle_line(line);
    }
});
