//! Translation of small, common pieces of ZIL code: conditions and room descriptions.
//!
//! The importer does not run ZIL. It only knows a few patterns. Code outside these
//! patterns is reported, so an author can port it by hand.

use crate::out::Frag;
use crate::reader::Node;
use crate::source::{Routine, Source};
use std::collections::{BTreeMap, BTreeSet};

/// A loremaster id for a ZIL name: `WEST-OF-HOUSE` gives `west_of_house`.
pub fn id(zil: &str) -> String {
    let mut out: String = zil
        .chars()
        .map(|c| match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => c,
            _ => '_',
        })
        .collect();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    // Names that the condition language uses for itself.
    if matches!(
        out.as_str(),
        "here" | "turns" | "time" | "night" | "action" | "first_visit" | "true" | "false"
    ) {
        out.push_str("_zil");
    }
    out
}

/// Clean the text of a ZIL string for inline markup: one space between words, braces escaped.
pub fn text(zil: &str) -> String {
    let mut out = String::with_capacity(zil.len());
    let mut space = false;
    for c in zil.chars() {
        // `|` is a line break in ZIL strings.
        if c.is_whitespace() || c == '|' {
            space = true;
            continue;
        }
        if space {
            out.push(' ');
        }
        space = false;
        match c {
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            c => out.push(c),
        }
    }
    if space {
        out.push(' ');
    }
    out
}

/// A short ZIL rendering of a node, for the import report.
pub fn show(node: &Node) -> String {
    let s = render(node);
    if s.chars().count() > 70 {
        let cut: String = s.chars().take(67).collect();
        format!("{cut}...")
    } else {
        s
    }
}

fn render(node: &Node) -> String {
    let join = |items: &[Node]| items.iter().map(render).collect::<Vec<_>>().join(" ");
    match node {
        Node::Atom(a) => a.clone(),
        Node::Int(i) => i.to_string(),
        Node::Str(s) => format!("\"{}\"", text(s).trim()),
        Node::Form(items) => format!("<{}>", join(items)),
        Node::List(items) => format!("({})", join(items)),
        Node::Vector(items) => format!("[{}]", join(items)),
        Node::Gval(n) => format!(",{}", render(n)),
        Node::Lval(n) => format!(".{}", render(n)),
        Node::Prefixed(c, n) => format!("{c}{}", render(n)),
    }
}

/// Translates conditions and collects the ZIL globals that they use as flags.
pub struct Translator<'a> {
    pub src: &'a Source,
    /// ZIL names of the true/false globals that conditions use.
    pub flags: BTreeSet<String>,
    /// ZIL names of the number globals that conditions use.
    pub numbers: BTreeSet<String>,
    /// Local variables of the routine that is translated now, set to a condition with `<SET X ...>`.
    locals: BTreeMap<String, String>,
}

impl<'a> Translator<'a> {
    pub fn new(src: &'a Source) -> Self {
        Self {
            src,
            flags: BTreeSet::new(),
            numbers: BTreeSet::new(),
            locals: BTreeMap::new(),
        }
    }

    /// Is this ZIL global a true/false flag?
    pub fn is_flag(&self, name: &str) -> bool {
        match self.src.globals.get(name) {
            Some(Node::Form(items)) => items.is_empty(),
            Some(Node::Atom(a)) => a == "T",
            _ => false,
        }
    }

    /// Use a ZIL global as a loremaster variable.
    pub fn flag(&mut self, name: &str) -> Option<String> {
        if !self.is_flag(name) {
            return None;
        }
        self.flags.insert(name.to_string());
        Some(id(name))
    }

    /// Use a ZIL global that holds a number as a loremaster variable.
    fn number(&mut self, node: &Node) -> Option<String> {
        let name = node.gval()?;
        if !matches!(self.src.globals.get(name), Some(Node::Int(_))) {
            return None;
        }
        self.numbers.insert(name.to_string());
        Some(id(name))
    }

    fn object(&self, node: &Node) -> Option<String> {
        let name = node.gval()?;
        self.src.object(name).map(|o| id(&o.name))
    }

    fn room(&self, node: &Node) -> Option<String> {
        let name = node.gval()?;
        self.src.room(name).map(|r| id(&r.name))
    }

