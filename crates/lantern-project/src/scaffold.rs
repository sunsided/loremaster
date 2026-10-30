//! The smallest valid story: what `lantern new` and the editor's "New story" write.

use crate::SourceFile;

/// The text of the start room until the author writes their own.
pub const START_TEXT: &str = "You stare at an empty screen, the cursor blinking in expectation.";

/// The files of a new story with this title: `story.yaml` and `rooms/start.yaml`.
pub fn scaffold(title: &str) -> Vec<SourceFile> {
    let title = title.trim();
    let title = if title.is_empty() { "Untitled" } else { title };
    vec![
        SourceFile {
            path: "story.yaml".into(),
            text: format!("title: {}\nstart: start\n\nintro: []\n", yaml_string(title)),
        },
        SourceFile {
            path: "rooms/start.yaml".into(),
            text: format!(
                "name: The Start\ndescription:\n  - {}\n",
                yaml_string(START_TEXT)
            ),
        },
    ]
}

/// A YAML scalar in double quotes, so that colons and other signs stay text.
fn yaml_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn the_scaffold_loads_and_checks_clean() {
        let project = crate::load(scaffold("A Tale: Part \"One\"")).unwrap();
        check!(project.story.title == "A Tale: Part \"One\"");
        check!(project.story.start.as_str() == "start");
        check!(crate::check(&project).is_empty());
    }

    #[test]
    fn an_empty_title_is_untitled() {
        let project = crate::load(scaffold("  ")).unwrap();
        check!(project.story.title == "Untitled");
    }
}
