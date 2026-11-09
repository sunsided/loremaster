//! The grammar of `world!` (DSL steps L3 and L4): story header, vars,
//! rooms, things, rules, machines, timers, endings, verbs, dialogues,
//! threads, behaviors and walkthrough tests, plus `extern` ids of other
//! packs. Read the crate docs for the
//! grammar by example.

use crate::ast::*;
use crate::cond::parse_until;
use dramaturge_syntax::Place;
use dramaturge_syntax::tokens::{ParseHost, parse_dialogue};
use proc_macro2::{Ident, TokenStream};
use syn::buffer::Cursor;
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::parse::{Parse, ParseStream};
use syn::{Error, LitFloat, LitInt, LitStr, Result, Token, braced, bracketed, parenthesized};

/// The items of a block, for the error on an unknown one.
const ITEMS: &str = "`extern`, `story`, `var`, `interface`, `room`, `thing`, `rule`, `machine`, `timer`, `ending`, `verb`, `dialogue`, `thread`, `behavior` or `test`";

/// The items that `replace` can stand before.
const REPLACEABLE: &[&str] = &[
    "room", "thing", "machine", "timer", "ending", "verb", "dialogue", "thread",
];

impl Parse for World {
    fn parse(input: ParseStream) -> Result<Self> {
        // `pack <name>;` first: the name of the function.
        let mut name_ = None;
        if peek_kw(input, "pack") {
            name(input)?;
            let n = name(input)?;
            // A keyword (`loop`, also `r#loop`) can not be the name of a function here.
            if syn::parse_str::<Ident>(&n.value).is_err() {
                return Err(Error::new(
                    n.span,
                    format!(
                        "`{}` is a Rust keyword; give the pack another name",
                        n.value
                    ),
                ));
            }
            semi(input)?;
            name_ = Some(n);
        }
        let mut items = Vec::new();
        while !input.is_empty() {
            if peek_kw(input, "pack") {
                return Err(input.error(
                    "`pack <name>;` names the function of the block; it comes first, once",
                ));
            }
            items.push(item(input)?);
        }
        Ok(World { name: name_, items })
    }
}

fn item(input: ParseStream) -> Result<Item> {
    if input.peek(Token![extern]) {
        input.parse::<Token![extern]>()?;
        return externs(input).map(Item::Extern);
    }
    let mut replace = false;
    if peek_kw(input, "replace") {
        let word = name(input)?;
        if !peek_word(input).is_some_and(|w| REPLACEABLE.contains(&w.as_str())) {
            return Err(Error::new(
                word.span,
                "`replace` stands before a `room`, `thing`, `machine`, `timer`, `ending`, `verb`, `dialogue` or `thread`",
            ));
        }
        replace = true;
    }
    let word = peek_word(input).ok_or_else(|| input.error(format!("expected {ITEMS}")))?;
    Ok(match word.as_str() {
        "story" => Item::Story(story(input)?),
        "var" => Item::Var(var(input)?),
        "interface" => interface(input)?,
        "room" => Item::Room(Room {
            replace,
            ..room(input)?
        }),
        "thing" => Item::Thing(Thing {
            replace,
            ..thing(input)?
        }),
        "rule" => Item::Rule(rule(input)?),
        "machine" => Item::Machine(Machine {
            replace,
            ..machine(input)?
        }),
        "timer" => Item::Timer(Timer {
            replace,
            ..timer(input)?
        }),
        "ending" => Item::Ending(Ending {
            replace,
            ..ending(input)?
        }),
        "verb" => Item::Verb(Verb {
            replace,
            ..verb(input)?
        }),
        "test" => Item::Test(test(input)?),
        "dialogue" => Item::Dialogue(Dialogue {
            replace,
            ..dialogue(input)?
        }),
        "thread" => Item::Thread(Thread {
            replace,
            ..thread(input)?
        }),
        "behavior" => {
            name(input)?;
            // A lone lowercase word is an id; anything else is the instance.
            let fork = input.fork();
            let lone = name(&fork).is_ok_and(|n| {
                n.value
                    .starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
            }) && fork.peek(Token![;]);
            let item = if lone {
                Item::Behavior(name(input)?)
            } else {
                Item::Register(input.parse()?)
            };
            semi(input)?;
            item
        }
        "texts" => {
            return Err(unsupported(
                input,
                "text pools (`texts`) are not in `world!` yet: they wait for the text core (R9, P4-01 #78)",
            ));
        }
        "extend" => {
            return Err(unsupported(
                input,
                "`extend` is not in `world!` yet: an extension needs the fields as data (later work); write the whole entity with `replace`, or extend it in YAML",
            ));
        }
        "agenda" => {
            return Err(unsupported(
                input,
                "agendas are not in `world!` yet: they wait for actors and agendas (D6, P4-02 #79)",
            ));
        }
        _ => return Err(input.error(format!("expected {ITEMS}"))),
    })
}

fn unsupported(input: ParseStream, message: &str) -> Error {
    input.error(message)
}

// ---------------------------------------------------------------- helpers

/// The next token as a word (an identifier or a keyword), without reading it.
fn peek_word(input: ParseStream) -> Option<String> {
    let fork = input.fork();
    fork.call(Ident::parse_any).ok().map(|i| unraw(&i))
}

fn peek_kw(input: ParseStream, kw: &str) -> bool {
    peek_word(input).as_deref() == Some(kw)
}

fn unraw(ident: &Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").map(str::to_string).unwrap_or(s)
}

/// Read the keyword `kw`.
fn keyword(input: ParseStream, kw: &str) -> Result<Ident> {
    if peek_kw(input, kw) {
        input.call(Ident::parse_any)
    } else {
        Err(input.error(format!("expected `{kw}`")))
    }
}

fn name(input: ParseStream) -> Result<Name> {
    let ident = input
        .call(Ident::parse_any)
        .map_err(|e| Error::new(e.span(), "expected an id"))?;
    Ok(Name {
        value: unraw(&ident),
        span: ident.span(),
    })
}

fn text(input: ParseStream) -> Result<Text> {
    let lit: LitStr = input
        .parse()
        .map_err(|e| Error::new(e.span(), "expected a string"))?;
    Ok(Text {
        value: lit.value(),
        span: lit.span(),
    })
}

/// A word or a string: `jump` or `"jump in"`.
fn word_or_text(input: ParseStream) -> Result<Text> {
    if input.peek(LitStr) {
        text(input)
    } else {
        let n = name(input).map_err(|e| Error::new(e.span(), "expected a word or a string"))?;
        Ok(Text {
            value: n.value,
            span: n.span,
        })
    }
}

/// `[a, "b c", d]`: words or strings.
fn words(input: ParseStream) -> Result<Vec<Text>> {
    let content;
    bracketed!(content in input);
    let list = content.parse_terminated(word_or_text, Token![,])?;
    Ok(list.into_iter().collect())
}

/// `[a, b]`: ids.
fn names(input: ParseStream) -> Result<Vec<Name>> {
    let content;
    bracketed!(content in input);
    let list = content.parse_terminated(name, Token![,])?;
    Ok(list.into_iter().collect())
}

/// One string, or a list of strings: `"x"` or `["x", "y"]`.
fn texts(input: ParseStream) -> Result<Vec<Text>> {
    if input.peek(syn::token::Bracket) {
        let content;
        bracketed!(content in input);
        let list = content.parse_terminated(text, Token![,])?;
        Ok(list.into_iter().collect())
    } else {
        Ok(vec![text(input)?])
    }
}

fn semi(input: ParseStream) -> Result<()> {
    input
        .parse::<Token![;]>()
        .map(|_| ())
        .map_err(|e| Error::new(e.span(), "expected `;`"))
}

fn int<T: std::str::FromStr>(input: ParseStream) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let negative = input.peek(Token![-]);
    if negative {
        input.parse::<Token![-]>()?;
    }
    let lit: LitInt = input.parse()?;
    let digits = if negative {
        format!("-{}", lit.base10_digits())
    } else {
        lit.base10_digits().to_string()
    };
    digits
        .parse::<T>()
        .map_err(|e| Error::new(lit.span(), e.to_string()))
}

fn float(input: ParseStream) -> Result<f32> {
    let negative = input.peek(Token![-]);
    if negative {
        input.parse::<Token![-]>()?;
    }
    let span = input.span();
    let value: f32 = if input.peek(LitFloat) {
        input.parse::<LitFloat>()?.base10_parse()?
    } else {
        input.parse::<LitInt>()?.base10_parse()?
    };
    if !value.is_finite() {
        return Err(Error::new(span, "the number is too large for a position"));
    }
    Ok(if negative { -value } else { value })
}

