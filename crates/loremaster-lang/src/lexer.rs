use crate::{LangError, Span};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    Ident(String),
    Int(i64),
    Str(String),
    Dot,
    Comma,
    LParen,
    RParen,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
}

pub(crate) fn lex(src: &str) -> Result<Vec<(Tok, Span)>, LangError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let start = i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let two = src.get(i..i + 2).unwrap_or("");
        let tok = match two {
            "==" => Some(Tok::Eq),
            "!=" => Some(Tok::Ne),
            "<=" => Some(Tok::Le),
            ">=" => Some(Tok::Ge),
            "&&" => Some(Tok::And),
            "||" => Some(Tok::Or),
            _ => None,
        };
        if let Some(tok) = tok {
            i += 2;
            out.push((tok, start..i));
            continue;
        }
        let tok = match c {
            '.' => Some(Tok::Dot),
            ',' => Some(Tok::Comma),
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '<' => Some(Tok::Lt),
            '>' => Some(Tok::Gt),
            '!' => Some(Tok::Not),
            '=' => Some(Tok::Eq),
            _ => None,
        };
        if let Some(tok) = tok {
            i += 1;
            out.push((tok, start..i));
            continue;
        }
        if c.is_ascii_digit() || (c == '-' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let n = src[start..i]
                .parse()
                .map_err(|_| LangError::new("number out of range", start..i))?;
            out.push((Tok::Int(n), start..i));
            continue;
        }
        if c == '"' || c == '\'' {
            i += 1;
            while i < bytes.len() && bytes[i] as char != c {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(LangError::new("unterminated string", start..i));
            }
            i += 1;
            out.push((Tok::Str(src[start + 1..i - 1].to_string()), start..i));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            while i < bytes.len() {
                let ch = src[i..].chars().next().unwrap_or(' ');
                if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            let word = &src[start..i];
            let tok = match word {
                "and" => Tok::And,
                "or" => Tok::Or,
                "not" => Tok::Not,
                "is" => Tok::Eq,
                _ => Tok::Ident(word.to_string()),
            };
            out.push((tok, start..i));
            continue;
        }
        let len = c.len_utf8();
        return Err(LangError::new(
            format!("unexpected character `{c}`"),
            start..start + len,
        ));
    }
    Ok(out)
}
