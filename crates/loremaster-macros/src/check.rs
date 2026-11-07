//! The checks at compile time. The macro builds the pack of the block with
//! the builder, as the expansion will at run time, and runs
//! [`check_packs`] on it, together with a pack of stubs for the `extern`
//! ids. Each error becomes a `compile_error!` at the tokens of the entity
//! and the field that it names.
//!
//! The origin of each entity in this pack has the entity key as its file
//! (`room:kitchen`). So a diagnostic names its entity by its file, and its
//! `at` (`exits[0].to`) names the field; [`Spans`] finds the tokens of both.

use crate::ast::*;
use crate::emit::camel;
use loremaster_dsl::{
    BuildPack, Effects, EndingBuilder, FragmentExt, HasNotes, RoomBuilder, ThingBuilder, frag, on,
    variants,
};
use loremaster_model::{
    EndingKind, EventPattern, Fragment, FragmentMode, Id, Layer, TimeOfDay, VarDef,
    VarType as ModelVarType, VerbSlots,
};
use loremaster_project::{Origin, Pack, Severity, check_packs};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::BTreeSet;
use syn::Error;

/// The file of the stubs for `extern` ids. Their diagnostics do not count.
const EXTERN: &str = "<extern>";

/// The checks before the model: what the builder would refuse with a panic,
/// and what the schema checks cannot see in one pack.
pub fn validate(world: &World) -> Result<(), Error> {
    let mut errors: Vec<Error> = Vec::new();
    let mut story_seen = false;
    let mut defined: BTreeSet<(String, String)> = BTreeSet::new();
    let mut tests = BTreeSet::new();
    let mut times = Vec::new();
    for item in &world.items {
        match item {
            Item::Story(s) => {
                if story_seen {
                    errors.push(Error::new(
                        s.title.span,
                        "a pack has one story header; this block has a second one",
                    ));
                }
                story_seen = true;
                times.extend(s.clock_start.iter());
                times.extend(s.phases.iter().flatten().map(|(_, t)| t));
            }
            Item::Room(r) => {
                defined.insert(("room".into(), r.id.value.clone()));
            }
            Item::Thing(t) => {
                defined.insert(("thing".into(), t.id.value.clone()));
            }
            Item::Var(v) => {
                defined.insert(("var".into(), v.id.value.clone()));
            }
            Item::Machine(m) => {
                defined.insert(("machine".into(), m.id.value.clone()));
                times.extend(m.steps.iter().filter_map(|s| time_of(&s.on)));
            }
            Item::Timer(t) => {
                defined.insert(("timer".into(), t.id.value.clone()));
            }
            Item::Ending(e) => {
                defined.insert(("ending".into(), e.id.value.clone()));
            }
            Item::Verb(v) => {
                defined.insert(("verb".into(), v.id.value.clone()));
            }
            Item::Rule(r) => times.extend(time_of(&r.on)),
            Item::Test(t) => {
                if !tests.insert(t.id.value.clone()) {
                    errors.push(Error::new(
                        t.id.span,
                        format!("the test `{}` is there twice", t.id.value),
                    ));
                }
            }
            Item::Extern(_) => {}
        }
    }
    for item in &world.items {
        match item {
            Item::Var(v) if !story_seen => errors.push(Error::new(
                v.id.span,
                format!(
                    "a `var` belongs to the story header, and this block has no `story`; for a variable of another pack, write `extern {{ var {}; }}`",
                    v.id.value
                ),
            )),
            Item::Extern(list) => {
                for e in list {
                    let kind = extern_kind(&e.kind);
                    if defined.contains(&(kind.to_string(), e.id.value.clone())) {
                        errors.push(Error::new(
                            e.id.span,
                            format!(
                                "the {kind} `{}` is defined in this block, so it is not extern",
                                e.id.value
                            ),
                        ));
                    }
                    if let ExternKind::Var(VarType::Enum(values)) = &e.kind
                        && values.is_empty()
                    {
                        errors.push(Error::new(e.id.span, "an enum needs at least one value"));
                    }
                }
            }
            _ => {}
        }
    }
    for t in times {
        if let Err(e) = t.value.parse::<TimeOfDay>() {
            errors.push(Error::new(t.span, format!("not a time of day: {e}")));
        }
    }
    combine(errors)
}

