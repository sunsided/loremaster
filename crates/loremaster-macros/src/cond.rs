//! Conditions as tokens: `when not windlass.carried` is the condition
//! `not windlass.carried`. The tokens print in the form that YAML authors
//! write, so that both forms give the same model: one space around operators
//! and words, none around `.` and before `(` of a call.

use crate::ast::Cond;
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use syn::parse::ParseStream;
use syn::{LitStr, Result};

/// Read a condition up to the first token that `stop` accepts (it is not
/// read). A single string literal is the condition as written.
pub fn parse_until(input: ParseStream, stop: impl Fn(ParseStream) -> bool) -> Result<Cond> {
    let mut tokens = Vec::new();
    while !input.is_empty() && !stop(input) {
        tokens.push(input.parse::<TokenTree>()?);
    }
    let stream: TokenStream = tokens.iter().cloned().collect();
    if tokens.is_empty() {
        return Err(input.error("expected a condition"));
    }
    if let [TokenTree::Literal(lit)] = tokens.as_slice()
        && let Ok(text) = syn::parse2::<LitStr>(TokenStream::from(TokenTree::Literal(lit.clone())))
    {
        return Ok(Cond {
            source: text.value(),
            tokens: stream,
        });
    }
    let source = print(&tokens)?;
    Ok(Cond {
        source,
        tokens: stream,
    })
}

#[derive(PartialEq)]
enum Piece {
    Word,
    Dot,
    Comma,
    Op,
    Unary,
    Call,
    Group,
}

fn print(tokens: &[TokenTree]) -> Result<String> {
    let mut out = String::new();
    let mut last: Option<Piece> = None;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Ident(ident) => {
                let word = ident.to_string();
                let word = word.strip_prefix("r#").unwrap_or(&word).to_string();
                space(&mut out, &last);
                out.push_str(&word);
                last = Some(Piece::Word);
                // `in kitchen` is `in(kitchen)`: the location sugar.
                if word == "in"
                    && let Some(TokenTree::Ident(room)) = tokens.get(i + 1)
                {
                    out.push('(');
                    out.push_str(&room.to_string());
                    out.push(')');
                    last = Some(Piece::Call);
                    i += 1;
                }
            }
            TokenTree::Literal(lit) => {
                space(&mut out, &last);
                out.push_str(&lit.to_string());
                last = Some(Piece::Word);
            }
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ('(', ')'),
                    Delimiter::Bracket => ('[', ']'),
                    Delimiter::None => (' ', ' '),
                    Delimiter::Brace => {
                        return Err(syn::Error::new(
                            group.span(),
                            "a condition has no `{`; did the rule body start too early?",
                        ));
                    }
                };
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                let call = last == Some(Piece::Word) && open == '(';
                if !call {
                    space(&mut out, &last);
                }
                out.push(open);
                out.push_str(&print(&inner)?);
                out.push(close);
                last = Some(if call { Piece::Call } else { Piece::Group });
            }
            TokenTree::Punct(p) => {
                let mut op = p.as_char().to_string();
                let mut spacing = p.spacing();
                while spacing == Spacing::Joint
                    && let Some(TokenTree::Punct(next)) = tokens.get(i + 1)
                {
                    op.push(next.as_char());
                    spacing = next.spacing();
                    i += 1;
                }
                match op.as_str() {
                    "." => {
                        out.push('.');
                        last = Some(Piece::Dot);
                    }
                    "," => {
                        out.push(',');
                        last = Some(Piece::Comma);
                    }
                    "!" | "-"
                        if !matches!(last, Some(Piece::Word | Piece::Call | Piece::Group)) =>
                    {
                        space(&mut out, &last);
                        out.push_str(&op);
                        last = Some(Piece::Unary);
                    }
                    _ => {
                        space(&mut out, &last);
                        out.push_str(&op);
                        last = Some(Piece::Op);
                    }
                }
            }
        }
        i += 1;
    }
    Ok(out)
}

/// One space before the next piece, unless it follows a `.`, a unary
/// operator, or starts the text.
fn space(out: &mut String, last: &Option<Piece>) {
    if !matches!(last, None | Some(Piece::Dot | Piece::Unary)) {
        out.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::print;
    use proc_macro2::{TokenStream, TokenTree};

    fn cond(text: &str) -> String {
        let tokens: TokenStream = text.parse().unwrap();
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        print(&tokens).unwrap()
    }

    #[test]
    fn prints_as_yaml_authors_write() {
        for (tokens, want) in [
            ("not windlass.carried", "not windlass.carried"),
            ("lock != empty", "lock != empty"),
            (
                "in(lock_chamber) and lock == full",
                "in(lock_chamber) and lock == full",
            ),
            ("in kitchen and night", "in(kitchen) and night"),
            ("in(pages, gear_housing)", "in(pages, gear_housing)"),
            ("timer.lock_drain >= 2", "timer.lock_drain >= 2"),
            ("!beacon_lit || x < -1", "!beacon_lit || x < -1"),
            ("(a or b) and c", "(a or b) and c"),
            ("mood == \"calm\"", "mood == \"calm\""),
        ] {
            assert_eq!(cond(tokens), want, "{tokens}");
        }
    }
}
