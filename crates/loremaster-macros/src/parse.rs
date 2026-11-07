//! The grammar of `world!` (DSL step L3, the core): story header, vars,
//! rooms, things, rules, machines, timers, endings, verbs and walkthrough
//! tests, plus `extern` ids of other packs. Read the crate docs for the
//! grammar by example.

use crate::ast::*;
use crate::cond::parse_until;
use proc_macro2::{Ident, TokenStream};
use syn::buffer::Cursor;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Error, LitFloat, LitInt, LitStr, Result, Token, braced, bracketed, parenthesized};

/// The items of a block, for the error on an unknown one.
const ITEMS: &str = "`extern`, `story`, `var`, `room`, `thing`, `rule`, `machine`, `timer`, `ending`, `verb` or `test`";

impl Parse for World {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(item(input)?);
        }
        Ok(World { items })
    }
}

fn item(input: ParseStream) -> Result<Item> {
    if input.peek(Token![extern]) {
        input.parse::<Token![extern]>()?;
        return externs(input).map(Item::Extern);
    }
    let word = peek_word(input).ok_or_else(|| input.error(format!("expected {ITEMS}")))?;
    Ok(match word.as_str() {
        "story" => Item::Story(story(input)?),
        "var" => Item::Var(var(input)?),
        "room" => Item::Room(room(input)?),
        "thing" => Item::Thing(thing(input)?),
        "rule" => Item::Rule(rule(input)?),
        "machine" => Item::Machine(machine(input)?),
        "timer" => Item::Timer(timer(input)?),
        "ending" => Item::Ending(ending(input)?),
        "verb" => Item::Verb(verb(input)?),
        "test" => Item::Test(test(input)?),
        "texts" => {
            return Err(unsupported(
                input,
                "text pools (`texts`) are not in `world!` yet: they wait for the text core (R9, P4-01 #78)",
            ));
        }
        "dialogue" => {
            return Err(unsupported(
                input,
                "dialogues are not in `world!` yet: they come with dramaturge's `dialogue!` (R12, DSL step L4, P4-08 #85)",
            ));
        }
        "agenda" => {
            return Err(unsupported(
                input,
                "agendas are not in `world!` yet: they wait for actors and agendas (D6, P4-02 #79)",
            ));
        }
        "thread" => {
            return Err(unsupported(
                input,
                "threads are not in `world!` yet (DSL step L4, P4-08 #85); write them with the builder",
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
    let value: f32 = if input.peek(LitFloat) {
        input.parse::<LitFloat>()?.base10_parse()?
    } else {
        input.parse::<LitInt>()?.base10_parse()?
    };
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
            "thing" => ExternKind::Thing,
            "var" => {
                let ty = if input.peek(Token![:]) {
                    input.parse::<Token![:]>()?;
                    var_type(input)?
                } else {
                    VarType::Bool
                };
                ExternKind::Var(ty)
            }
            "machine" => ExternKind::Machine(if input.peek(syn::token::Bracket) {
                names(input)?
            } else {
                Vec::new()
            }),
            "timer" => ExternKind::Timer,
            "ending" => ExternKind::Ending,
            "verb" => ExternKind::Verb,
            "behavior" => ExternKind::Behavior,
            "texts" => {
                return Err(Error::new(
                    kind.span,
                    "text pools (`texts`) are not in `world!` yet: they wait for the text core (R9, P4-01 #78)",
                ));
            }
            _ => {
                return Err(Error::new(
                    kind.span,
                    "expected `room`, `thing`, `var`, `machine`, `timer`, `ending`, `verb` or `behavior`",
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
        clock_start: None,
        minutes_per_turn: None,
        phases: None,
        tokens,
    };
    let content;
    braced!(content in input);
    while !content.is_empty() {
        let field = name(&content)?;
        match field.value.as_str() {
            "clock" => clock(&content, &mut story)?,
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
                    _ => {
                        return Err(Error::new(
                            field.span,
                            "expected `author`, `start`, `seed`, `intro`, `death_prompt` or `clock`",
                        ));
                    }
                }
            }
        }
        if !content.is_empty() {
            content.parse::<Token![,]>()?;
        }
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
    Ok(Var { id, ty, initial })
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
    Ok(Frag {
        layer,
        mode,
        when,
        texts,
        variants,
        tokens: since(start, input),
    })
}

/// The second token is the word `kw`: `keeper says`.
fn peek_word_then(input: ParseStream, kw: &str) -> bool {
    let fork = input.fork();
    fork.call(Ident::parse_any).is_ok() && peek_kw(&fork, kw)
}

fn speech(input: ParseStream) -> Error {
    input.error(
        "speech segments (`says`, `narrate`) are not in `world!` yet (P3-15 #111); write the line as one string, for example a raw string r#\"…\"#",
    )
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
            Some(w) if !LAYERS.contains(&w) && !MODES.contains(&w) && w != "when" => {
                return Err(input.error(
                    "expected a text, `at`, `exit`, `tag`, `region`, `level`, `note` or `todo`",
                ));
            }
            _ => RoomPart::Text(frag(input)?),
        };
        semi(input)?;
        Ok(part)
    })?;
    Ok(Room {
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
    let stop = |i: ParseStream| {
        i.peek(Token![;]) || peek_word(i).is_some_and(|w| EXIT_FIELDS.contains(&w.as_str()))
    };
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
            Some(w) if !LAYERS.contains(&w) && !MODES.contains(&w) && w != "when" => {
                return Err(input.error(
                    "expected a text, `aliases`, `portable`, `container`, `supporter`, `unlisted`, `proper`, `capacity`, `key`, `prop`, `examine`, `presence`, `also_in`, `note` or `todo`",
                ));
            }
            _ => ThingPart::Examine(frag(input)?),
        };
        semi(input)?;
        Ok(part)
    })?;
    Ok(Thing {
        id,
        name: thing_name,
        location,
        parts,
    })
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
    let args = |input: ParseStream| -> Result<Vec<Name>> {
        let content;
        parenthesized!(content in input);
        let mut list = Vec::new();
        while !content.is_empty() {
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
        "enter" => EventKind::Enter(one(input)?),
        "leave" => EventKind::Leave(one(input)?),
        "time" => {
            let content;
            parenthesized!(content in input);
            EventKind::Time(text(&content)?)
        }
        "verb" => EventKind::Verb(args(input)?),
        "check" => EventKind::Check(args(input)?),
        "timer" => EventKind::Timer(one(input)?),
        "event" => EventKind::Event(one(input)?),
        "talk" => EventKind::Talk(args(input)?),
        _ => {
            return Err(Error::new(
                word.span,
                "expected an event: `start`, `turn`, `enter(room)`, `leave(room)`, `time(\"HH:MM\")`, `verb(..)`, `check(..)`, `timer(id)`, `event(id)` or `talk(dialogue, node)`",
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
            "expected `instead`, `once`, `priority`, `within`, `when` or the rule body `{ .. }`",
        ));
    }
    rule.effects = effects(input)?;
    Ok(rule)
}

/// A `{ ... }` body of effects.
fn effects(input: ParseStream) -> Result<Vec<Effect>> {
    body(input, effect)
}

fn effect(input: ParseStream) -> Result<Effect> {
    let start = input.cursor();
    let word = name(input)?;
    let kind = match word.value.as_str() {
        "line" => {
            if !input.peek(LitStr) && input.peek(Ident::peek_any) {
                return Err(input.error(
                    "a line from a text pool is not in `world!` yet: it waits for the text core (R9, P4-01 #78)",
                ));
            }
            let t = text(input)?;
            if !input.peek(Token![;]) {
                return Err(speech(input));
            }
            EffectKind::Line(t)
        }
        "set" => {
            let mut list = Vec::new();
            loop {
                let target = path(input)?;
                input.parse::<Token![=]>()?;
                list.push((target, value(input)?));
                if !input.peek(Token![,]) {
                    break;
                }
                input.parse::<Token![,]>()?;
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
                list.push((target, sign * int::<i64>(input)?));
                if !input.peek(Token![,]) {
                    break;
                }
                input.parse::<Token![,]>()?;
            }
            EffectKind::Add(list)
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
                if !input.peek(Token![,]) {
                    break;
                }
                input.parse::<Token![,]>()?;
            }
            EffectKind::Move(list)
        }
        "goto" => EffectKind::Goto(name(input)?),
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
        "say" => {
            return Err(Error::new(
                word.span,
                "the effect that shows a text is `line \"<text>\"`",
            ));
        }
        _ => {
            return Err(Error::new(
                word.span,
                "expected an effect: `line`, `set`, `add`, `move`, `goto`, `end`, `stop`, `emit`, `start`, `cancel` or `call`",
            ));
        }
    };
    semi(input)?;
    Ok(Effect {
        kind,
        tokens: since(start, input),
    })
}

fn machine(input: ParseStream) -> Result<Machine> {
    keyword(input, "machine")?;
    let id = name(input)?;
    keyword(input, "starts")?;
    let start = name(input)?;
    let mut states = Vec::new();
    let mut steps = Vec::new();
    let content;
    braced!(content in input);
    while !content.is_empty() {
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
        steps.push(step(&content)?);
    }
    Ok(Machine {
        id,
        start,
        states,
        steps,
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
    let say = match effects_list.first() {
        Some(Effect {
            kind: EffectKind::Line(t),
            ..
        }) => {
            let t = t.clone();
            effects_list.remove(0);
            Some(t)
        }
        _ => None,
    };
    Ok(Step {
        from,
        from_list,
        to,
        on,
        instead,
        when,
        say,
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
        id,
        turns,
        running: false,
        repeat: false,
        runs_while: None,
        at: Vec::new(),
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
                timer.at.push((n, effects(&content)?));
            }
            _ => {
                return Err(Error::new(
                    field.span,
                    "expected `running;`, `repeat;`, `while <cond>;` or `at <n> { .. }`",
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

fn test(input: ParseStream) -> Result<Test> {
    keyword(input, "test")?;
    let id = name(input)?;
    let test_name = text(input)?;
    let mut test = Test {
        id,
        name: test_name,
        pending: None,
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
        if peek_kw(&content, "expect") {
            name(&content)?;
            test.expect.extend(expectations(&content)?);
            semi(&content)?;
            continue;
        }
        let command = word_or_text(&content)
            .map_err(|e| Error::new(e.span(), "expected a command, `expect` or `pending`"))?;
        let times = if content.peek(Token![*]) {
            content.parse::<Token![*]>()?;
            int(&content)?
        } else {
            1
        };
        let mut checks = Vec::new();
        while !content.peek(Token![;]) {
            match peek_word(&content).as_deref() {
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
                    return Err(content
                        .error("expected `sees`, `not_sees`, `expect` or `;` after the command"));
                }
            }
        }
        semi(&content)?;
        if times == 0 || (times > 1 && !checks.is_empty()) {
            return Err(Error::new(
                command.span,
                "`<command> * <n>` repeats a step without checks, n at least 1",
            ));
        }
        test.steps.push(TestStep {
            command,
            times,
            checks,
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