fn time_of(e: &Event) -> Option<&Text> {
    match &e.kind {
        EventKind::Time(t) => Some(t),
        _ => None,
    }
}

fn extern_kind(k: &ExternKind) -> &'static str {
    match k {
        ExternKind::Room => "room",
        ExternKind::Thing => "thing",
        ExternKind::Var(_) => "var",
        ExternKind::Machine(_) => "machine",
        ExternKind::Timer => "timer",
        ExternKind::Ending => "ending",
        ExternKind::Verb => "verb",
        ExternKind::Behavior => "behavior",
    }
}

fn combine(errors: Vec<Error>) -> Result<(), Error> {
    let mut iter = errors.into_iter();
    match iter.next() {
        None => Ok(()),
        Some(mut first) => {
            for e in iter {
                first.combine(e);
            }
            Err(first)
        }
    }
}

/// Run the schema checks on the pack of the block. Warnings do not stop
/// the build.
pub fn check(world: &World) -> Result<(), Error> {
    let mut spans = Spans::default();
    let mut pack = build(world, &mut spans);
    for (key, origin) in pack.origins.iter_mut() {
        *origin = Origin::Rust {
            file: key.clone(),
            line: 0,
            column: 0,
            pack: "world".into(),
        };
    }
    let stubs = stubs(world, &mut pack, &mut spans);
    let checked = check_packs(vec![stubs, pack]);
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    for d in &checked.diagnostics {
        if d.severity != Severity::Error || d.file == EXTERN {
            continue;
        }
        if !seen.insert((d.file.clone(), d.at.clone(), d.message.clone())) {
            continue;
        }
        let message = rust_hint(&d.message);
        errors.push(match spans.find(&d.file, &d.at) {
            Some(tokens) => Error::new_spanned(tokens, message),
            None => Error::new(Span::call_site(), message),
        });
    }
    combine(errors)
}

/// The checks write their hints for YAML files. An unknown id in a block
/// gets the hint for Rust: define it, or declare it as an id of another
/// pack.
fn rust_hint(message: &str) -> String {
    let message = message.split(": add it as ").next().unwrap_or(message);
    for kind in ["room", "thing", "timer", "ending", "verb"] {
        if let Some(rest) = message.strip_prefix(&format!("unknown {kind} `"))
            && let Some(id) = rest.strip_suffix('`')
        {
            return format!(
                "{message}; for an id of another pack, write `extern {{ {kind} {id}; }}`"
            );
        }
    }
    message.to_string()
}

/// The tokens of each entity and of its fields, by entity key and `at`.
#[derive(Default)]
struct Spans {
    list: Vec<(String, String, TokenStream)>,
}

impl Spans {
    fn add(&mut self, key: &str, at: impl Into<String>, tokens: impl ToTokens) {
        self.list
            .push((key.to_string(), at.into(), tokens.into_token_stream()));
    }

    /// The tokens of the field `at` of the entity, or of the entity: the
    /// longest recorded `at` that starts the diagnostic's `at`.
    fn find(&self, key: &str, at: &str) -> Option<&TokenStream> {
        self.list
            .iter()
            .filter(|(k, a, _)| {
                k == key
                    && (a.is_empty()
                        || at == a
                        || (at.starts_with(a.as_str())
                            && matches!(at.as_bytes().get(a.len()), Some(b'.' | b'['))))
            })
            .max_by_key(|(_, a, _)| a.len())
            .map(|(_, _, t)| t)
    }
}

fn name_tokens(n: &Name) -> TokenStream {
    proc_macro2::Ident::new("x", n.span).into_token_stream()
}

fn text_tokens(t: &Text) -> TokenStream {
    syn::LitStr::new(&t.value, t.span).into_token_stream()
}

