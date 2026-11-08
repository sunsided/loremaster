//! A `build.rs` helper: generate the host types of a YAML project (IF-3).
//!
//! ```no_run
//! // build.rs
//! loremaster_build::generate("story").unwrap();
//! ```
//!
//! ```ignore
//! // src/main.rs
//! include!(concat!(env!("OUT_DIR"), "/loremaster_interface.rs"));
//! ```
//!
//! The generated file has `INTERFACE_HASH`, `interface()`, `Request`, one
//! `<Action>Result` enum per action and `HostEvent`. A `match` over `Request`
//! fails to compile when the story adds an action.

use loremaster_project::{check_files, read_dir, rust_bindings};
use std::path::Path;

/// Check the project in `dir` and write `loremaster_interface.rs` to
/// `OUT_DIR`. An error of the project fails the build.
pub fn generate(dir: impl AsRef<Path>) -> Result<(), String> {
    let dir = dir.as_ref();
    println!("cargo:rerun-if-changed={}", dir.display());
    let out = std::env::var("OUT_DIR").map_err(|_| "OUT_DIR is not set; run from build.rs")?;
    let source = source_of(dir)?;
    std::fs::write(Path::new(&out).join("loremaster_interface.rs"), source)
        .map_err(|e| e.to_string())
}

fn source_of(dir: &Path) -> Result<String, String> {
    let files = read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let checked = check_files(files);
    let errors: Vec<String> = checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == loremaster_project::Severity::Error)
        .map(ToString::to_string)
        .collect();
    let Some(project) = checked.project.filter(|_| errors.is_empty()) else {
        return Err(format!("the story has errors:\n{}", errors.join("\n")));
    };
    let interface = project.story.interface.unwrap_or_default();
    Ok(rust_bindings(&interface, "::loremaster_runtime"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_gives_the_generated_source() {
        let dir = std::env::temp_dir().join(format!("loremaster-build-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("rooms")).unwrap();
        std::fs::write(
            dir.join("story.yaml"),
            "title: T\nstart: r\ninterface:\n  actions:\n    - { id: pick, results: [yes_it, no_it] }\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("rooms/r.yaml"),
            "name: R\ndescription: [\"Here.\"]\n",
        )
        .unwrap();
        let source = source_of(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(source.contains("pub enum PickResult"), "{source}");
        assert!(source.contains("pub enum Request"), "{source}");
    }
}