    /// A loremaster condition for a ZIL test, or the reason why there is none.
    pub fn condition(&mut self, node: &Node) -> Result<String, String> {
        let fail = || Err(format!("condition `{}`", show(node)));
        if let Some(name) = node.gval() {
            return self.flag(name).map_or_else(fail, Ok);
        }
        if let Some(local) = node.lval() {
            return self.locals.get(local).cloned().map_or_else(fail, Ok);
        }
        let Some((head, args)) = node.form() else {
            return fail();
        };
        match (head, args) {
            ("NOT", [inner]) => Ok(format!("not {}", group(&self.condition(inner)?))),
            ("AND" | "OR", items) if !items.is_empty() => {
                let joiner = if head == "AND" { " and " } else { " or " };
                let parts = items
                    .iter()
                    .map(|i| self.condition(i).map(|c| group(&c)))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(parts.join(joiner))
            }
            ("0?", [n]) => match self.number(n) {
                Some(n) => Ok(format!("{n} == 0")),
                None => fail(),
            },
            ("G?" | "L?" | "GRTR?" | "LESS?", [n, Node::Int(limit)]) => match self.number(n) {
                Some(n) => {
                    let op = if head.starts_with('G') { ">" } else { "<" };
                    Ok(format!("{n} {op} {limit}"))
                }
                None => fail(),
            },
            ("FSET?", [obj, flag]) => {
                let Some(obj) = self.object(obj) else {
                    return fail();
                };
                match flag.gval() {
                    Some("OPENBIT") => Ok(format!("{obj}.open")),
                    Some("ONBIT") => Ok(format!("{obj}.lit")),
                    _ => fail(),
                }
            }
            ("IN?", [obj, holder]) if matches!(holder.gval(), Some("WINNER" | "PLAYER")) => {
                match self.object(obj) {
                    Some(obj) => Ok(format!("carried({obj})")),
                    None => fail(),
                }
            }
            ("EQUAL?" | "==?", [first, rooms @ ..])
                if first.gval() == Some("HERE") && !rooms.is_empty() =>
            {
                let parts = rooms
                    .iter()
                    .map(|r| self.room(r).map(|r| format!("here == {r}")))
                    .collect::<Option<Vec<_>>>();
                match parts {
                    Some(p) => Ok(p.join(" or ")),
                    None => fail(),
                }
            }
            _ => fail(),
        }
    }

    /// The fragments of a room description that a room routine prints for `M-LOOK`.
    pub fn look(&mut self, routine: &Routine) -> Result<Vec<Frag>, String> {
        let Some(body) = look_clause(&routine.body) else {
            return Err("the routine has no M-LOOK branch".into());
        };
        self.locals.clear();
        let mut w = Writer::default();
        for stmt in body {
            self.statement(&mut w, stmt)?;
        }
        w.flush();
        if w.out.is_empty() {
            return Err("the M-LOOK branch prints no text".into());
        }
        Ok(w.out)
    }