/// The stubs for the `extern` ids: a pack in front of the block's pack. An
/// extern variable goes into the header of the block, when it has one.
fn stubs(world: &World, pack: &mut Pack, spans: &mut Spans) -> Pack {
    let mut stub = Pack::new("extern");
    let mut vars = Vec::new();
    for item in &world.items {
        let Item::Extern(list) = item else { continue };
        for e in list {
            let id = e.id.value.as_str();
            match &e.kind {
                ExternKind::Room => {
                    stub.room(id, id);
                }
                ExternKind::Thing => {
                    // Any holder: the block may put a thing inside or upon it.
                    stub.thing(id, id).container().supporter();
                }
                ExternKind::Var(ty) => {
                    let (ty, initial) = match ty {
                        VarType::Bool => (ModelVarType::Bool, loremaster_model::Value::Bool(false)),
                        VarType::Int => (ModelVarType::Int, loremaster_model::Value::Int(0)),
                        VarType::Enum(values) => (
                            ModelVarType::Enum(values.iter().map(|v| v.value.clone()).collect()),
                            loremaster_model::Value::Text(
                                values.first().map(|v| v.value.clone()).unwrap_or_default(),
                            ),
                        ),
                    };
                    vars.push(VarDef {
                        id: Id::new(id),
                        ty,
                        initial,
                    });
                    spans.add("story", format!("vars.{id}"), e.tokens.clone());
                }
                ExternKind::Machine(states) => {
                    let start = states
                        .first()
                        .map(|s| s.value.clone())
                        .unwrap_or_else(|| "unknown".into());
                    let mut m = stub.machine(id, start);
                    for s in states {
                        m = m.state(s.value.as_str());
                    }
                }
                ExternKind::Timer => {
                    stub.timer(id, 1);
                }
                ExternKind::Ending => {
                    stub.ending(id, EndingKind::Neutral, id);
                }
                ExternKind::Verb => {
                    stub.verb(id, id);
                }
                ExternKind::Behavior => {
                    stub.behavior(id);
                }
            }
        }
    }
    match &mut pack.header {
        Some(header) => header.vars.extend(vars),
        None => {
            stub.story("extern", "extern");
            if let Some(header) = &mut stub.header {
                header.vars.extend(vars);
            }
        }
    }
    for origin in stub.origins.values_mut() {
        *origin = Origin::Rust {
            file: EXTERN.into(),
            line: 0,
            column: 0,
            pack: "extern".into(),
        };
    }
    stub
}

/// The pack of the block, as the expansion makes it (crate::emit).
fn build(world: &World, spans: &mut Spans) -> Pack {
    let mut p = Pack::new("world");
    let vars: Vec<&Var> = world
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Var(v) => Some(v),
            _ => None,
        })
        .collect();
    for item in &world.items {
        match item {
            Item::Extern(_) | Item::Var(_) => {}
            Item::Story(s) => story(&mut p, s, &vars, spans),
            Item::Room(r) => room(&mut p, r, spans),
            Item::Thing(t) => thing(&mut p, t, spans),
            Item::Rule(r) => rule(&mut p, r, spans),
            Item::Machine(m) => machine(&mut p, m, spans),
            Item::Timer(t) => timer(&mut p, t, spans),
            Item::Ending(e) => ending(&mut p, e, spans),
            Item::Verb(v) => {
                let mut b = p.verb(v.id.value.as_str(), v.label.value.as_str());
                b = b.words(v.words.iter().map(|w| w.value.as_str()));
                if let Some(s) = &v.slots {
                    b = b.slots(match s.value.as_str() {
                        "none" => VerbSlots::None,
                        "two" => VerbSlots::Two,
                        _ => VerbSlots::One,
                    });
                }
                if let Some(prep) = &v.preposition {
                    b = b.preposition(prep.value.as_str());
                }
                if v.no_bar {
                    b = b.parser_only();
                }
                if let Some(d) = &v.default {
                    b.default_response(d.value.as_str());
                }
                spans.add(&format!("verb:{}", v.id.value), "", name_tokens(&v.id));
            }
            // Tests have no checks of the schema.
            Item::Test(_) => {}
        }
    }
    p
}

fn value(v: &Value) -> loremaster_model::Value {
    match v {
        Value::Bool(b) => loremaster_model::Value::Bool(*b),
        Value::Int(i) => loremaster_model::Value::Int(*i),
        Value::Text(t) => loremaster_model::Value::Text(t.clone()),
    }
}

