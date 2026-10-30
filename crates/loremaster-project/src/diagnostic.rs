use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// A problem in a project, pointing at a file and, where possible, a line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