/// A value: `true`, `false`, a number, a word (an enum value) or a string.
fn value(input: ParseStream) -> Result<Value> {
    if input.peek(syn::LitBool) {
        return Ok(Value::Bool(input.parse::<syn::LitBool>()?.value));
    }
    if input.peek(LitInt) || input.peek(Token![-]) {
        return Ok(Value::Int(int(input)?));
    }
    if input.peek(LitStr) {
        return Ok(Value::Text(text(input)?.value));
    }
    Ok(Value::Text(name(input)?.value))
}

/// A dotted name: `stove.hot`.
fn path(input: ParseStream) -> Result<Name> {
    let first = name(input)?;
    let mut value = first.value;
    while input.peek(Token![.]) {
        input.parse::<Token![.]>()?;
        value.push('.');
        value.push_str(&name(input)?.value);
    }
    Ok(Name {
        value,
        span: first.span,
    })
}

/// The tokens from `start` up to the current place of `input`.
fn since(start: Cursor, input: ParseStream) -> TokenStream {
    let end = input.cursor();
    let mut out = TokenStream::new();
    let mut c = start;
    while c != end {
        match c.token_tree() {
            Some((tt, next)) => {
                out.extend([tt]);
                c = next;
            }
            None => break,
        }
    }
    out
}

/// The statements of a `{ ... }` body, each read by `f`.
fn body<T>(input: ParseStream, mut f: impl FnMut(ParseStream) -> Result<T>) -> Result<Vec<T>> {
    let content;
    braced!(content in input);
    let mut out = Vec::new();
    while !content.is_empty() {
        out.push(f(&content)?);
    }
    Ok(out)
}

// ------------------------------------------------------------------ items