fn fragment(f: &Frag) -> Fragment {
    let mut out = if f.variants {
        variants(f.texts.iter().map(|t| t.value.as_str()))
    } else {
        frag(f.texts[0].value.as_str())
    };
    if let Some(l) = &f.layer {
        out = out.layer(match l.value.as_str() {
            "ambient" => Layer::Ambient,
            "event" => Layer::Event,
            "objects" => Layer::Objects,
            "characters" => Layer::Characters,
            "exits" => Layer::Exits,
            _ => Layer::Base,
        });
    }
    if let Some(m) = &f.mode {
        out = out.mode(match camel(&m.value).as_str() {
            "FirstVisit" => FragmentMode::FirstVisit,
            "Once" => FragmentMode::Once,
            "Cycle" => FragmentMode::Cycle,
            "Shuffle" => FragmentMode::Shuffle,
            "Sticky" => FragmentMode::Sticky,
            _ => FragmentMode::Always,
        });
    }
    if let Some(w) = &f.when {
        out = out.when(w.source.as_str());
    }
    out
}

fn story(p: &mut Pack, s: &Story, vars: &[&Var], spans: &mut Spans) {
    let start = s.start.as_ref().expect("parse requires start");
    let mut b = p.story(s.title.value.as_str(), start.value.as_str());
    if let Some(a) = &s.author {
        b = b.author(a.value.as_str());
    }
    if let Some(seed) = s.seed {
        b = b.seed(seed);
    }
    for (i, f) in s.intro.iter().enumerate() {
        b = b.intro(fragment(f));
        spans.add("story", format!("intro[{i}]"), f.tokens.clone());
    }
    if s.death_prompt == Some(false) {
        b = b.no_death_prompt();
    }
    if let Some(t) = &s.clock_start {
        b = b.clock_start(t.value.as_str());
    }
    if let Some(m) = s.minutes_per_turn {
        b = b.minutes_per_turn(m);
    }
    if let Some(phases) = &s.phases {
        b = b.phases(
            phases
                .iter()
                .map(|(n, t)| (n.value.clone(), t.value.as_str())),
        );
    }
    for v in vars {
        let id = v.id.value.as_str();
        b = match (&v.ty, &v.initial) {
            (VarType::Enum(values), Value::Text(initial)) => b.var_enum(
                id,
                values.iter().map(|v| v.value.as_str()),
                initial.as_str(),
            ),
            (_, Value::Bool(x)) => b.var(id, *x),
            (_, Value::Int(x)) => b.var(id, *x),
            (_, Value::Text(_)) => b,
        };
        spans.add("story", format!("vars.{id}"), name_tokens(&v.id));
    }
    spans.add("story", "", s.tokens.clone());
    spans.add("story", "start", name_tokens(start));
}

fn room(p: &mut Pack, r: &Room, spans: &mut Spans) {
    let key = format!("room:{}", r.id.value);
    let mut b: RoomBuilder = p.room(r.id.value.as_str(), r.name.value.as_str());
    let (mut texts, mut exits) = (0, 0);
    for part in &r.parts {
        b = match part {
            RoomPart::At(x, y) => b.at(*x, *y),
            RoomPart::Text(f) => {
                spans.add(&key, format!("description[{texts}]"), f.tokens.clone());
                texts += 1;
                b.text(fragment(f))
            }
            RoomPart::Exit(e) => {
                spans.add(&key, format!("exits[{exits}]"), e.tokens.clone());
                exits += 1;
                let dir = e.direction.value.as_str();
                let mut b = match &e.to {
                    Some(to) => b.exit(dir, to.value.as_str()),
                    None => b.exit_blocked(
                        dir,
                        e.blocked.as_ref().map(|t| t.value.as_str()).unwrap_or(""),
                    ),
                };
                if let Some(d) = &e.door {
                    b = b.door(d.value.as_str());
                }
                if let Some(c) = &e.visible {
                    b = b.visible(c.source.as_str());
                }
                if let Some(c) = &e.passable {
                    b = b.passable(c.source.as_str());
                }
                if e.to.is_some()
                    && let Some(t) = &e.blocked
                {
                    b = b.blocked(t.value.as_str());
                }
                if let Some(t) = &e.traversal {
                    b = b.traversal(t.value.as_str());
                }
                b
            }
            RoomPart::Tag(t) => b.tag(t.value.as_str()),
            RoomPart::Region(n) => b.region(n.value.as_str()),
            RoomPart::Level(l) => b.level(*l),
            RoomPart::Note(n) if n.todo => b.todo(n.text.value.as_str()),
            RoomPart::Note(n) => b.note(n.text.value.as_str()),
        };
    }
    spans.add(&key, "", name_tokens(&r.id));
}

