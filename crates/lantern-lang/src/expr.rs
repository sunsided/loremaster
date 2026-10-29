use crate::lexer::{Tok, lex};
use crate::{CmpOp, LangError, Span};

/// Parsed, untyped condition expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Bool(bool, Span),
    Int(i64, Span),
    Str(String, Span),
    /// `a.b.c`
    Path(Vec<String>, Span),
    /// `visited(cellar)`
    Call(String, Vec<Expr>, Span),
    Not(Box<Expr>, Span),
    And(Vec<Expr>, Span),
    Or(Vec<Expr>, Span),
    Cmp(Box<Expr>, CmpOp, Box<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Bool(_, s)
            | Expr::Int(_, s)
            | Expr::Str(_, s)
            | Expr::Path(_, s)
            | Expr::Call(_, _, s)
            | Expr::Not(_, s)
            | Expr::And(_, s)
            | Expr::Or(_, s)
            | Expr::Cmp(_, _, _, s) => s.clone(),
        }
    }
}

/// Parse a condition expression.
pub fn parse_expr(src: &str) -> Result<Expr, LangError> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err(LangError::new("empty condition", 0..src.len()));
    }
    let mut p = Parser {
        toks,
        pos: 0,
        len: src.len(),
    };
    let e = p.or()?;
    if let Some((_, span)) = p.toks.get(p.pos) {
        return Err(LangError::new(
            "unexpected text after condition",
            span.clone(),
        ));
    }
    Ok(e)
}

struct Parser {
    toks: Vec<(Tok, Span)>,
    pos: usize,
    len: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn span_here(&self) -> Span {
        self.toks
            .get(self.pos)
            .map(|(_, s)| s.clone())
            .unwrap_or(self.len..self.len)
    }

    fn bump(&mut self) -> Option<(Tok, Span)> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == Some(tok) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn or(&mut self) -> Result<Expr, LangError> {
        let first = self.and()?;
        let mut items = vec![first];
        while self.eat(&Tok::Or) {
            items.push(self.and()?);
        }
        Ok(join(items, Expr::Or))
    }

    fn and(&mut self) -> Result<Expr, LangError> {
        let first = self.not()?;
        let mut items = vec![first];
        while self.eat(&Tok::And) {
            items.push(self.not()?);
        }
        Ok(join(items, Expr::And))
    }

    fn not(&mut self) -> Result<Expr, LangError> {
        let start = self.span_here();
        if self.eat(&Tok::Not) {
            let inner = self.not()?;
            let span = start.start..inner.span().end;
            return Ok(Expr::Not(Box::new(inner), span));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, LangError> {
        let lhs = self.atom()?;
        let op = match self.peek() {
            Some(Tok::Eq) => CmpOp::Eq,
            Some(Tok::Ne) => CmpOp::Ne,
            Some(Tok::Lt) => CmpOp::Lt,
            Some(Tok::Le) => CmpOp::Le,
            Some(Tok::Gt) => CmpOp::Gt,
            Some(Tok::Ge) => CmpOp::Ge,
            _ => return Ok(lhs),
        };
        self.pos += 1;
        // `x is not y`
        let op = if op == CmpOp::Eq && self.eat(&Tok::Not) {
            CmpOp::Ne
        } else {
            op
        };
        let rhs = self.atom()?;
        let span = lhs.span().start..rhs.span().end;
        Ok(Expr::Cmp(Box::new(lhs), op, Box::new(rhs), span))
    }

    fn atom(&mut self) -> Result<Expr, LangError> {
        let here = self.span_here();
        match self.bump() {
            Some((Tok::LParen, _)) => {
                let e = self.or()?;
                if !self.eat(&Tok::RParen) {
                    return Err(LangError::new("expected `)`", self.span_here()));
                }
                Ok(e)
            }
            Some((Tok::Int(n), s)) => Ok(Expr::Int(n, s)),
            Some((Tok::Str(t), s)) => Ok(Expr::Str(t, s)),
            Some((Tok::Ident(first), s)) => {
                if first == "true" || first == "false" {
                    return Ok(Expr::Bool(first == "true", s));
                }
                if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            args.push(self.or()?);
                            if self.eat(&Tok::RParen) {
                                break;
                            }
                            if !self.eat(&Tok::Comma) {
                                return Err(LangError::new(
                                    "expected `,` or `)`",
                                    self.span_here(),
                                ));
                            }
                        }
                    }
                    let end = self.toks[self.pos - 1].1.end;
                    return Ok(Expr::Call(first, args, s.start..end));
                }
                let mut parts = vec![first];
                let mut end = s.end;
                while self.eat(&Tok::Dot) {
                    match self.bump() {
                        Some((Tok::Ident(p), ps)) => {
                            end = ps.end;
                            parts.push(p);
                        }
                        _ => {
                            return Err(LangError::new(
                                "expected a name after `.`",
                                self.span_here(),
                            ));
                        }
                    }
                }
                Ok(Expr::Path(parts, s.start..end))
            }
            Some((_, s)) => Err(LangError::new("expected a value or a name", s)),
            None => Err(LangError::new("condition ends too early", here)),
        }
    }
}

fn join(mut items: Vec<Expr>, make: fn(Vec<Expr>, Span) -> Expr) -> Expr {
    if items.len() == 1 {
        return items.pop().expect("one item");
    }
    let span = items[0].span().start..items[items.len() - 1].span().end;
    make(items, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};

    #[test]
    fn precedence() {
        let e = parse_expr("a or b and not c").unwrap();
        assert!(let Expr::Or(items, _) = e);
        check!(items.len() == 2);
        check!(matches!(items[1], Expr::And(_, _)));
    }

    #[test]
    fn comparison_and_paths() {
        let e = parse_expr("time.phase == night").unwrap();
        assert!(let Expr::Cmp(lhs, CmpOp::Eq, rhs, _) = e);
        check!(*lhs == Expr::Path(vec!["time".into(), "phase".into()], 0..10));
        check!(*rhs == Expr::Path(vec!["night".into()], 14..19));
    }

    #[test]
    fn calls_and_is_not() {
        let e = parse_expr("visited(cellar) && weather is not fog").unwrap();
        assert!(let Expr::And(items, _) = e);
        check!(
            matches!(&items[0], Expr::Call(name, args, _) if name == "visited" && args.len() == 1)
        );
        check!(matches!(&items[1], Expr::Cmp(_, CmpOp::Ne, _, _)));
    }

    #[test]
    fn errors_point_at_the_problem() {
        let err = parse_expr("lamp.lit and").unwrap_err();
        check!(err.span == (12..12));
        let err = parse_expr("lamp.lit $").unwrap_err();
        check!(err.span == (9..10));
    }
}