    fn statement(&mut self, w: &mut Writer, node: &Node) -> Result<(), String> {
        // A routine often returns `T` at the end of a branch.
        if node.is_atom("T") {
            return Ok(());
        }
        let Some((head, args)) = node.form() else {
            return Err(format!("statement `{}`", show(node)));
        };
        match head {
            "SET" => {
                let [Node::Atom(local), value] = args else {
                    return Err(format!("statement `{}`", show(node)));
                };
                let c = self.condition(value)?;
                self.locals.insert(local.clone(), c);
                Ok(())
            }
            "TELL" => {
                let mut items = args.iter();
                while let Some(item) = items.next() {
                    match item {
                        Node::Str(s) => w.text.push_str(&text(s)),
                        Node::Atom(a) if a == "CR" || a == "CRLF" => w.flush(),
                        Node::Atom(a) if a == "D" => {
                            let obj = items.next().and_then(|o| self.object(o));
                            let Some(obj) = obj else {
                                return Err(format!("`{}`", show(node)));
                            };
                            w.text.push_str(&format!("{{{obj}.name}}"));
                        }
                        _ => return Err(format!("`{}`", show(node))),
                    }
                }
                Ok(())
            }
            "CRLF" => {
                w.flush();
                Ok(())
            }
            "RTRUE" | "RFALSE" => Ok(()),
            "COND" => {
                // Inside a sentence, the COND becomes inline markup; else each branch becomes a fragment.
                let open = w.text.trim_end();
                let mid_sentence = !open.is_empty() && !open.ends_with(['.', '!', '?']);
                if mid_sentence && let Some((markup, ends_line)) = self.inline(args) {
                    w.text.push_str(&markup);
                    if ends_line {
                        w.flush();
                    }
                    return Ok(());
                }
                w.flush();
                let mut earlier: Vec<String> = Vec::new();
                for clause in args {
                    let Some((test, body)) = clause.list().and_then(<[Node]>::split_first) else {
                        return Err(format!("`{}`", show(node)));
                    };
                    let this = if test.is_atom("T") || test.is_atom("ELSE") {
                        None
                    } else {
                        Some(self.condition(test)?)
                    };
                    let mut parts: Vec<String> = earlier
                        .iter()
                        .map(|c| format!("not {}", group(c)))
                        .collect();
                    parts.extend(this.clone());
                    w.conditions.push(parts);
                    for stmt in body {
                        self.statement(w, stmt)?;
                    }
                    w.flush();
                    w.conditions.pop();
                    match this {
                        Some(c) => earlier.push(c),
                        None => break,
                    }
                }
                Ok(())
            }
            _ => Err(format!("statement `{}`", show(node))),
        }
    }
}

impl Translator<'_> {
    /// A COND whose branches only print short text, as inline markup:
    /// `{a ? x | {b ? y | z}}`. The text stays in one sentence, so no space appears before
    /// punctuation such as `. To the west`.
    /// Also tells if the branches end the line.
    fn inline(&mut self, clauses: &[Node]) -> Option<(String, bool)> {
        let mut ends_line = false;
        let mut branches: Vec<(Option<String>, String)> = Vec::new();
        for clause in clauses {
            let (test, body) = clause.list()?.split_first()?;
            let cond = if test.is_atom("T") || test.is_atom("ELSE") {
                None
            } else {
                Some(self.condition(test).ok()?)
            };
            let mut out = String::new();
            for stmt in body {
                let ("TELL", items) = stmt.form()? else {
                    return None;
                };
                for item in items {
                    match item {
                        Node::Str(s) => out.push_str(&text(s)),
                        // A line break at the end of a branch. Fragments are joined with spaces.
                        Node::Atom(a) if a == "CR" || a == "CRLF" => ends_line = true,
                        _ => return None,
                    }
                }
            }
            if out.contains(['?', ':', '|', '{', '}']) {
                return None;
            }
            let last = cond.is_none();
            branches.push((cond, out));
            if last {
                break;
            }
        }
        if branches.last().is_some_and(|(c, _)| c.is_some()) {
            branches.push((None, String::new()));
        }
        // Branches lose the spaces at their ends, so all of them must agree on these spaces.
        let lead = branches[0].1.starts_with(' ');
        let trail = branches[0].1.ends_with(' ');
        let agree = branches.iter().all(|(_, t)| {
            if t.is_empty() {
                !lead && !trail
            } else {
                t.starts_with(' ') == lead && t.ends_with(' ') == trail
            }
        });
        if !agree {
            return None;
        }
        let mut iter = branches.into_iter().rev();
        let mut markup = iter.next()?.1.trim().to_string();
        for (cond, then) in iter {
            let cond = cond?;
            markup = if markup.is_empty() {
                format!("{{{cond} ? {}}}", then.trim())
            } else {
                format!("{{{cond} ? {} | {markup}}}", then.trim())
            };
        }
        let markup = format!(
            "{}{markup}{}",
            if lead { " " } else { "" },
            if trail { " " } else { "" }
        );
        Some((markup, ends_line))
    }
}

/// Put a compound condition in parentheses.
fn group(c: &str) -> String {
    if c.contains(' ') && !(c.starts_with('(') && c.ends_with(')')) {
        format!("({c})")
    } else {
        c.to_string()
    }
}