fn thing(p: &mut Pack, t: &Thing, spans: &mut Spans) {
    let key = format!("thing:{}", t.id.value);
    let mut b: ThingBuilder = p.thing(t.id.value.as_str(), t.name.value.as_str());
    if let Some(loc) = &t.location {
        spans.add(&key, "location", loc.tokens.clone());
        b = match &loc.kind {
            LocationKind::Room(r) => b.in_room(r.value.as_str()),
            LocationKind::Inside(h) => b.in_thing(h.value.as_str()),
            LocationKind::Upon(h) => b.on_thing(h.value.as_str()),
            LocationKind::Carried => b.carried(),
            LocationKind::Nowhere => b.nowhere(),
        };
    }
    let (mut texts, mut presence) = (0, 0);
    for part in &t.parts {
        b = match part {
            ThingPart::Aliases(list) => b.aliases(list.iter().map(|a| a.value.as_str())),
            ThingPart::Portable => b.portable(),
            ThingPart::Container => b.container(),
            ThingPart::Supporter => b.supporter(),
            ThingPart::Unlisted => b.unlisted(),
            ThingPart::Proper => b.proper(),
            ThingPart::Capacity(c) => b.capacity(*c),
            ThingPart::Key(k) => {
                spans.add(&key, "key", name_tokens(k));
                b.key(k.value.as_str())
            }
            ThingPart::Prop(n, v) => b.prop(n.value.as_str(), value(v)),
            ThingPart::Examine(f) => {
                spans.add(&key, format!("description[{texts}]"), f.tokens.clone());
                texts += 1;
                b.text(fragment(f))
            }
            ThingPart::Presence(f) => {
                spans.add(&key, format!("presence[{presence}]"), f.tokens.clone());
                presence += 1;
                b.presence(fragment(f))
            }
            ThingPart::AlsoIn(n) => b.also_in(n.value.as_str()),
            ThingPart::Note(n) if n.todo => b.todo(n.text.value.as_str()),
            ThingPart::Note(n) => b.note(n.text.value.as_str()),
        };
    }
    spans.add(&key, "", name_tokens(&t.id));
}

fn event(e: &Event) -> EventPattern {
    let ids = |list: &[Name]| list.iter().map(|n| n.value.clone()).collect::<Vec<_>>();
    match &e.kind {
        EventKind::Start => on::start(),
        EventKind::Turn => on::turn(),
        EventKind::Enter(r) => on::enter(r.value.as_str()),
        EventKind::Leave(r) => on::leave(r.value.as_str()),
        EventKind::Time(t) => on::time(t.value.as_str()),
        EventKind::Verb(v) => on::verb(ids(v)),
        EventKind::Check(v) => on::check(ids(v)),
        EventKind::Timer(t) => on::timer(t.value.as_str()),
        EventKind::Event(t) => on::event(t.value.as_str()),
        EventKind::Talk(v) => on::talk(ids(v)),
    }
}

/// Apply the effects in order. `then` is the `at` of the list, for the
/// spans of the effects: the builder joins a `set` into the one before, so
/// the index of an effect is the length of the list after it.
fn effects<B: Effects>(mut b: B, list: &[Effect], key: &str, then: &str, spans: &mut Spans) -> B {
    for e in list {
        b = match &e.kind {
            EffectKind::Line(t) => b.effect(loremaster_model::Effect::Say(t.value.as_str().into())),
            EffectKind::Set(pairs) => pairs
                .iter()
                .fold(b, |b, (n, v)| b.set(n.value.as_str(), value(v))),
            EffectKind::Add(pairs) => pairs
                .iter()
                .fold(b, |b, (n, v)| b.add(n.value.as_str(), *v)),
            EffectKind::Move(pairs) => pairs
                .iter()
                .fold(b, |b, (n, to)| b.move_to(n.value.as_str(), to.as_str())),
            EffectKind::Goto(r) => b.goto(r.value.as_str()),
            EffectKind::End(r) => b.end(r.value.as_str()),
            EffectKind::Stop => b.stop(),
            EffectKind::Emit(r) => b.emit(r.value.as_str()),
            EffectKind::Start(t, None) => b.start_timer(t.value.as_str()),
            EffectKind::Start(t, Some(n)) => b.start_timer_for(t.value.as_str(), *n),
            EffectKind::Cancel(t) => b.cancel(t.value.as_str()),
            EffectKind::Call(c) => {
                b.effect(loremaster_model::Effect::Call(Id::new(c.value.as_str())))
            }
        };
        let i = b.effects_mut().len().saturating_sub(1);
        spans.add(key, format!("{then}[{i}]"), e.tokens.clone());
    }
    b
}