fn externs(input: ParseStream) -> Result<Vec<Extern>> {
    body(input, |input| {
        let start = input.cursor();
        let kind = name(input)?;
        let id = name(input)?;
        let kind = match kind.value.as_str() {
            "room" => ExternKind::Room,
            "thing" => {
                let mut props: Vec<(Name, PropType)> = Vec::new();
                if input.peek(syn::token::Bracket) {
                    let content;
                    bracketed!(content in input);
                    while !content.is_empty() {
                        let prop = name(&content)?;
                        content.parse::<Token![:]>().map_err(|e| {
                            Error::new(e.span(), "a property needs its type: `hot: bool`")
                        })?;
                        let ty = name(&content)?;
                        let ty = match ty.value.as_str() {
                            "bool" => PropType::Bool,
                            "int" => PropType::Int,
                            "text" => PropType::Text,
                            _ => {
                                return Err(Error::new(
                                    ty.span,
                                    "expected `bool`, `int` or `text`",
                                ));
                            }
                        };
                        if let Some((first, _)) = props.iter().find(|(p, _)| p.value == prop.value)
                        {
                            let mut e = Error::new(
                                prop.span,
                                format!("the property `{}` is declared twice", prop.value),
                            );
                            e.combine(Error::new(first.span, "first declared here"));
                            return Err(e);
                        }
                        props.push((prop, ty));
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                ExternKind::Thing(props)
            }
            "var" => {
                let ty = if input.peek(Token![:]) {
                    input.parse::<Token![:]>()?;
                    var_type(input)?
                } else {
                    VarType::Bool
                };
                ExternKind::Var(ty)
            }
            "machine" => {
                if !input.peek(syn::token::Bracket) {
                    return Err(Error::new(
                        id.span,
                        format!(
                            "an extern machine needs its states, so that conditions on it check: `machine {} [a, b];`",
                            id.value
                        ),
                    ));
                }
                let states = names(input)?;
                if states.is_empty() {
                    return Err(Error::new(
                        id.span,
                        "an extern machine needs at least one state",
                    ));
                }
                ExternKind::Machine(states)
            }
            "timer" => ExternKind::Timer,
            "ending" => ExternKind::Ending,
            "verb" => ExternKind::Verb,
            "behavior" => ExternKind::Behavior,
            "relation" => ExternKind::Relation,
            "texts" => {
                return Err(Error::new(
                    kind.span,
                    "text pools (`texts`) are not in `world!` yet: they wait for the text core (R9, P4-01 #78)",
                ));
            }
            _ => {
                return Err(Error::new(
                    kind.span,
                    "expected `room`, `thing`, `var`, `machine`, `timer`, `ending`, `verb`, `behavior` or `relation`",
                ));
            }
        };
        semi(input)?;
        Ok(Extern {
            kind,
            id,
            tokens: since(start, input),
        })
    })
}

fn story(input: ParseStream) -> Result<Story> {
    let start = input.cursor();
    keyword(input, "story")?;
    let title = text(input)?;
    let tokens = since(start, input);
    let mut story = Story {
        title,
        start: None,
        author: None,
        seed: None,
        intro: Vec::new(),
        death_prompt: None,
        fresh_rolls: None,
        clock_start: None,
        minutes_per_turn: None,
        phases: None,
        relations: Vec::new(),
        score: None,
        score_var: None,
        ranks: Vec::new(),
        stats: Vec::new(),
        tokens,
    };
    let content;
    braced!(content in input);
    while !content.is_empty() {
        let field = name(&content)?;
        match field.value.as_str() {
            "clock" => clock(&content, &mut story)?,
            "stats" => {
                let list;
                braced!(list in content);
                while !list.is_empty() {
                    let var = path(&list)?;
                    let mut stat = loremaster_model::Stat {
                        var: var.value.clone(),
                        label: text(&list)?.value,
                        short: None,
                        max: None,
                    };
                    while let Some(key @ ("short" | "max")) = peek_word(&list).as_deref() {
                        let at = name(&list)?.span;
                        let taken = if key == "short" {
                            stat.short.replace(text(&list)?.value).is_some()
                        } else {
                            let max = if list.peek(LitInt) || list.peek(Token![-]) {
                                loremaster_model::Limit::Value(int(&list)?)
                            } else {
                                loremaster_model::Limit::Var(loremaster_model::Id::new(
                                    path(&list)?.value,
                                ))
                            };
                            stat.max.replace(max).is_some()
                        };
                        if taken {
                            return Err(Error::new(at, format!("`{key}` is given twice")));
                        }
                    }
                    story.stats.push((var, stat));
                    if !list.is_empty() {
                        list.parse::<Token![,]>()?;
                    }
                }
            }
            "ranks" => {
                let list;
                braced!(list in content);
                while !list.is_empty() {
                    let from = int(&list)?;
                    let rank = text(&list)?;
                    story.ranks.push((from, rank));
                    if !list.is_empty() {
                        list.parse::<Token![,]>()?;
                    }
                }
            }
            _ => {
                content.parse::<Token![:]>()?;
                match field.value.as_str() {
                    "author" => story.author = Some(text(&content)?),
                    "start" => story.start = Some(name(&content)?),
                    "seed" => story.seed = Some(int(&content)?),
                    "intro" => story.intro.push(frag(&content)?),
                    "death_prompt" => {
                        story.death_prompt = Some(content.parse::<syn::LitBool>()?.value)
                    }
                    "reload_rolls" => {
                        let mode = name(&content)?;
                        story.fresh_rolls = Some(match mode.value.as_str() {
                            "same" => false,
                            "fresh" => true,
                            _ => return Err(Error::new(mode.span, "expected `same` or `fresh`")),
                        });
                    }
                    "relations" => story.relations = names(&content)?,
                    "score" => story.score = Some(int(&content)?),
                    "score_var" => story.score_var = Some(name(&content)?),
                    _ => {
                        return Err(Error::new(
                            field.span,
                            "expected `author`, `start`, `seed`, `intro`, `death_prompt`, `reload_rolls`, `relations`, `score`, `score_var`, `ranks`, `stats` or `clock`",
                        ));
                    }
                }
            }
        }
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
    }
    if (story.score_var.is_some() || !story.ranks.is_empty()) && story.score.is_none() {
        return Err(Error::new(
            story.title.span,
            "`score_var` and `ranks` need `score: <max>`",
        ));
    }
    if story.start.is_none() {
        return Err(Error::new(
            story.title.span,
            "the story needs `start: <room>`",
        ));
    }
    Ok(story)
}

fn clock(input: ParseStream, story: &mut Story) -> Result<()> {
    let content;
    braced!(content in input);
    while !content.is_empty() {
        let field = name(&content)?;
        match field.value.as_str() {
            "phases" => {
                let list;
                braced!(list in content);
                let mut phases = Vec::new();
                while !list.is_empty() {
                    let phase = name(&list)?;
                    let from = text(&list)?;
                    phases.push((phase, from));
                    if !list.is_empty() {
                        list.parse::<Token![,]>()?;
                    }
                }
                if phases.is_empty() {
                    return Err(Error::new(
                        field.span,
                        "`phases { .. }` needs at least one phase: `night \"00:00\"`",
                    ));
                }
                story.phases = Some(phases);
            }
            "start" => {
                content.parse::<Token![:]>()?;
                story.clock_start = Some(text(&content)?);
            }
            "minutes_per_turn" => {
                content.parse::<Token![:]>()?;
                story.minutes_per_turn = Some(int(&content)?);
            }
            _ => {
                return Err(Error::new(
                    field.span,
                    "expected `start`, `minutes_per_turn` or `phases`",
                ));
            }
        }
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(())
}

fn var_type(input: ParseStream) -> Result<VarType> {
    if input.peek(syn::token::Bracket) {
        return Ok(VarType::Enum(names(input)?));
    }
    let ty = name(input)?;
    match ty.value.as_str() {
        "bool" => Ok(VarType::Bool),
        "int" => Ok(VarType::Int),
        _ => Err(Error::new(
            ty.span,
            "expected `bool`, `int` or a list of enum values `[a, b]`",
        )),
    }
}

/// `interface { action name(arg: int) -> a | b; event name(arg: text) when "cond"; }`.
fn interface(input: ParseStream) -> Result<Item> {
    use loremaster_model::{ActionDecl, EventDecl, Id, Interface};
    let at = keyword(input, "interface")?.span();
    let body;
    syn::braced!(body in input);
    let mut out = Interface::default();
    while !body.is_empty() {
        match peek_word(&body).as_deref() {
            Some("action") => {
                keyword(&body, "action")?;
                let id = Id::new(name(&body)?.value);
                let args = declared_args(&body)?;
                let mut results = Vec::new();
                if body.peek(Token![->]) {
                    body.parse::<Token![->]>()?;
                    results.push(Id::new(name(&body)?.value));
                    while body.peek(Token![|]) {
                        body.parse::<Token![|]>()?;
                        results.push(Id::new(name(&body)?.value));
                    }
                }
                out.actions.push(ActionDecl { id, args, results });
            }
            Some("event") => {
                keyword(&body, "event")?;
                let id = Id::new(name(&body)?.value);
                let args = declared_args(&body)?;
                let when = if peek_kw(&body, "when") {
                    keyword(&body, "when")?;
                    Some(text(&body)?.value)
                } else {
                    None
                };
                out.events.push(EventDecl { id, args, when });
            }
            _ => return Err(body.error("expected `action` or `event`")),
        }
        semi(&body)?;
    }
    Ok(Item::Interface(out, at))
}

/// `(a: int, b: text, c: [x, y])`; the parentheses can be left out.
fn declared_args(
    input: ParseStream,
) -> Result<std::collections::BTreeMap<String, loremaster_model::ArgType>> {
    use loremaster_model::ArgType;
    let mut out = std::collections::BTreeMap::new();
    if !input.peek(syn::token::Paren) {
        return Ok(out);
    }
    let body;
    syn::parenthesized!(body in input);
    while !body.is_empty() {
        let arg = name(&body)?;
        body.parse::<Token![:]>()?;
        let ty = if body.peek(syn::token::Bracket) {
            let list;
            syn::bracketed!(list in body);
            let mut values = Vec::new();
            while !list.is_empty() {
                values.push(name(&list)?.value);
                if !list.is_empty() {
                    list.parse::<Token![,]>()?;
                }
            }
            if values.is_empty() {
                return Err(Error::new(arg.span, "an enum needs at least one value"));
            }
            ArgType::Enum(values)
        } else {
            let word = name(&body)?;
            match word.value.as_str() {
                "bool" => ArgType::Bool,
                "int" => ArgType::Int,
                "text" => ArgType::Text,
                _ => {
                    return Err(Error::new(
                        word.span,
                        "expected `bool`, `int`, `text` or `[a, b]`",
                    ));
                }
            }
        };
        if out.contains_key(&arg.value) {
            return Err(Error::new(arg.span, format!("`{}` comes twice", arg.value)));
        }
        out.insert(arg.value, ty);
        if !body.is_empty() {
            body.parse::<Token![,]>()?;
        }
    }
    Ok(out)
}

fn var(input: ParseStream) -> Result<Var> {
    keyword(input, "var")?;
    let id = name(input)?;
    let ty = if input.peek(Token![:]) {
        input.parse::<Token![:]>()?;
        Some(var_type(input)?)
    } else {
        None
    };
    input.parse::<Token![=]>()?;
    let at = input.span();
    let initial = value(input)?;
    let (mut min, mut max) = (None, None);
    while let Some(word) = peek_word(input).filter(|w| w == "min" || w == "max") {
        name(input)?;
        let limit = if input.peek(LitInt) || input.peek(Token![-]) {
            let at = input.span();
            Name {
                value: int::<i64>(input)?.to_string(),
                span: at,
            }
        } else {
            name(input)?
        };
        *if word == "min" { &mut min } else { &mut max } = Some(limit);
    }
    semi(input)?;
    let ty = match (ty, &initial) {
        (Some(ty), _) => ty,
        (None, Value::Bool(_)) => VarType::Bool,
        (None, Value::Int(_)) => VarType::Int,
        (None, Value::Text(_)) => {
            return Err(Error::new(
                at,
                "an enum variable needs its values: `var mood: [calm, angry] = calm;`",
            ));
        }
    };
    let fits = matches!(
        (&ty, &initial),
        (VarType::Bool, Value::Bool(_)) | (VarType::Int, Value::Int(_))
    ) || matches!((&ty, &initial), (VarType::Enum(values), Value::Text(t)) if values.iter().any(|v| v.value == *t));
    if !fits {
        return Err(Error::new(at, "the initial value does not match the type"));
    }
    Ok(Var {
        id,
        ty,
        initial,
        min,
        max,
    })
}

const LAYERS: &[&str] = &["base", "ambient", "event", "objects", "characters", "exits"];
const MODES: &[&str] = &[
    "always",
    "first_visit",
    "once",
    "cycle",
    "shuffle",
    "sticky",
];

/// A fragment: `[layer] [mode] [when <cond> =>] "text"`, or `[..]` for
/// variants.
fn frag(input: ParseStream) -> Result<Frag> {
    let start = input.cursor();
    let mut layer = None;
    let mut mode = None;
    let mut when = None;
    if let Some(w) = peek_word(input)
        && LAYERS.contains(&w.as_str())
    {
        layer = Some(name(input)?);
    }
    if let Some(w) = peek_word(input)
        && MODES.contains(&w.as_str())
    {
        mode = Some(name(input)?);
    }
    if peek_kw(input, "when") {
        keyword(input, "when")?;
        when = Some(parse_until(input, |i| {
            i.peek(Token![=>]) || i.peek(Token![;])
        })?);
        input
            .parse::<Token![=>]>()
            .map_err(|e| Error::new(e.span(), "expected `=>` and the text after the condition"))?;
    }
    if peek_kw(input, "one_of") {
        return Err(input.error(
            "`one_of` text groups are not in `world!` yet: they wait for the text core (R9, P4-01 #78)",
        ));
    }
    let (texts, variants) = if input.peek(syn::token::Bracket) {
        let content;
        bracketed!(content in input);
        let list = content.parse_terminated(text, Token![,])?;
        (list.into_iter().collect(), true)
    } else {
        (vec![text(input)?], false)
    };
    if peek_kw(input, "says") || peek_kw(input, "narrate") || peek_word_then(input, "says") {
        return Err(speech(input));
    }
    let mut id = None;
    let mut replaces = false;
    let mut notes = Vec::new();
    loop {
        match peek_word(input).as_deref() {
            Some("id") => {
                name(input)?;
                id = Some(name(input)?);
            }
            Some("replaces") => {
                name(input)?;
                replaces = true;
            }
            Some("note") => {
                name(input)?;
                notes.push(text(input)?);
            }
            _ => break,
        }
    }
    Ok(Frag {
        layer,
        mode,
        when,
        texts,
        variants,
        id,
        replaces,
        notes,
        tokens: since(start, input),
    })
}

/// The second token is the word `kw`: `keeper says`.
fn peek_word_then(input: ParseStream, kw: &str) -> bool {
    let fork = input.fork();
    fork.call(Ident::parse_any).is_ok() && peek_kw(&fork, kw)
}

/// `<speaker> says "…"` starts here: a word, the word `says`, a string.
fn peek_speaker(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.call(Ident::parse_any).is_ok()
        && peek_kw(&fork, "says")
        && fork.call(Ident::parse_any).is_ok()
        && fork.peek(LitStr)
}

fn speech(input: ParseStream) -> Error {
    input.error(
        "speech segments (`says`, `narrate`) belong in a `line` effect; a description or a presence is one text",
    )
}

/// The segments of a `line`, up to `;`:
///
/// - `"…"`: narration, a bare string;
/// - `narrate "…"`: narration, written out;
/// - `says "…"`: speech with the default speaker, `keeper says "…"` or
///   `player says "…"` with one;
/// - `when <cond> then <segment>`: the one segment after `then` shows only
///   while the condition holds. A bare string with a condition is
///   narration, as `{ narrate: …, when: … }` in YAML.
///
/// A bare string right after speech reads like more speech, so it is an
/// error that suggests `narrate` (the lint of #111; YAML loads it with a
/// warning).
///
/// In a dialogue (`dialogue`), the line also ends at `{`, `=>`, `,` and a
/// `when` without `then`: the node's body, the
/// option's target, the next effect of `then` and the option's condition.
fn line_until(input: ParseStream, dialogue: bool) -> Result<Vec<Segment>> {
    let mut segments: Vec<Segment> = Vec::new();
    let end = |input: ParseStream| {
        input.peek(Token![;])
            || input.is_empty()
            || (dialogue
                && (input.peek(syn::token::Brace)
                    || input.peek(Token![=>])
                    || input.peek(Token![,])
                    || (peek_kw(input, "when") && !when_then(input))))
    };
    while !end(input) {
        if segments.is_empty()
            && input.peek(Ident::peek_any)
            && !peek_kw(input, "when")
            && !peek_kw(input, "narrate")
            && !peek_kw(input, "says")
            // Two words are enough here: `keeper says foo` is a segment
            // with a missing string, not a text pool.
            && !peek_word_then(input, "says")
        {
            return Err(input.error(
                "a line from a text pool is not in `world!` yet: it waits for the text core (R9, P4-01 #78)",
            ));
        }
        let at = input.span();
        let s = segment(input)?;
        if matches!(s.voice, SegmentVoice::Plain)
            && s.when.is_none()
            && segments
                .last()
                .is_some_and(|p| matches!(p.voice, SegmentVoice::Says(_)))
        {
            return Err(Error::new(
                at,
                "a bare string after `says` reads like more speech; write `narrate \"…\"` for narration",
            ));
        }
        segments.push(s);
    }
    if segments.is_empty() {
        return Err(input.error("the line needs a text or at least one segment"));
    }
    Ok(segments)
}

/// `when <cond> then`: the condition of a segment, not of an option. It
/// looks ahead up to `then`, `;`, `=>` or `{`.
fn when_then(input: ParseStream) -> bool {
    let fork = input.fork();
    if fork.call(Ident::parse_any).is_err() {
        return false;
    }
    let mut operand = true;
    while !fork.is_empty() {
        if !operand && peek_marker(&fork) {
            return true;
        }
        if fork.peek(Token![;]) || fork.peek(Token![=>]) || fork.peek(syn::token::Brace) {
            return false;
        }
        operand = wants_operand(&fork);
        if fork.parse::<proc_macro2::TokenTree>().is_err() {
            return false;
        }
    }
    false
}

/// The `then` of a segment condition: a segment follows it. The callers
/// ask only where the condition needs no operand: a `then` at the start,
/// after an operator, a `.`, `in`, `and`, `or` or `not` is an id (a room,
/// an enum value, a state, a property: `when in then then "…"`).
fn peek_marker(input: ParseStream) -> bool {
    let fork = input.fork();
    peek_kw(&fork, "then")
        && fork.call(Ident::parse_any).is_ok()
        && (fork.peek(LitStr)
            || peek_kw(&fork, "narrate")
            || peek_kw(&fork, "says")
            || peek_kw(&fork, "when")
            || peek_speaker(&fork))
}

/// The token here leaves the condition waiting for an operand.
fn wants_operand(input: ParseStream) -> bool {
    matches!(
        input.fork().parse::<proc_macro2::TokenTree>(),
        Ok(proc_macro2::TokenTree::Punct(_))
    ) || ["in", "and", "or", "not"].iter().any(|k| peek_kw(input, k))
}

fn segment(input: ParseStream) -> Result<Segment> {
    let start = input.cursor();
    let mut s = segment_body(input)?;
    s.tokens = since(start, input);
    Ok(s)
}

fn segment_body(input: ParseStream) -> Result<Segment> {
    let plain = |voice, text| Segment {
        voice,
        text,
        when: None,
        tokens: TokenStream::new(),
    };
    // `<speaker> says "…"` first: a speaker may have the id of a keyword of
    // segments (`narrate says "…"`). It needs the string right after `says`,
    // so a condition that starts with an id `says` (`when says then "…"`)
    // stays a condition.
    if peek_speaker(input) {
        let speaker = name(input)?;
        name(input)?;
        return Ok(plain(SegmentVoice::Says(Some(speaker)), text(input)?));
    }
    if peek_kw(input, "when") {
        keyword(input, "when")?;
        // A `then` where an operand goes is an id: `stop` runs once per token.
        let operand = std::cell::Cell::new(true);
        let cond = parse_until(input, |i| {
            let wanted = operand.replace(wants_operand(i));
            (!wanted && peek_marker(i)) || i.peek(Token![;])
        })?;
        keyword(input, "then").map_err(|e| {
            Error::new(
                e.span(),
                "expected `then` and the segment after the condition",
            )
        })?;
        if peek_kw(input, "when") {
            return Err(input.error("a segment has one condition"));
        }
        let mut s = segment_body(input)?;
        if matches!(s.voice, SegmentVoice::Plain) {
            s.voice = SegmentVoice::Narrate;
        }
        s.when = Some(cond);
        return Ok(s);
    }
    if input.peek(LitStr) {
        return Ok(plain(SegmentVoice::Plain, text(input)?));
    }
    if peek_kw(input, "narrate") {
        name(input)?;
        return Ok(plain(SegmentVoice::Narrate, text(input)?));
    }
    if peek_kw(input, "says") {
        name(input)?;
        return Ok(plain(SegmentVoice::Says(None), text(input)?));
    }
    // `<speaker> says` without its string: report the missing string.
    if peek_word_then(input, "says") {
        name(input)?;
        name(input)?;
        text(input)?;
    }
    Err(input.error(
        "expected a segment: a string, `narrate \"…\"`, `says \"…\"`, `<speaker> says \"…\"` or `when <cond> then <segment>`, or `;`",
    ))
}

/// `before <verb> <frag>`, `instead ..` or `after ..`.
fn frame(input: ParseStream) -> Result<Frame> {
    let word = name(input)?;
    let when = match word.value.as_str() {
        "before" => FrameWhen::Before,
        "instead" => FrameWhen::Instead,
        _ => FrameWhen::After,
    };
    let verb = name(input)?;
    Ok(Frame {
        when,
        verb,
        frag: frag(input)?,
    })
}

fn note(input: ParseStream, todo: bool) -> Result<Note> {
    name(input)?;
    let text = text(input)?;
    semi(input)?;
    Ok(Note { todo, text })
}

fn room(input: ParseStream) -> Result<Room> {
    keyword(input, "room")?;
    let id = name(input)?;
    let room_name = text(input)?;
    let parts = body(input, |input| {
        let part = match peek_word(input).as_deref() {
            Some("at") => {
                name(input)?;
                let content;
                bracketed!(content in input);
                let x = float(&content)?;
                content.parse::<Token![,]>()?;
                let y = float(&content)?;
                if !content.is_empty() {
                    return Err(content.error("a position has two numbers: `at [x, y]`"));
                }
                RoomPart::At(x, y)
            }
            Some("exit") => RoomPart::Exit(exit(input)?),
            Some("tag") => {
                name(input)?;
                RoomPart::Tag(word_or_text(input)?)
            }
            Some("region") => {
                name(input)?;
                RoomPart::Region(name(input)?)
            }
            Some("level") => {
                name(input)?;
                RoomPart::Level(int(input)?)
            }
            Some("note") => return note(input, false).map(RoomPart::Note),
            Some("todo") => return note(input, true).map(RoomPart::Note),
            Some("before" | "instead" | "after") => RoomPart::Frame(frame(input)?),
            Some(w) if !LAYERS.contains(&w) && !MODES.contains(&w) && w != "when" => {
                return Err(input.error(
                    "expected a text, `at`, `exit`, `tag`, `region`, `level`, `before`, `instead`, `after`, `note` or `todo`",
                ));
            }
            _ => RoomPart::Text(frag(input)?),
        };
        semi(input)?;
        Ok(part)
    })?;
    Ok(Room {
        replace: false,
        id,
        name: room_name,
        parts,
    })
}

const EXIT_FIELDS: &[&str] = &["door", "visible", "passable", "blocked", "traversal"];

fn exit(input: ParseStream) -> Result<Exit> {
    let start = input.cursor();
    keyword(input, "exit")?;
    let direction = if input.peek(LitStr) {
        let t = text(input)?;
        Name {
            value: t.value,
            span: t.span,
        }
    } else {
        name(input)?
    };
    let to = if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        Some(name(input)?)
    } else {
        None
    };
    let mut exit = Exit {
        direction,
        to,
        door: None,
        visible: None,
        passable: None,
        blocked: None,
        traversal: None,
        tokens: TokenStream::new(),
    };
    let stop = |i: ParseStream| i.peek(Token![;]) || starts_exit_field(i);
    while !input.peek(Token![;]) {
        let field = name(input)?;
        match field.value.as_str() {
            "door" => exit.door = Some(name(input)?),
            "visible" => exit.visible = Some(parse_until(input, stop)?),
            "passable" => exit.passable = Some(parse_until(input, stop)?),
            "blocked" => exit.blocked = Some(text(input)?),
            "traversal" => exit.traversal = Some(text(input)?),
            _ => {
                return Err(Error::new(
                    field.span,
                    "expected `door`, `visible`, `passable`, `blocked`, `traversal` or `;`",
                ));
            }
        }
    }
    if exit.to.is_none() && exit.blocked.is_none() {
        return Err(Error::new(
            exit.direction.span,
            "the exit needs `-> <room>`, or `blocked \"<text>\"` for a message exit",
        ));
    }
    exit.tokens = since(start, input);
    Ok(exit)
}

/// An exit field starts here: a field word followed by its value (an id,
/// a condition word or a string), not by `.`, `(` or an operator. So
/// `visible door.locked` reads `door.locked` as the condition.
fn starts_exit_field(input: ParseStream) -> bool {
    if !peek_word(input).is_some_and(|w| EXIT_FIELDS.contains(&w.as_str())) {
        return false;
    }
    let fork = input.fork();
    if fork.call(Ident::parse_any).is_err() {
        return false;
    }
    fork.peek(Ident::peek_any) || fork.peek(LitStr)
}

fn thing(input: ParseStream) -> Result<Thing> {
    keyword(input, "thing")?;
    let id = name(input)?;
    let thing_name = text(input)?;
    let location = if input.peek(syn::token::Brace) {
        None
    } else {
        Some(location(input)?)
    };
    let parts = body(input, |input| {
        let part = match peek_word(input).as_deref() {
            Some("aliases") => {
                name(input)?;
                ThingPart::Aliases(words(input)?)
            }
            Some("portable") => flag(input, ThingPart::Portable)?,
            Some("container") => flag(input, ThingPart::Container)?,
            Some("supporter") => flag(input, ThingPart::Supporter)?,
            Some("unlisted") => flag(input, ThingPart::Unlisted)?,
            Some("proper") => flag(input, ThingPart::Proper)?,
            Some("capacity") => {
                name(input)?;
                ThingPart::Capacity(int(input)?)
            }
            Some("key") => {
                name(input)?;
                ThingPart::Key(name(input)?)
            }
            Some("prop") => {
                name(input)?;
                let prop = name(input)?;
                input.parse::<Token![=]>()?;
                ThingPart::Prop(prop, value(input)?)
            }
            Some("examine") => {
                name(input)?;
                ThingPart::Examine(frag(input)?)
            }
            Some("presence") => {
                name(input)?;
                ThingPart::Presence(frag(input)?)
            }
            Some("also_in") => {
                name(input)?;
                ThingPart::AlsoIn(name(input)?)
            }
            Some("note") => return note(input, false).map(ThingPart::Note),
            Some("todo") => return note(input, true).map(ThingPart::Note),
            Some("before" | "instead" | "after") => ThingPart::Frame(frame(input)?),
            Some("name") => ThingPart::Name(name_stage(input)?),
            Some(w) if !LAYERS.contains(&w) && !MODES.contains(&w) && w != "when" => {
                return Err(input.error(
                    "expected a text, `aliases`, `portable`, `container`, `supporter`, `unlisted`, `proper`, `capacity`, `key`, `prop`, `examine`, `presence`, `also_in`, `name`, `before`, `instead`, `after`, `note` or `todo`",
                ));
            }
            _ => ThingPart::Examine(frag(input)?),
        };
        semi(input)?;
        Ok(part)
    })?;
    Ok(Thing {
        replace: false,
        id,
        name: thing_name,
        location,
        parts,
    })
}

/// `name [when <cond> =>] "text" [aliases [..]] [proper]`.
fn name_stage(input: ParseStream) -> Result<NameStage> {
    let start = input.cursor();
    name(input)?;
    let when = if peek_kw(input, "when") {
        keyword(input, "when")?;
        let c = parse_until(input, |i| i.peek(Token![=>]) || i.peek(Token![;]))?;
        input
            .parse::<Token![=>]>()
            .map_err(|e| Error::new(e.span(), "expected `=>` and the name after the condition"))?;
        Some(c)
    } else {
        None
    };
    let stage_text = text(input)?;
    let mut stage = NameStage {
        when,
        text: stage_text,
        aliases: Vec::new(),
        proper: false,
        tokens: TokenStream::new(),
    };
    loop {
        match peek_word(input).as_deref() {
            Some("aliases") => {
                name(input)?;
                stage.aliases.extend(words(input)?);
            }
            Some("proper") => {
                name(input)?;
                stage.proper = true;
            }
            _ => break,
        }
    }
    stage.tokens = since(start, input);
    Ok(stage)
}

fn flag(input: ParseStream, part: ThingPart) -> Result<ThingPart> {
    name(input)?;
    Ok(part)
}

/// `in <room>`, `inside <thing>`, `upon <thing>`, `carried` or `nowhere`.
fn location(input: ParseStream) -> Result<Location> {
    let start = input.cursor();
    let word = name(input)?;
    let kind = match word.value.as_str() {
        "in" => LocationKind::Room(name(input)?),
        "inside" => LocationKind::Inside(name(input)?),
        "upon" => LocationKind::Upon(name(input)?),
        "carried" => LocationKind::Carried,
        "nowhere" => LocationKind::Nowhere,
        _ => {
            return Err(Error::new(
                word.span,
                "expected a place: `in <room>`, `inside <thing>`, `upon <thing>`, `carried` or `nowhere`",
            ));
        }
    };
    Ok(Location {
        kind,
        tokens: since(start, input),
    })
}

fn event(input: ParseStream) -> Result<Event> {
    let start = input.cursor();
    let word = name(input)?;
    // `star`: the verb can be `*`, any verb (`verb(*, darkness)`).
    let args = |input: ParseStream, star: bool| -> Result<Vec<Name>> {
        let content;
        parenthesized!(content in input);
        let mut list = Vec::new();
        while !content.is_empty() {
            if star && list.is_empty() && content.peek(Token![*]) {
                let at = content.parse::<Token![*]>()?.span;
                list.push(Name {
                    value: loremaster_model::ANY_VERB.into(),
                    span: at,
                });
                if !content.is_empty() {
                    content.parse::<Token![,]>()?;
                }
                continue;
            }
            if content.peek(Token![*]) {
                return Err(
                    content.error("`*` (any verb) goes only first in `verb(..)` or `check(..)`")
                );
            }
            let n = name(&content)?;
            if content.peek(Token![:]) {
                return Err(Error::new(
                    n.span,
                    "named slots of the command record (`direct: <thing>`) are not in `world!` yet: they wait for D6 (phase 4)",
                ));
            }
            list.push(n);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(list)
    };
    let one = |input: ParseStream| -> Result<Name> {
        let content;
        parenthesized!(content in input);
        let n = name(&content)?;
        if !content.is_empty() {
            return Err(content.error("expected `)`"));
        }
        Ok(n)
    };
    let kind = match word.value.as_str() {
        "start" => EventKind::Start,
        "turn" => EventKind::Turn,
        "next" => EventKind::Next,
        "enter" => EventKind::Enter(one(input)?),
        "leave" => EventKind::Leave(one(input)?),
        "time" => {
            let content;
            parenthesized!(content in input);
            let time = text(&content)?;
            if !content.is_empty() {
                return Err(content.error("expected `)`: `time(\"HH:MM\")` takes one time"));
            }
            EventKind::Time(time)
        }
        "verb" => EventKind::Verb(args(input, true)?),
        "check" => EventKind::Check(args(input, true)?),
        "timer" => EventKind::Timer(one(input)?),
        "event" => EventKind::Event(one(input)?),
        "talk" => EventKind::Talk(args(input, false)?),
        _ => {
            return Err(Error::new(
                word.span,
                "expected an event: `start`, `turn`, `next`, `enter(room)`, `leave(room)`, `time(\"HH:MM\")`, `verb(..)`, `check(..)`, `timer(id)`, `event(id)` or `talk(dialogue, node)`",
            ));
        }
    };
    Ok(Event {
        kind,
        tokens: since(start, input),
    })
}

fn rule(input: ParseStream) -> Result<Rule> {
    keyword(input, "rule")?;
    let id = name(input)?;
    keyword(input, "on")?;
    let on = event(input)?;
    let mut rule = Rule {
        id,
        on,
        instead: false,
        once: false,
        free: false,
        priority: None,
        within: None,
        when: None,
        effects: Vec::new(),
    };
    loop {
        match peek_word(input).as_deref() {
            Some("instead") => {
                name(input)?;
                rule.instead = true;
            }
            Some("once") => {
                name(input)?;
                rule.once = true;
            }
            Some("free") => {
                name(input)?;
                rule.free = true;
            }
            Some("priority") => {
                name(input)?;
                rule.priority = Some(int(input)?);
            }
            Some("within") => {
                name(input)?;
                rule.within = Some(name(input)?);
            }
            _ => break,
        }
    }
    if peek_kw(input, "when") {
        keyword(input, "when")?;
        rule.when = Some(parse_until(input, |i| i.peek(syn::token::Brace))?);
    }
    if !input.peek(syn::token::Brace) {
        return Err(input.error(
            "expected `instead`, `once`, `free`, `priority`, `within`, `when` or the rule body `{ .. }`",
        ));
    }
    rule.effects = effects(input)?;
    own_rust(&mut rule.effects, Some(&rule.id.value))?;
    Ok(rule)
}

/// Give the `rust` block of a list of effects its owner, the rule. A rule
/// has one block at most. Without an owner (a timer), a block is an error.
fn own_rust(effects: &mut [Effect], owner: Option<&str>) -> Result<()> {
    let mut seen = false;
    for e in effects {
        if let EffectKind::Rust(block) = &mut e.kind {
            let Some(owner) = owner else {
                return Err(Error::new(
                    block.cx.span(),
                    "a `rust` block goes in a rule or a machine step",
                ));
            };
            if seen {
                return Err(Error::new(
                    block.cx.span(),
                    "a rule or machine step has one `rust` block at most; put the code into the first one",
                ));
            }
            seen = true;
            block.owner = owner.to_string();
        }
    }
    Ok(())
}

/// A `{ ... }` body of effects.
fn effects(input: ParseStream) -> Result<Vec<Effect>> {
    body(input, effect)
}

fn effect(input: ParseStream) -> Result<Effect> {
    let start = input.cursor();
    let kind = effect_body(input, false)?.kind;
    // A `rust` block ends with its `}`; a `;` after it is fine.
    if !matches!(kind, EffectKind::Rust(_)) || input.peek(Token![;]) {
        semi(input)?;
    }
    Ok(Effect {
        kind,
        tokens: since(start, input),
    })
}

/// Read `,` and go on with a list of `set`, `add` or `move` only when the
/// next entry starts with `entry`; else the comma belongs to the next effect
/// (`then set a = true, goto hall` in a dialogue).
fn more(input: ParseStream, entry: impl Fn(ParseStream) -> bool) -> Result<bool> {
    if !input.peek(Token![,]) {
        return Ok(false);
    }
    let fork = input.fork();
    fork.parse::<Token![,]>()?;
    if !entry(&fork) {
        return Ok(false);
    }
    input.parse::<Token![,]>()?;
    Ok(true)
}

/// An effect without its `;`. In a dialogue's `then`, a `line` ends at a
/// comma too.
fn effect_body(input: ParseStream, dialogue: bool) -> Result<Effect> {
    let start = input.cursor();
    let word = name(input)?;
    let kind = match word.value.as_str() {
        "line" => EffectKind::Line(line_until(input, dialogue)?),
        "set" => {
            let mut list = Vec::new();
            loop {
                let target = path(input)?;
                input.parse::<Token![=]>()?;
                list.push((target, value(input)?));
                if !more(input, |f| {
                    path(f).is_ok() && f.peek(Token![=]) && !f.peek(Token![==])
                })? {
                    break;
                }
            }
            EffectKind::Set(list)
        }
        "add" => {
            let mut list = Vec::new();
            loop {
                let target = path(input)?;
                let sign = if input.peek(Token![-=]) {
                    input.parse::<Token![-=]>()?;
                    -1
                } else {
                    input
                        .parse::<Token![+=]>()
                        .map_err(|e| Error::new(e.span(), "expected `+=` or `-=`"))?;
                    1
                };
                let at = input.span();
                let amount = int::<i64>(input)?
                    .checked_mul(sign)
                    .ok_or_else(|| Error::new(at, "the amount is out of range"))?;
                list.push((target, amount));
                if !more(input, |f| {
                    path(f).is_ok() && (f.peek(Token![+=]) || f.peek(Token![-=]))
                })? {
                    break;
                }
            }
            EffectKind::Add(list)
        }
        "calc" => {
            let target = path(input)?;
            input.parse::<Token![=]>()?;
            let mut sum = String::new();
            if input.peek(Token![-]) {
                input.parse::<Token![-]>()?;
                sum.push('-');
            }
            loop {
                if input.peek(LitInt) {
                    sum.push_str(input.parse::<LitInt>()?.base10_digits());
                } else {
                    sum.push_str(&path(input)?.value);
                }
                if input.peek(Token![+]) {
                    input.parse::<Token![+]>()?;
                    sum.push_str(" + ");
                } else if input.peek(Token![-]) {
                    input.parse::<Token![-]>()?;
                    sum.push_str(" - ");
                } else {
                    break;
                }
            }
            EffectKind::Calc(target, sum)
        }
        "move" => {
            let mut list = Vec::new();
            loop {
                let thing = name(input)?;
                input.parse::<Token![->]>()?;
                let target = name(input)?;
                let to = match target.value.as_str() {
                    "inside" => format!("in:{}", name(input)?.value),
                    "upon" => format!("on:{}", name(input)?.value),
                    _ => target.value,
                };
                list.push((thing, to));
                if !more(input, |f| name(f).is_ok() && f.peek(Token![->]))? {
                    break;
                }
            }
            EffectKind::Move(list)
        }
        "goto" => EffectKind::Goto(name(input)?),
        "end_combat" => EffectKind::EndCombat(name(input)?),
        "end" => {
            if input.peek(LitStr) {
                return Err(input.error(
                    "`end` names an ending entity: `end <id>;` with `ending <id> \"<title>\" { .. }`",
                ));
            }
            EffectKind::End(name(input)?)
        }
        "stop" => EffectKind::Stop,
        "emit" => EffectKind::Emit(name(input)?),
        "start" => {
            let timer = name(input)?;
            let turns = if peek_kw(input, "for") {
                name(input)?;
                Some(int(input)?)
            } else {
                None
            };
            EffectKind::Start(timer, turns)
        }
        "cancel" => EffectKind::Cancel(name(input)?),
        "call" => EffectKind::Call(name(input)?),
        "await" => EffectKind::Await(await_spec(input)?),
        "combat" => EffectKind::Combat(combat(input)?),
        "rust" if dialogue => {
            return Err(Error::new(
                word.span,
                "a `rust` block goes in a rule or a machine step, not in a dialogue",
            ));
        }
        "rust" => {
            input.parse::<Token![|]>()?;
            let cx: Ident = input.parse()?;
            input.parse::<Token![|]>()?;
            EffectKind::Rust(RustBlock {
                owner: String::new(),
                cx,
                body: input.parse()?,
            })
        }
        "say" => {
            return Err(Error::new(
                word.span,
                "the effect that shows a text is `line \"<text>\"`",
            ));
        }
        _ => {
            let last = if dialogue {
                " or `call`"
            } else {
                ", `await`, `combat`, `call` or `rust`"
            };
            return Err(Error::new(
                word.span,
                format!(
                    "expected an effect: `line`, `set`, `add`, `calc`, `move`, `goto`, `end`, `stop`, `emit`, `start`, `cancel`{last}"
                ),
            ));
        }
    };
    Ok(Effect {
        kind,
        tokens: since(start, input),
    })
}

/// The rest of `await <kind> .. [into <var>] [against <var>]`.
fn await_spec(input: ParseStream) -> Result<loremaster_model::AwaitSpec> {
    use loremaster_model::{AwaitOption, Id};
    let mut spec = loremaster_model::AwaitSpec::default();
    let kind = name(input)?;
    match kind.value.as_str() {
        "roll" => spec.roll = Some(text(input)?.value),
        "walk_to" => spec.walk_to = Some(Id::new(name(input)?.value)),
        "choice" => {
            let list;
            bracketed!(list in input);
            while !list.is_empty() {
                let id = Id::new(name(&list)?.value);
                spec.choice.push(if list.peek(LitStr) {
                    AwaitOption::Labeled {
                        id,
                        text: text(&list)?.value,
                    }
                } else {
                    AwaitOption::Id(id)
                });
                if !list.is_empty() {
                    list.parse::<Token![,]>()?;
                }
            }
        }
        "action" => {
            spec.action = Some(Id::new(name(input)?.value));
            if input.peek(syn::token::Brace) {
                let args;
                braced!(args in input);
                while !args.is_empty() {
                    let arg = name(&args)?;
                    args.parse::<Token![=]>()?;
                    let v = match value(&args)? {
                        Value::Bool(b) => loremaster_model::Value::Bool(b),
                        Value::Int(i) => loremaster_model::Value::Int(i),
                        Value::Text(t) => loremaster_model::Value::Text(t),
                    };
                    if spec.args.insert(arg.value.clone(), v).is_some() {
                        return Err(Error::new(
                            arg.span,
                            format!("the argument `{}` is given twice", arg.value),
                        ));
                    }
                    if !args.is_empty() {
                        args.parse::<Token![,]>()?;
                    }
                }
            }
            if peek_kw(input, "results") {
                name(input)?;
                spec.results = names(input)?
                    .into_iter()
                    .map(|n| Id::new(n.value))
                    .collect();
            }
        }
        _ => {
            return Err(Error::new(
                kind.span,
                "expected `roll`, `choice`, `walk_to` or `action` after `await`",
            ));
        }
    }
    while let Some(key @ ("into" | "against")) = peek_word(input).as_deref() {
        let at = name(input)?.span;
        let var = Some(path(input)?.value);
        let slot = if key == "into" {
            &mut spec.into
        } else {
            &mut spec.against
        };
        if std::mem::replace(slot, var).is_some() {
            return Err(Error::new(at, format!("`{key}` is given twice")));
        }
    }
    Ok(spec)
}

/// The rest of `combat [enemies] win <room> loss <room> [escape <room>] [together]`.
fn combat(input: ParseStream) -> Result<loremaster_model::CombatSpec> {
    use loremaster_model::Id;
    let at = input.span();
    let enemies = names(input)?
        .into_iter()
        .map(|n| Id::new(n.value))
        .collect();
    let (mut win, mut loss, mut escape, mut together) = (None, None, None, false);
    loop {
        match peek_word(input).as_deref() {
            Some("together") => {
                name(input)?;
                together = true;
            }
            Some(w @ ("win" | "loss" | "escape")) => {
                let at = name(input)?.span;
                let room = Some(Id::new(name(input)?.value));
                let slot = match w {
                    "win" => &mut win,
                    "loss" => &mut loss,
                    _ => &mut escape,
                };
                if std::mem::replace(slot, room).is_some() {
                    return Err(Error::new(at, format!("`{w}` is given twice")));
                }
            }
            _ => break,
        }
    }
    let (Some(win), Some(loss)) = (win, loss) else {
        return Err(Error::new(
            at,
            "`combat` needs `win <room>` and `loss <room>`",
        ));
    };
    Ok(loremaster_model::CombatSpec {
        enemies,
        win,
        loss,
        escape,
        together,
    })
}

fn machine(input: ParseStream) -> Result<Machine> {
    keyword(input, "machine")?;
    let id = name(input)?;
    keyword(input, "starts")?;
    let start = name(input)?;
    let mut states = Vec::new();
    let mut steps = Vec::new();
    let mut notes = Vec::new();
    let content;
    braced!(content in input);
    while !content.is_empty() {
        if peek_kw(&content, "note") || peek_kw(&content, "todo") {
            let todo = peek_kw(&content, "todo");
            notes.push(note(&content, todo)?);
            continue;
        }
        if peek_kw(&content, "state") {
            name(&content)?;
            let state = name(&content)?;
            let description = if content.peek(LitStr) {
                Some(text(&content)?)
            } else {
                None
            };
            semi(&content)?;
            states.push((state, description));
            continue;
        }
        let mut s = step(&content)?;
        // The rule of the step, as `Machine::step_rule` names it.
        own_rust(
            &mut s.effects,
            Some(&format!("{}#{}", id.value, steps.len() + 1)),
        )?;
        steps.push(s);
    }
    Ok(Machine {
        replace: false,
        id,
        start,
        states,
        steps,
        notes,
    })
}

fn step(input: ParseStream) -> Result<Step> {
    let start = input.cursor();
    let (from, from_list) = if input.peek(syn::token::Bracket) {
        (names(input)?, true)
    } else {
        (
            vec![name(input).map_err(|e| {
                Error::new(
                    e.span(),
                    "expected `state <name>;` or a step `<from> -> <to> on <event>`",
                )
            })?],
            false,
        )
    };
    input.parse::<Token![->]>()?;
    let to = name(input)?;
    keyword(input, "on")?;
    let on = event(input)?;
    let instead = peek_kw(input, "instead");
    if instead {
        name(input)?;
    }
    let when = if peek_kw(input, "when") {
        keyword(input, "when")?;
        Some(parse_until(input, |i| {
            i.peek(syn::token::Brace) || i.peek(Token![;])
        })?)
    } else {
        None
    };
    let mut effects_list = if input.peek(Token![;]) {
        input.parse::<Token![;]>()?;
        Vec::new()
    } else {
        effects(input)?
    };
    let (say, say_tokens) = match effects_list.first() {
        Some(Effect {
            kind: EffectKind::Line(segments),
            tokens,
        }) => {
            let say = (segments.clone(), tokens.clone());
            effects_list.remove(0);
            (Some(say.0), say.1)
        }
        _ => (None, TokenStream::new()),
    };
    Ok(Step {
        from,
        from_list,
        to,
        on,
        instead,
        when,
        say,
        say_tokens,
        effects: effects_list,
        tokens: since(start, input),
    })
}

fn timer(input: ParseStream) -> Result<Timer> {
    keyword(input, "timer")?;
    let id = name(input)?;
    keyword(input, "turns")?;
    let turns = int(input)?;
    let mut timer = Timer {
        replace: false,
        id,
        turns,
        running: false,
        repeat: false,
        runs_while: None,
        at: Vec::new(),
        notes: Vec::new(),
    };
    if input.peek(Token![;]) {
        input.parse::<Token![;]>()?;
        return Ok(timer);
    }
    let content;
    braced!(content in input);
    while !content.is_empty() {
        let field = name(&content)?;
        match field.value.as_str() {
            "running" => {
                timer.running = true;
                semi(&content)?;
            }
            "repeat" => {
                timer.repeat = true;
                semi(&content)?;
            }
            "while" => {
                timer.runs_while = Some(parse_until(&content, |i| i.peek(Token![;]))?);
                semi(&content)?;
            }
            "at" => {
                let n = int(&content)?;
                let mut list = effects(&content)?;
                own_rust(&mut list, None)?;
                timer.at.push((n, list));
            }
            "note" | "todo" => {
                timer.notes.push(Note {
                    todo: field.value == "todo",
                    text: text(&content)?,
                });
                semi(&content)?;
            }
            _ => {
                return Err(Error::new(
                    field.span,
                    "expected `running;`, `repeat;`, `while <cond>;`, `at <n> { .. }`, `note` or `todo`",
                ));
            }
        }
    }
    Ok(timer)
}

fn ending(input: ParseStream) -> Result<Ending> {
    keyword(input, "ending")?;
    let id = name(input)?;
    let title = text(input)?;
    let mut kind = None;
    let mut rank = None;
    let mut ending_text = None;
    let mut notes = Vec::new();
    let content;
    braced!(content in input);
    while !content.is_empty() {
        if content.peek(LitStr) {
            let t = text(&content)?;
            if ending_text.is_some() {
                return Err(Error::new(t.span, "an ending has one text"));
            }
            ending_text = Some(t);
            semi(&content)?;
            continue;
        }
        match peek_word(&content).as_deref() {
            Some("kind") => {
                name(&content)?;
                let k = name(&content)?;
                if !["win", "loss", "death", "neutral"].contains(&k.value.as_str()) {
                    return Err(Error::new(
                        k.span,
                        "expected `win`, `loss`, `death` or `neutral`",
                    ));
                }
                kind = Some(k);
            }
            Some("rank") => {
                name(&content)?;
                rank = Some(int(&content)?);
            }
            Some("note") => {
                notes.push(note(&content, false)?);
                continue;
            }
            Some("todo") => {
                notes.push(note(&content, true)?);
                continue;
            }
            _ => {
                return Err(content.error("expected the text, `kind`, `rank`, `note` or `todo`"));
            }
        }
        semi(&content)?;
    }
    let kind = kind.ok_or_else(|| {
        Error::new(
            id.span,
            "the ending needs `kind win;`, `loss`, `death` or `neutral`",
        )
    })?;
    Ok(Ending {
        replace: false,
        id,
        title,
        kind,
        rank,
        text: ending_text,
        notes,
    })
}

fn verb(input: ParseStream) -> Result<Verb> {
    keyword(input, "verb")?;
    let id = name(input)?;
    let label = text(input)?;
    let mut verb = Verb {
        replace: false,
        id,
        label,
        words: Vec::new(),
        slots: None,
        preposition: None,
        no_bar: false,
        default: None,
    };
    let content;
    braced!(content in input);
    while !content.is_empty() {
        let field = name(&content)?;
        match field.value.as_str() {
            "words" => verb.words.extend(words(&content)?),
            "slots" => {
                let s = name(&content)?;
                if !["none", "one", "two"].contains(&s.value.as_str()) {
                    return Err(Error::new(s.span, "expected `none`, `one` or `two`"));
                }
                verb.slots = Some(s);
            }
            "preposition" => verb.preposition = Some(word_or_text(&content)?),
            "no_bar" => verb.no_bar = true,
            "default" => verb.default = Some(text(&content)?),
            _ => {
                return Err(Error::new(
                    field.span,
                    "expected `words`, `slots`, `preposition`, `no_bar` or `default`",
                ));
            }
        }
        semi(&content)?;
    }
    Ok(verb)
}

/// The most repeats of one step: each repeat is a step in the expansion.
const MAX_REPEAT: u32 = 1000;

fn test(input: ParseStream) -> Result<Test> {
    keyword(input, "test")?;
    let id = name(input)?;
    let test_name = text(input)?;
    let mut test = Test {
        id,
        name: test_name,
        pending: None,
        seed: None,
        steps: Vec::new(),
        expect: Vec::new(),
    };
    let content;
    braced!(content in input);
    while !content.is_empty() {
        if peek_kw(&content, "pending") {
            name(&content)?;
            keyword(&content, "phase")?;
            let phase = int(&content)?;
            content.parse::<Token![,]>()?;
            keyword(&content, "needs")?;
            let needs = text(&content)?;
            semi(&content)?;
            test.pending = Some((phase, needs));
            continue;
        }
        if peek_kw(&content, "seed") {
            name(&content)?;
            test.seed = Some(int(&content)?);
            semi(&content)?;
            continue;
        }
        if peek_kw(&content, "expect") {
            name(&content)?;
            test.expect.extend(expectations(&content)?);
            semi(&content)?;
            continue;
        }
        let command = word_or_text(&content).map_err(|e| {
            Error::new(
                e.span(),
                "expected a command, `expect`, `seed` or `pending`",
            )
        })?;
        let times = if content.peek(Token![*]) {
            content.parse::<Token![*]>()?;
            let at = content.span();
            let n: u32 = int(&content)?;
            if n > MAX_REPEAT {
                return Err(Error::new(
                    at,
                    format!("`<command> * <n>` repeats a step at most {MAX_REPEAT} times"),
                ));
            }
            n
        } else {
            1
        };
        let mut checks = Vec::new();
        let mut rolls = Vec::new();
        while !content.peek(Token![;]) {
            match peek_word(&content).as_deref() {
                Some("rolls") => {
                    name(&content)?;
                    loop {
                        rolls.push(int(&content)?);
                        if !content.peek(Token![,]) {
                            break;
                        }
                        content.parse::<Token![,]>()?;
                    }
                }
                Some("sees") => {
                    name(&content)?;
                    checks.push(Expectation::Sees(texts(&content)?));
                }
                Some("not_sees") => {
                    name(&content)?;
                    checks.push(Expectation::NotSees(texts(&content)?));
                }
                Some("expect") => {
                    name(&content)?;
                    checks.extend(expectations(&content)?);
                }
                _ => {
                    return Err(content.error(
                        "expected `sees`, `not_sees`, `expect`, `rolls` or `;` after the command",
                    ));
                }
            }
        }
        semi(&content)?;
        if times == 0 || (times > 1 && (!checks.is_empty() || !rolls.is_empty())) {
            return Err(Error::new(
                command.span,
                "`<command> * <n>` repeats a step without checks, n at least 1",
            ));
        }
        test.steps.push(TestStep {
            command,
            times,
            checks,
            rolls,
        });
    }
    Ok(test)
}

/// `ended`, `not ended`, `ending <id>`, `room == <id>`, `sees ..`,
/// `not_sees ..` or `<fact> == <value>`, separated by commas.
fn expectations(input: ParseStream) -> Result<Vec<Expectation>> {
    let mut out = Vec::new();
    loop {
        let e = match peek_word(input).as_deref() {
            Some("ended") => {
                name(input)?;
                Expectation::Ended(true)
            }
            Some("not") => {
                name(input)?;
                keyword(input, "ended")?;
                Expectation::Ended(false)
            }
            Some("ending") => {
                name(input)?;
                Expectation::Ending(name(input)?)
            }
            Some("sees") => {
                name(input)?;
                Expectation::Sees(texts(input)?)
            }
            Some("not_sees") => {
                name(input)?;
                Expectation::NotSees(texts(input)?)
            }
            Some(_) => {
                let fact = path(input)?;
                input
                    .parse::<Token![==]>()
                    .map_err(|e| Error::new(e.span(), "expected `==` and the value"))?;
                if fact.value == "room" {
                    Expectation::Room(name(input)?)
                } else {
                    Expectation::Fact(fact, value(input)?)
                }
            }
            None => {
                return Err(input.error(
                    "expected `ended`, `not ended`, `ending <id>`, `room == <id>`, `sees`, `not_sees` or `<fact> == <value>`",
                ));
            }
        };
        out.push(e);
        if !input.peek(Token![,]) {
            return Ok(out);
        }
        input.parse::<Token![,]>()?;
    }
}

fn thread(input: ParseStream) -> Result<Thread> {
    keyword(input, "thread")?;
    let id = name(input)?;
    let thread_name = text(input)?;
    let mut thread = Thread {
        replace: false,
        id,
        name: thread_name,
        status: None,
        summary: None,
        tracks: Vec::new(),
        notes: Vec::new(),
    };
    let content;
    braced!(content in input);
    while !content.is_empty() {
        match peek_word(&content).as_deref() {
            Some("status") => {
                name(&content)?;
                let s = name(&content)?;
                if !["open", "resolved", "dropped"].contains(&s.value.as_str()) {
                    return Err(Error::new(
                        s.span,
                        "expected `open`, `resolved` or `dropped`",
                    ));
                }
                thread.status = Some(s);
            }
            Some("summary") => {
                name(&content)?;
                thread.summary = Some(text(&content)?);
            }
            Some("tracks") => {
                name(&content)?;
                thread.tracks.push(path(&content)?);
            }
            Some("note") => {
                thread.notes.push(note(&content, false)?);
                continue;
            }
            Some("todo") => {
                thread.notes.push(note(&content, true)?);
                continue;
            }
            _ => {
                return Err(
                    content.error("expected `status`, `summary`, `tracks`, `note` or `todo`")
                );
            }
        }
        semi(&content)?;
    }
    Ok(thread)
}

// --------------------------------------------------------------- dialogue

/// loremaster's payloads in dramaturge's token form of a dialogue: a
/// condition is a `when` of `world!`, a text is a line of segments, and an
/// effect is `then <effect>`.
struct Host;

impl ParseHost for Host {
    type Condition = Cond;
    type Text = DialogueLine;
    type Effect = Effect;

    fn condition(&mut self, tokens: TokenStream, _: proc_macro2::Span) -> Result<Cond> {
        (|input: ParseStream| parse_until(input, |_| false)).parse2(tokens)
    }

    fn text(&mut self, input: ParseStream) -> Result<DialogueLine> {
        Ok(DialogueLine {
            segments: line_until(input, true)?,
        })
    }

    fn peek_effect(&self, input: ParseStream) -> bool {
        peek_kw(input, "then") && !peek_word_then(input, "says")
    }

    fn effect(&mut self, input: ParseStream) -> Result<Effect> {
        // `then a, b`: dramaturge reads the effects after a comma without
        // their `then`.
        if peek_kw(input, "then") {
            name(input)?;
        }
        effect_body(input, true)
    }
}

/// `dialogue [<id>] with <thing> start [..] { .. }`, in dramaturge's token
/// form with loremaster's payloads.
fn dialogue(input: ParseStream) -> Result<Dialogue> {
    let fork = input.fork();
    name(&fork)?;
    let id = if peek_kw(&fork, "with") {
        None
    } else {
        Some(name(&fork)?)
    };
    let tree: DialogueTree = parse_dialogue(input, &mut Host)?;
    let id = id.unwrap_or_else(|| Name {
        value: tree.dialogue.with.clone(),
        span: tree.spans.get(&Place::With).copied().unwrap_or(tree.span),
    });
    Ok(Dialogue {
        replace: false,
        id,
        tree,
    })
}
