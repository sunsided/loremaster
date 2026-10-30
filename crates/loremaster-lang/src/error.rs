use std::fmt;
use std::ops::Range;

/// Byte range into the source text.
pub type Span = Range<usize>;

/// A parse or type error, with the byte range it refers to.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct LangError {
    pub message: String,
    pub span: Span,
}

impl LangError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Shift the span, for errors inside a larger text (markup).
    pub(crate) fn offset(mut self, by: usize) -> Self {
        self.span = self.span.start + by..self.span.end + by;
        self
    }
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at {}..{})",
            self.message, self.span.start, self.span.end
        )
    }
}