fn rule(p: &mut Pack, r: &Rule, spans: &mut Spans) {
    let key = format!("rule:{}", r.id.value);
    let mut b = p.rule(r.id.value.as_str(), event(&r.on));
    spans.add(&key, "on", r.on.tokens.clone());
    if let Some(w) = &r.when {
        spans.add(&key, "when", w.tokens.clone());
        b = b.when(w.source.as_str());
    }
    if r.once {
        b = b.once();
    }
    if r.instead {
        b = b.instead();
    }
    if let Some(w) = &r.within {
        spans.add(&key, "within", name_tokens(w));
        b = b.within(w.value.as_str());
    }
    if let Some(n) = r.priority {
        b = b.priority(n);
    }
    effects(b, &r.effects, &key, "then", spans);
    spans.add(&key, "", name_tokens(&r.id));
}

fn machine(p: &mut Pack, m: &Machine, spans: &mut Spans) {
    let key = format!("machine:{}", m.id.value);
    let at = format!("machines.{}", m.id.value);
    let mut b = p.machine(m.id.value.as_str(), m.start.value.as_str());
    for (state, description) in &m.states {
        b = match description {
            Some(d) => b.state_described(state.value.as_str(), d.value.as_str()),
            None => b.state(state.value.as_str()),
        };
    }
    for (i, step) in m.steps.iter().enumerate() {
        spans.add(&key, format!("{at}.steps[{i}]"), step.tokens.clone());
        let from: Vec<String> = step.from.iter().map(|n| n.value.clone()).collect();
        b = if step.from_list {
            b.step(from, event(&step.on), step.to.value.as_str())
        } else {
            b.step(from[0].as_str(), event(&step.on), step.to.value.as_str())
        };
        if let Some(w) = &step.when {
            b = b.when(w.source.as_str());
        }
        if step.instead {
            b = b.instead();
        }
        if let Some(say) = &step.say {
            b = b.say(say.value.as_str());
        }
        b = effects(
            b,
            &step.effects,
            &key,
            &format!("{at}.steps[{i}].then"),
            spans,
        );
    }
    if let Some((first, _)) = m.states.first() {
        spans.add(&key, format!("{at}.states"), name_tokens(first));
    }
    spans.add(&key, format!("{at}.start"), name_tokens(&m.start));
    spans.add(&key, "", name_tokens(&m.id));
}

fn timer(p: &mut Pack, t: &Timer, spans: &mut Spans) {
    let key = format!("timer:{}", t.id.value);
    let at = format!("timers.{}", t.id.value);
    let mut b = p.timer(t.id.value.as_str(), t.turns);
    if t.running {
        b = b.running();
    }
    if t.repeat {
        b = b.repeat();
    }
    if let Some(c) = &t.runs_while {
        spans.add(&key, format!("{at}.runs_while"), c.tokens.clone());
        b = b.runs_while(c.source.as_str());
    }
    for (n, list) in &t.at {
        b = b.at(*n);
        b = effects(b, list, &key, &format!("{at}.at.{n}"), spans);
    }
    spans.add(&key, "", name_tokens(&t.id));
}

fn ending(p: &mut Pack, e: &Ending, spans: &mut Spans) {
    let key = format!("ending:{}", e.id.value);
    let at = format!("endings.{}", e.id.value);
    let kind = match e.kind.value.as_str() {
        "win" => EndingKind::Win,
        "loss" => EndingKind::Loss,
        "death" => EndingKind::Death,
        _ => EndingKind::Neutral,
    };
    let mut b: EndingBuilder = p.ending(e.id.value.as_str(), kind, e.title.value.as_str());
    spans.add(&key, format!("{at}.title"), text_tokens(&e.title));
    if let Some(r) = e.rank {
        b = b.rank(r);
    }
    if let Some(t) = &e.text {
        spans.add(&key, format!("{at}.text"), text_tokens(t));
        b = b.text(t.value.as_str());
    }
    for n in &e.notes {
        b = if n.todo {
            b.todo(n.text.value.as_str())
        } else {
            b.note(n.text.value.as_str())
        };
    }
    spans.add(&key, "", name_tokens(&e.id));
}
