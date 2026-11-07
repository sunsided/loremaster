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
    // A predicate can start here: at the start, or after `and`, `or`,
    // `not`, `&&`, `||` or `!`. Only there is `in <room>` the sugar; after
    // `==` a word `in` is a value, such as a machine state.
    let mut predicate = true;
    let mut last_word = String::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Ident(ident) => {
                let word = unraw(ident);
                space(&mut out, &last);
                out.push_str(&word);
                last = Some(Piece::Word);
                // `in kitchen` is `in(kitchen)`: the location sugar.
                if word == "in"
                    && predicate
                    && let Some(TokenTree::Ident(room)) = tokens.get(i + 1)
                    && !KEYWORDS.contains(&unraw(room).as_str())
                {
                    out.push('(');
                    out.push_str(&unraw(room));
                    out.push(')');
                    last = Some(Piece::Call);
                    i += 1;
                }
                predicate = KEYWORDS[..3].contains(&word.as_str());
                last_word = word;
            }
            TokenTree::Literal(lit) => {
                space(&mut out, &last);
                out.push_str(&literal(lit)?);
                last = Some(Piece::Word);
                predicate = false;
            }
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ('(', ')'),
                    Delimiter::Bracket => ('[', ']'),
                    // An invisible group, such as a `$c:expr` of a
                    // macro_rules! wrapper: keep its grouping.
                    Delimiter::None => ('(', ')'),
                    Delimiter::Brace => {
                        return Err(syn::Error::new(
                            group.span(),
                            "a condition has no `{`; did the rule body start too early?",
                        ));
                    }
                };
                let inner: Vec<TokenTree> = group.stream().into_iter().collect();
                let call = last == Some(Piece::Word)
                    && open == '('
                    && (group.delimiter() == Delimiter::Parenthesis || last_word == "in");
                if !call {
                    space(&mut out, &last);
                }
                out.push(open);
                out.push_str(&print(&inner)?);
                out.push(close);
                last = Some(if call { Piece::Call } else { Piece::Group });
                predicate = false;
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
                        predicate = false;
                    }
                    "," => {
                        out.push(',');
                        last = Some(Piece::Comma);
                        predicate = false;
                    }
                    "!" | "-"
                        if !matches!(last, Some(Piece::Word | Piece::Call | Piece::Group)) =>
                    {
                        space(&mut out, &last);
                        out.push_str(&op);
                        last = Some(Piece::Unary);
                        predicate = op == "!";
                    }
                    _ => {
                        space(&mut out, &last);
                        out.push_str(&op);
                        last = Some(Piece::Op);
                        predicate = op == "&&" || op == "||";
                    }
                }
            }
        }
        i += 1;
    }
    Ok(out)
}

/// `and`, `or` and `not` start a predicate; none of these is a room.
const KEYWORDS: &[&str] = &["and", "or", "not", "true", "false"];

/// A literal as the condition language reads it. A string is its value,
/// quoted again: the language has no escapes, so `"a\tb"` must reach it as
/// the decoded text, and a raw string as a plain one. A text with both
/// quote characters cannot be written.
fn literal(lit: &proc_macro2::Literal) -> Result<String> {
    let token = TokenStream::from(TokenTree::Literal(lit.clone()));
    let Ok(text) = syn::parse2::<LitStr>(token) else {
        return Ok(lit.to_string());
    };
    let value = text.value();
    if !value.contains('"') {
        Ok(format!("\"{value}\""))
    } else if !value.contains('\'') {
        Ok(format!("'{value}'"))
    } else {
        Err(syn::Error::new(
            lit.span(),
            "a string in a condition cannot hold both `\"` and `'`",
        ))
    }
}

/// An identifier without the raw prefix: `r#type` is the id `type`.
fn unraw(ident: &proc_macro2::Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").map(str::to_string).unwrap_or(s)
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
            ("in r#type and r#loop.lit", "in(type) and loop.lit"),
            ("lock == r#in and night", "lock == in and night"),
            ("not in kitchen || in hall", "not in(kitchen) || in(hall)"),
            ("name == \"a\\tb\"", "name == \"a\tb\""),
            ("name == r#\"say \"hi\"\"#", "name == 'say \"hi\"'"),
        ] {
            assert_eq!(cond(tokens), want, "{tokens}");
        }
    }

    #[test]
    fn an_invisible_group_keeps_its_grouping() {
        use proc_macro2::{Delimiter, Group};
        // `$c and night` with `$c = lock_open or windlass.carried`.
        let c: TokenStream = "lock_open or windlass.carried".parse().unwrap();
        let rest: TokenStream = "and night".parse().unwrap();
        let mut tokens = vec![TokenTree::Group(Group::new(Delimiter::None, c))];
        tokens.extend(rest);
        assert_eq!(
            print(&tokens).unwrap(),
            "(lock_open or windlass.carried) and night"
        );
        // `in $room`: the sugar still applies.
        let room: TokenStream = "kitchen".parse().unwrap();
        let tokens = vec![
            TokenTree::Ident(proc_macro2::Ident::new(
                "in",
                proc_macro2::Span::call_site(),
            )),
            TokenTree::Group(Group::new(Delimiter::None, room)),
        ];
        assert_eq!(print(&tokens).unwrap(), "in(kitchen)");
    }
}
