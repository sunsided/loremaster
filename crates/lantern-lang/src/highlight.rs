//! Token spans for an editor. The tokenizer never fails: it also works on text
//! that the author is typing, with braces that are not closed yet.

use crate::Span;
use crate::lexer::{Tok, lex};
use serde::Serialize;

/// The kind of a highlighted span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// `{` and `}` of markup.
    Brace,
    /// `:` or `?` after a condition, `|` or `:` between branches.
    Separator,
    /// A name in a condition or an insert: `lamp.lit`, `night`, `first`.
    Name,
    /// `and`, `or`, `not`, `==` and the other operators.
    Operator,
    /// A number or a quoted string in a condition.
    Literal,
    /// `{{` or `}}`: a literal brace.
    Escape,
    /// `*emphasis*`, with its stars.
    Emphasis,
    /// Text that does not lex, for example a stray character in a condition.
    Invalid,
}

/// One highlighted span. Text outside all spans is prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

/// The highlighted spans of fragment text with inline markup, in order.
pub fn markup_tokens(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    text(src, 0, &mut out);
    out
}

/// The highlighted spans of a condition, for example the `when` of a fragment.
pub fn condition_tokens(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    expression(src, 0, &mut out);
    out
}

fn push(out: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end > start {
        out.push(Token {
            span: start..end,
            kind,
        });
    }
}

fn text(src: &str, base: usize, out: &mut Vec<Token>) {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut emphasis: Option<usize> = None;
    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => {
                push(out, base + i, base + i + 2, TokenKind::Escape);
                i += 2;
            }
            b'}' if bytes.get(i + 1) == Some(&b'}') => {
                push(out, base + i, base + i + 2, TokenKind::Escape);
                i += 2;
            }
            b'{' => {
                let end = matching(src, i);
                push(out, base + i, base + i + 1, TokenKind::Brace);
                let inner_end = end.unwrap_or(src.len());
                inner(&src[i + 1..inner_end], base + i + 1, out);
                match end {
                    Some(e) => {
                        push(out, base + e, base + e + 1, TokenKind::Brace);
                        i = e + 1;
                    }
                    None => i = src.len(),
                }
            }
            b'}' => {
                push(out, base + i, base + i + 1, TokenKind::Invalid);
                i += 1;
            }
            b'*' => {
                match emphasis.take() {
                    Some(start) => push(out, base + start, base + i + 1, TokenKind::Emphasis),
                    None => emphasis = Some(i),
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    out.sort_by_key(|t| t.span.start);
}

fn matching(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn top(src: &str, chars: &[char]) -> Option<(usize, char)> {
    let mut depth = 0;
    for (i, c) in src.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if depth == 0 && chars.contains(&c) => return Some((i, c)),
            _ => {}
        }
    }
    None
}

fn inner(src: &str, base: usize, out: &mut Vec<Token>) {
    let Some((sep, kind)) = top(src, &['?', ':']) else {
        expression(src, base, out);
        return;
    };
    expression(&src[..sep], base, out);
    push(out, base + sep, base + sep + 1, TokenKind::Separator);
    let rest = &src[sep + 1..];
    let rest_base = base + sep + 1;
    let seps: &[char] = if kind == '?' { &['|', ':'] } else { &['|'] };
    match top(rest, seps) {
        Some((j, _)) => {
            text(&rest[..j], rest_base, out);
            push(out, rest_base + j, rest_base + j + 1, TokenKind::Separator);
            text(&rest[j + 1..], rest_base + j + 1, out);
        }
        None => text(rest, rest_base, out),
    }
}

fn expression(src: &str, base: usize, out: &mut Vec<Token>) {
    match lex(src) {
        Ok(tokens) => {
            for (tok, span) in tokens {
                let kind = match tok {
                    Tok::Ident(_) | Tok::Dot => TokenKind::Name,
                    Tok::Int(_) | Tok::Str(_) => TokenKind::Literal,
                    Tok::Comma | Tok::LParen | Tok::RParen => TokenKind::Operator,
                    _ => TokenKind::Operator,
                };
                push(out, base + span.start, base + span.end, kind);
            }
        }
        Err(e) => {
            // Highlight the part before the error, and mark the rest.
            if e.span.start > 0 {
                expression(&src[..e.span.start], base, out);
            }
            let trimmed = src.trim_end().len().max(e.span.start);
            push(out, base + e.span.start, base + trimmed, TokenKind::Invalid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn kinds(src: &str) -> Vec<(&str, TokenKind)> {
        markup_tokens(src)
            .into_iter()
            .map(|t| (&src[t.span.clone()], t.kind))
            .collect()
    }

    #[test]
    fn choices_inserts_and_emphasis() {
        use TokenKind::*;
        check!(
            kinds("The {night: dark | *bright*} {lamp.name}.")
                == vec![
                    ("{", Brace),
                    ("night", Name),
                    (":", Separator),
                    ("|", Separator),
                    ("*bright*", Emphasis),
                    ("}", Brace),
                    ("{", Brace),
                    ("lamp", Name),
                    (".", Name),
                    ("name", Name),
                    ("}", Brace),
                ]
        );
    }

    #[test]
    fn unfinished_text_still_highlights() {
        use TokenKind::*;
        check!(
            kinds("A {lamp.lit ? glow")
                == vec![
                    ("{", Brace),
                    ("lamp", Name),
                    (".", Name),
                    ("lit", Name),
                    ("?", Separator)
                ]
        );
        check!(kinds("x }") == vec![("}", Invalid)]);
        check!(kinds("{a == 2 and not b}").len() == 8);
    }
}
