use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// A problem in a project, pointing at a file and, where possible, a line.
///
/// The one form of every diagnostic that the tools show (XP-6): from loading,
/// from the checks and from compiling a story. An editor needs only the file,
/// the position, the severity and the message, whatever the source is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: String,
    /// Where in the entity, e.g. `description[2].when`.
    pub at: String,
    pub message: String,
    /// 1-based line and column in the file, when it could be located.
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl Diagnostic {
    /// An error in `file`, at a place inside its entity (for example
    /// `exits[0]`), without a line yet.
    pub fn error(
        file: impl Into<String>,
        at: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, file, at, message)
    }

    /// A warning in `file`, without a line yet.
    pub fn warning(
        file: impl Into<String>,
        at: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, file, at, message)
    }

    fn new(
        severity: Severity,
        file: impl Into<String>,
        at: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            file: file.into(),
            at: at.into(),
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{level}: {}", self.file)?;
        if let (Some(l), Some(c)) = (self.line, self.column) {
            write!(f, ":{l}:{c}")?;
        }
        if !self.at.is_empty() {
            write!(f, " ({})", self.at)?;
        }
        write!(f, ": {}", self.message)
    }
}
