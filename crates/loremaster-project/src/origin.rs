use serde::{Deserialize, Serialize};
use std::fmt;

/// Where an entity of a project comes from (DS-2). The editor uses it for the
/// source badge, and shows parts from Rust as read-only.
///
/// `#[non_exhaustive]`, so that a script source can be added later (XP-6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Origin {
    /// A file of a YAML folder, relative to the folder.
    Yaml { path: String },
    /// A `world!` block or a builder function, in a Rust file.
    Rust {
        file: String,
        line: u32,
        column: u32,
        /// The name of the pack.
        pack: String,
        /// The content hash of the entity (DS-11): SHA-256 of its model
        /// value as JSON, not of the source text. So `rustfmt` and comments
        /// do not change it. `assemble` fills it in, when the entity is
        /// complete; a builder cannot, because it still changes the entity.
        #[serde(default)]
        hash: String,
    },
}

impl Origin {
    pub fn yaml(path: impl Into<String>) -> Self {
        Origin::Yaml { path: path.into() }
    }

    /// The file: the YAML path, or the Rust file.
    pub fn file(&self) -> &str {
        match self {
            Origin::Yaml { path } => path,
            Origin::Rust { file, .. } => file,
        }
    }

    /// The YAML path, when the entity comes from a YAML file. The editor can
    /// change only these.
    pub fn yaml_path(&self) -> Option<&str> {
        match self {
            Origin::Yaml { path } => Some(path),
            _ => None,
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Yaml { path } => write!(f, "{path}"),
            Origin::Rust {
                file,
                line,
                column,
                pack,
                ..
            } => write!(f, "{file}:{line}:{column} (pack {pack})"),
        }
    }
}
