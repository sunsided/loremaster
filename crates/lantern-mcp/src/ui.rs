//! The web editor, embedded in the binary with the feature `embed-ui`.
//!
//! `web/build` must exist when the crate is built: run `task build:local`.
//! A path without a file gets `200.html`, the fallback page of the single-page app.

use include_dir::{Dir, include_dir};

static UI: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../web/build");

/// The file for a request path, with its content type. `None` when the build has no fallback page.
pub fn file(path: &str) -> Option<(&'static [u8], &'static str)> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let file = UI.get_file(path).or_else(|| UI.get_file("200.html"))?;
    let name = file.path().to_str().unwrap_or_default();
    Some((file.contents(), content_type(name)))
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
