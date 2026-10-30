//! A reader for the MDL syntax that ZIL source files use.

use std::fmt;

/// One MDL value.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Atom(String),
    Int(i64),
    Str(String),
    /// `<...>`. The empty form `<>` is false.
    Form(Vec<Node>),
    /// `(...)`.
    List(Vec<Node>),
    /// `[...]`.
    Vector(Vec<Node>),
    /// `,X`: the global value of `X`.
    Gval(Box<Node>),
    /// `.X`: the local value of `X`.
    Lval(Box<Node>),
    /// `'X`, `%X`, `!X` and `#X Y`. Kept so that nothing is lost, but not interpreted.
    Prefixed(char, Box<Node>),
}

impl Node {
    pub fn atom(&self) -> Option<&str> {
        match self {
            Node::Atom(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_atom(&self, name: &str) -> bool {
        self.atom() == Some(name)
    }

    pub fn string(&self) -> Option<&str> {
        match self {
            Node::Str(s) => Some(s),
            _ => None,
        }
    }

    /// `,X` gives `X`.
    pub fn gval(&self) -> Option<&str> {
        match self {
            Node::Gval(n) => n.atom(),
            _ => None,
        }
    }

    /// `.X` gives `X`.
    pub fn lval(&self) -> Option<&str> {
        match self {
            Node::Lval(n) => n.atom(),
            _ => None,
        }
    }

    /// `<HEAD args...>` gives `("HEAD", args)`.
    pub fn form(&self) -> Option<(&str, &[Node])> {
        match self {
            Node::Form(items) => {
                let (head, rest) = items.split_first()?;
                Some((head.atom()?, rest))
            }
            _ => None,
        }
    }

    pub fn list(&self) -> Option<&[Node]> {
        match self {
            Node::List(items) => Some(items),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadError {
    /// Character offset in the source.
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at character {}: {}", self.offset, self.message)
    }
}

/// Read all top-level values of a source file. Comments (`;` and the value after it) are dropped.
pub fn read(src: &str) -> Result<Vec<Node>, ReadError> {
    let mut reader = Reader {
        chars: src.chars().collect(),
        pos: 0,
    };
    let mut out = Vec::new();
    while let Some(node) = reader.node()? {
        out.push(node);
    }
    if let Some(c) = reader.peek() {
        return Err(reader.error(format!("unexpected `{c}`")));
    }
    Ok(out)
}

struct Reader {
    chars: Vec<char>,
    pos: usize,
}

const DELIMITERS: &[char] = &['<', '>', '(', ')', '[', ']', '{', '}', '"', ';'];

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn error(&self, message: impl Into<String>) -> ReadError {
        ReadError {
            offset: self.pos,
            message: message.into(),
        }
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    /// The next value, or `None` at a closing bracket (not consumed) or at the end.
    fn node(&mut self) -> Result<Option<Node>, ReadError> {
        loop {
            self.skip_space();
            let Some(c) = self.peek() else {
                return Ok(None);
            };
            let node = match c {
                ';' => {
                    self.pos += 1;
                    self.required()?;
                    continue;
                }
                '>' | ')' | ']' | '}' => return Ok(None),
                '<' => {
                    self.pos += 1;
                    Node::Form(self.sequence('>')?)
                }
                '(' => {
                    self.pos += 1;
                    Node::List(self.sequence(')')?)
                }
                '[' => {
                    self.pos += 1;
                    Node::Vector(self.sequence(']')?)
                }
                '"' => {
                    self.pos += 1;
                    Node::Str(self.string()?)
                }
                ',' => {
                    self.pos += 1;
                    Node::Gval(Box::new(self.required()?))
                }
                '.' => {
                    self.pos += 1;
                    Node::Lval(Box::new(self.required()?))
                }
                '\'' | '%' | '!' => {
                    self.pos += 1;
                    Node::Prefixed(c, Box::new(self.required()?))
                }
                '#' => {
                    self.pos += 1;
                    let kind = self.required()?;
                    let value = self.required()?;
                    Node::Prefixed('#', Box::new(Node::List(vec![kind, value])))
                }
                _ => self.atom(),
            };
            return Ok(Some(node));
        }
    }

    fn required(&mut self) -> Result<Node, ReadError> {
        self.node()?.ok_or_else(|| self.error("a value is missing"))
    }

    fn sequence(&mut self, close: char) -> Result<Vec<Node>, ReadError> {
        let mut items = Vec::new();
        loop {
            if let Some(node) = self.node()? {
                items.push(node);
                continue;
            }
            return match self.peek() {
                Some(c) if c == close => {
                    self.pos += 1;
                    Ok(items)
                }
                Some(c) => Err(self.error(format!("expected `{close}`, found `{c}`"))),
                None => Err(self.error(format!("`{close}` is missing at the end"))),
            };
        }
    }

    fn string(&mut self) -> Result<String, ReadError> {
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error("a string is never closed")),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.pos += 1;
                    if let Some(c) = self.peek() {
                        out.push(c);
                        self.pos += 1;
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn atom(&mut self) -> Node {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || DELIMITERS.contains(&c) {
                break;
            }
            self.pos += 1;
            if c == '\\' {
                if let Some(escaped) = self.peek() {
                    name.push(escaped);
                    self.pos += 1;
                }
            } else {
                name.push(c);
            }
        }
        match name.parse::<i64>() {
            Ok(i) => Node::Int(i),
            Err(_) => Node::Atom(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn reads_a_room() {
        let src = r#"<ROOM WEST-OF-HOUSE ;"comment"
      (DESC "West of House")
      (SW TO STONE-BARROW IF WON-FLAG)
      (FLAGS RLANDBIT ONBIT)>"#;
        let nodes = read(src).unwrap();
        let (head, args) = nodes[0].form().unwrap();
        check!(head == "ROOM");
        check!(args.len() == 4);
        check!(args[1].list().unwrap()[1] == Node::Str("West of House".into()));
    }

    #[test]
    fn reads_prefixes_and_escapes() {
        let nodes = read(r#"<TELL "a \"b\"" D ,LAMP .RARG <> 42>"#).unwrap();
        let (_, args) = nodes[0].form().unwrap();
        check!(args[0].string() == Some("a \"b\""));
        check!(args[2].gval() == Some("LAMP"));
        check!(args[3].lval() == Some("RARG"));
        check!(args[4] == Node::Form(vec![]));
        check!(args[5] == Node::Int(42));
    }

    #[test]
    fn reports_an_unclosed_form() {
        check!(read("<FOO (BAR)").is_err());
    }
}