/// The statements of the `(<EQUAL? .RARG ,M-LOOK> ...)` clause of a room routine.
fn look_clause(body: &[Node]) -> Option<&[Node]> {
    body.iter().find_map(|stmt| {
        let ("COND", clauses) = stmt.form()? else {
            return None;
        };
        clauses.iter().find_map(|clause| {
            let (test, rest) = clause.list()?.split_first()?;
            let (head, args) = test.form()?;
            let is_look = matches!(head, "EQUAL?" | "==?")
                && args.first()?.lval().is_some()
                && args.iter().skip(1).any(|a| a.gval() == Some("M-LOOK"));
            is_look.then_some(rest)
        })
    })
}

/// Collects text into fragments. A new fragment starts at each line break and each change of condition.
#[derive(Default)]
struct Writer {
    out: Vec<Frag>,
    text: String,
    /// One list of conditions for each open COND clause.
    conditions: Vec<Vec<String>>,
}

impl Writer {
    fn flush(&mut self) {
        let text = std::mem::take(&mut self.text);
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let parts: Vec<String> = self.conditions.iter().flatten().cloned().collect();
        self.out.push(Frag {
            text: text.to_string(),
            when: match parts.len() {
                0 => None,
                1 => Some(parts[0].clone()),
                _ => Some(
                    parts
                        .iter()
                        .map(|p| group(p))
                        .collect::<Vec<_>>()
                        .join(" and "),
                ),
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn source() -> Source {
        let mut src = Source::default();
        src.add_file(
            "test.zil",
            r#"
<GLOBAL WON-FLAG <>>
<OBJECT KITCHEN-WINDOW (DESC "kitchen window")>
<ROOM KITCHEN (DESC "Kitchen")>
<ROUTINE KITCHEN-FCN (RARG)
  <COND (<EQUAL? .RARG ,M-LOOK>
         <TELL "A window   which is ">
         <COND (<FSET? ,KITCHEN-WINDOW ,OPENBIT> <TELL "open." CR>)
               (T <TELL "slightly ajar." CR>)>
         <COND (,WON-FLAG <TELL "A path leads away.">)>
         <CRLF>)>>
"#,
        );
        src
    }

    #[test]
    fn makes_ids() {
        check!(id("WEST-OF-HOUSE") == "west_of_house");
        check!(id("HERE") == "here_zil");
    }

    #[test]
    fn cleans_text() {
        check!(text("You are\n   here.  ") == "You are here. ");
        check!(text("a {b}") == "a {{b}}");
    }

    #[test]
    fn translates_conditions() {
        let src = source();
        let mut t = Translator::new(&src);
        let node = crate::reader::read("<AND ,WON-FLAG <NOT <FSET? ,KITCHEN-WINDOW ,OPENBIT>>>")
            .unwrap()
            .remove(0);
        check!(t.condition(&node) == Ok("won_flag and (not kitchen_window.open)".to_string()));
        check!(t.flags.contains("WON-FLAG"));
    }

    #[test]
    fn translates_locals_and_numbers() {
        let mut src = Source::default();
        src.add_file(
            "test.zil",
            r#"<GLOBAL RUG-MOVED <>> <GLOBAL WRATH 0>
<ROUTINE R (RARG "AUX" RUG?)
  <COND (<EQUAL? .RARG ,M-LOOK>
         <SET RUG? ,RUG-MOVED>
         <COND (.RUG? <TELL "Moved.">) (<G? ,WRATH 0> <TELL "Angry.">)>
         T)>>"#,
        );
        let mut t = Translator::new(&src);
        let frags = t.look(&src.routines["R"]).unwrap();
        check!(frags[1] == Frag::when("(not rug_moved) and (wrath > 0)", "Angry."));
        check!(t.numbers.contains("WRATH"));
    }

    #[test]
    fn translates_a_look_routine() {
        let src = source();
        let mut t = Translator::new(&src);
        let frags = t.look(&src.routines["KITCHEN-FCN"]).unwrap();
        check!(
            frags
                == vec![
                    Frag::plain("A window which is {kitchen_window.open ? open. | slightly ajar.}"),
                    Frag::when("won_flag", "A path leads away."),
                ]
        );
    }
}
