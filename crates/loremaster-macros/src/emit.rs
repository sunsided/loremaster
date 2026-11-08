//! The expansion: a function `pack()` that makes the pack with the builder
//! API of loremaster-dsl. Each entity's call has the span of its id, so the
//! builder's `#[track_caller]` gives the entity an `Origin::Rust` with the
//! file, line and column of the id in the `world!` block.

use crate::ast::*;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::LitStr;
use syn::spanned::Spanned;

pub fn expand(world: &World) -> TokenStream {
    let mut body = TokenStream::new();
    let vars: Vec<&Var> = world
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Var(v) => Some(v),
            _ => None,
        })
        .collect();
    for item in &world.items {
        body.extend(match item {
            Item::Extern(_) | Item::Var(_) => TokenStream::new(),
            Item::Story(s) => story(s, &vars),
            Item::Room(r) => room(r),
            Item::Thing(t) => thing(t),
            Item::Rule(r) => rule(r),
            Item::Machine(m) => machine(m),
            Item::Timer(t) => timer(t),
            Item::Ending(e) => ending(e),
            Item::Verb(v) => verb(v),
            Item::Test(t) => test(t),
            Item::Dialogue(d) => dialogue(d),
            Item::Thread(t) => thread(t),
            Item::Behavior(b) => {
                let span = b.span;
                let p = pack_ident(span);
                let b = name(b);
                quote_spanned!(span=> #p.behavior(#b);)
            }
            Item::Register(expr) => {
                let span = expr.span();
                let p = pack_ident(span);
                quote_spanned!(span=> #p.register(#expr);)
            }
        });
    }
    let pack = pack_ident(Span::call_site());
    // `pack <name>;` names the function, and the pack after the module
    // and the function, so that two blocks of one module differ.
    let (function, pack_name, doc) = match &world.name {
        Some(n) => {
            let f = Ident::new(&n.value, n.span);
            let suffix = s(&format!("::{}", n.value), n.span);
            (
                f,
                quote!(::core::concat!(::core::module_path!(), #suffix)),
                format!("The pack `{}` of this `world!` block.", n.value),
            )
        }
        None => (
            Ident::new("pack", Span::call_site()),
            quote!(::core::module_path!()),
            "The pack of this `world!` block.".to_string(),
        ),
    };
    quote! {
        #[doc = #doc]
        pub fn #function() -> ::loremaster::dsl::Pack {
            #[allow(unused_imports)]
            use ::loremaster::dsl::{
                BuildPack as _, Effects as _, FragmentExt as _, HasFrames as _, HasNotes as _,
            };
            let mut #pack = ::loremaster::dsl::Pack::new(#pack_name);
            #body
            #pack
        }
    }
}

/// `.replace()` for an entity with `replace` before it.
fn replace(on: bool, span: Span) -> TokenStream {
    if on {
        quote_spanned!(span=> .replace())
    } else {
        TokenStream::new()
    }
}

fn pack_ident(span: Span) -> Ident {
    Ident::new("__lm_pack", span)
}

fn s(value: &str, span: Span) -> LitStr {
    LitStr::new(value, span)
}

fn name(n: &Name) -> LitStr {
    s(&n.value, n.span)
}

fn text(t: &Text) -> LitStr {
    s(&t.value, t.span)
}

fn lit<L: Into<Literal>>(l: L, span: Span) -> Literal {
    let mut l = l.into();
    l.set_span(span);
    l
}

fn value(v: &Value, span: Span) -> TokenStream {
    match v {
        Value::Bool(b) => quote_spanned!(span=> #b),
        Value::Int(i) => {
            let l = lit(Literal::i64_suffixed(*i), span);
            quote_spanned!(span=> #l)
        }
        Value::Text(t) => {
            let l = s(t, span);
            quote_spanned!(span=> #l)
        }
    }
}

fn str_array(items: &[LitStr], span: Span) -> TokenStream {
    let n = items.len();
    quote_spanned!(span=> { let a: [&str; #n] = [#(#items),*]; a })
}

pub fn frag(f: &Frag, span: Span) -> TokenStream {
    let texts: Vec<LitStr> = f.texts.iter().map(text).collect();
    let mut out = if f.variants {
        let list = str_array(&texts, span);
        quote_spanned!(span=> ::loremaster::dsl::variants(#list))
    } else {
        let t = &texts[0];
        quote_spanned!(span=> ::loremaster::dsl::frag(#t))
    };
    if let Some(layer) = &f.layer {
        let v = Ident::new(&camel(&layer.value), layer.span);
        out.extend(quote_spanned!(span=> .layer(::loremaster::dsl::model::Layer::#v)));
    }
    if let Some(mode) = &f.mode {
        let v = Ident::new(&camel(&mode.value), mode.span);
        out.extend(quote_spanned!(span=> .mode(::loremaster::dsl::model::FragmentMode::#v)));
    }
    if let Some(when) = &f.when {
        let c = s(&when.source, span);
        out.extend(quote_spanned!(span=> .when(#c)));
    }
    if let Some(id) = &f.id {
        let id = name(id);
        out.extend(quote_spanned!(span=> .id(#id)));
    }
    if f.replaces {
        out.extend(quote_spanned!(span=> .replaces()));
    }
    for n in &f.notes {
        let n = text(n);
        out.extend(quote_spanned!(span=> .note(#n)));
    }
    out
}

/// `.frame_before(verb, frag)` and the like.
fn frame(f: &Frame, span: Span) -> TokenStream {
    let verb = name(&f.verb);
    let fr = frag(&f.frag, span);
    let m = Ident::new(
        match f.when {
            FrameWhen::Before => "frame_before",
            FrameWhen::Instead => "frame_instead",
            FrameWhen::After => "frame_after",
        },
        span,
    );
    quote_spanned!(span=> .#m(#verb, #fr))
}

/// `first_visit` is `FirstVisit`.
pub fn camel(word: &str) -> String {
    word.split('_')
        .map(|part| {
            let mut c = part.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

fn notes(list: &[&Note], span: Span) -> TokenStream {
    let mut out = TokenStream::new();
    for n in list {
        let t = text(&n.text);
        out.extend(if n.todo {
            quote_spanned!(span=> .todo(#t))
        } else {
            quote_spanned!(span=> .note(#t))
        });
    }
    out
}

fn story(st: &Story, vars: &[&Var]) -> TokenStream {
    let span = st.title.span;
    let p = pack_ident(span);
    let title = text(&st.title);
    let start = st.start.as_ref().map(name).expect("parse requires start");
    let mut chain = TokenStream::new();
    if let Some(a) = &st.author {
        let a = text(a);
        chain.extend(quote_spanned!(span=> .author(#a)));
    }
    if let Some(seed) = st.seed {
        let l = lit(Literal::u64_suffixed(seed), span);
        chain.extend(quote_spanned!(span=> .seed(#l)));
    }
    for f in &st.intro {
        let f = frag(f, span);
        chain.extend(quote_spanned!(span=> .intro(#f)));
    }
    if st.death_prompt == Some(false) {
        chain.extend(quote_spanned!(span=> .no_death_prompt()));
    }
    if let Some(t) = &st.clock_start {
        let t = text(t);
        chain.extend(quote_spanned!(span=> .clock_start(#t)));
    }
    if let Some(m) = st.minutes_per_turn {
        let l = lit(Literal::u16_suffixed(m), span);
        chain.extend(quote_spanned!(span=> .minutes_per_turn(#l)));
    }
    if let Some(phases) = &st.phases {
        let list = phases.iter().map(|(n, from)| {
            let n = name(n);
            let from = text(from);
            quote_spanned!(span=> (#n, #from))
        });
        chain.extend(quote_spanned!(span=> .phases([#(#list),*])));
    }
    for r in &st.relations {
        let r = name(r);
        chain.extend(quote_spanned!(span=> .relation(#r)));
    }
    if let Some(max) = st.score {
        let l = lit(Literal::i64_suffixed(max), span);
        chain.extend(quote_spanned!(span=> .score(#l)));
        if let Some(v) = &st.score_var {
            let v = name(v);
            chain.extend(quote_spanned!(span=> .score_var(#v)));
        }
        for (from, rank) in &st.ranks {
            let from = lit(Literal::i64_suffixed(*from), span);
            let rank = text(rank);
            chain.extend(quote_spanned!(span=> .rank(#from, #rank)));
        }
    }
    for v in vars {
        let id = name(&v.id);
        chain.extend(match (&v.ty, &v.initial) {
            (VarType::Enum(values), Value::Text(initial)) => {
                let values: Vec<LitStr> = values.iter().map(name).collect();
                let values = str_array(&values, span);
                quote_spanned!(span=> .var_enum(#id, #values, #initial))
            }
            (_, initial) => {
                let initial = value(initial, span);
                quote_spanned!(span=> .var(#id, #initial))
            }
        });
    }
    quote_spanned!(span=> #p.story(#title, #start) #chain;)
}

fn room(r: &Room) -> TokenStream {
    let span = r.id.span;
    let p = pack_ident(span);
    let id = name(&r.id);
    let room_name = text(&r.name);
    let mut chain = TokenStream::new();
    let mut note_list = Vec::new();
    for part in &r.parts {
        chain.extend(match part {
            RoomPart::At(x, y) => {
                let (x, y) = (
                    lit(Literal::f32_suffixed(*x), span),
                    lit(Literal::f32_suffixed(*y), span),
                );
                quote_spanned!(span=> .at(#x, #y))
            }
            RoomPart::Text(f) => {
                let f = frag(f, span);
                quote_spanned!(span=> .text(#f))
            }
            RoomPart::Exit(e) => exit(e, span),
            RoomPart::Tag(t) => {
                let t = text(t);
                quote_spanned!(span=> .tag(#t))
            }
            RoomPart::Region(n) => {
                let n = name(n);
                quote_spanned!(span=> .region(#n))
            }
            RoomPart::Level(l) => {
                let l = lit(Literal::i32_suffixed(*l), span);
                quote_spanned!(span=> .level(#l))
            }
            RoomPart::Note(n) => {
                note_list.push(n);
                TokenStream::new()
            }
            RoomPart::Frame(f) => frame(f, span),
        });
    }
    let notes = notes(&note_list, span);
    let replace = replace(r.replace, span);
    quote_spanned!(span=> #p.room(#id, #room_name) #chain #notes #replace;)
}

fn exit(e: &Exit, span: Span) -> TokenStream {
    let dir = name(&e.direction);
    let mut out = match (&e.to, &e.blocked) {
        (Some(to), _) => {
            let to = name(to);
            quote_spanned!(span=> .exit(#dir, #to))
        }
        (None, Some(blocked)) => {
            let b = text(blocked);
            quote_spanned!(span=> .exit_blocked(#dir, #b))
        }
        (None, None) => unreachable!("parse requires `->` or `blocked`"),
    };
    if let Some(d) = &e.door {
        let d = name(d);
        out.extend(quote_spanned!(span=> .door(#d)));
    }
    if let Some(c) = &e.visible {
        let c = s(&c.source, span);
        out.extend(quote_spanned!(span=> .visible(#c)));
    }
    if let Some(c) = &e.passable {
        let c = s(&c.source, span);
        out.extend(quote_spanned!(span=> .passable(#c)));
    }
    if e.to.is_some()
        && let Some(b) = &e.blocked
    {
        let b = text(b);
        out.extend(quote_spanned!(span=> .blocked(#b)));
    }
    if let Some(t) = &e.traversal {
        let t = text(t);
        out.extend(quote_spanned!(span=> .traversal(#t)));
    }
    out
}

fn thing(t: &Thing) -> TokenStream {
    let span = t.id.span;
    let p = pack_ident(span);
    let id = name(&t.id);
    let thing_name = text(&t.name);
    let mut chain = TokenStream::new();
    if let Some(loc) = &t.location {
        chain.extend(match &loc.kind {
            LocationKind::Room(r) => {
                let r = name(r);
                quote_spanned!(span=> .in_room(#r))
            }
            LocationKind::Inside(h) => {
                let h = name(h);
                quote_spanned!(span=> .in_thing(#h))
            }
            LocationKind::Upon(h) => {
                let h = name(h);
                quote_spanned!(span=> .on_thing(#h))
            }
            LocationKind::Carried => quote_spanned!(span=> .carried()),
            LocationKind::Nowhere => quote_spanned!(span=> .nowhere()),
        });
    }
    let mut note_list = Vec::new();
    for part in &t.parts {
        chain.extend(match part {
            ThingPart::Aliases(list) => {
                let list: Vec<LitStr> = list.iter().map(text).collect();
                let list = str_array(&list, span);
                quote_spanned!(span=> .aliases(#list))
            }
            ThingPart::Portable => quote_spanned!(span=> .portable()),
            ThingPart::Container => quote_spanned!(span=> .container()),
            ThingPart::Supporter => quote_spanned!(span=> .supporter()),
            ThingPart::Unlisted => quote_spanned!(span=> .unlisted()),
            ThingPart::Proper => quote_spanned!(span=> .proper()),
            ThingPart::Capacity(c) => {
                let c = lit(Literal::u32_suffixed(*c), span);
                quote_spanned!(span=> .capacity(#c))
            }
            ThingPart::Key(k) => {
                let k = name(k);
                quote_spanned!(span=> .key(#k))
            }
            ThingPart::Prop(n, v) => {
                let n = name(n);
                let v = value(v, span);
                quote_spanned!(span=> .prop(#n, #v))
            }
            ThingPart::Examine(f) => {
                let f = frag(f, span);
                quote_spanned!(span=> .text(#f))
            }
            ThingPart::Presence(f) => {
                let f = frag(f, span);
                quote_spanned!(span=> .presence(#f))
            }
            ThingPart::AlsoIn(n) => {
                let n = name(n);
                quote_spanned!(span=> .also_in(#n))
            }
            ThingPart::Note(n) => {
                note_list.push(n);
                TokenStream::new()
            }
            ThingPart::Frame(f) => frame(f, span),
            ThingPart::Name(n) => {
                let t = text(&n.text);
                let aliases = n.aliases.iter().map(text);
                let when = match &n.when {
                    Some(w) => {
                        let w = s(&w.source, span);
                        quote_spanned!(span=> ::core::option::Option::Some(::std::string::String::from(#w)))
                    }
                    None => quote_spanned!(span=> ::core::option::Option::None),
                };
                let proper = n.proper;
                quote_spanned!(span=> .name_stage(::loremaster::dsl::model::NameStage {
                    text: ::std::string::String::from(#t),
                    aliases: ::std::vec![#(::std::string::String::from(#aliases)),*],
                    when: #when,
                    proper: #proper,
                }))
            }
        });
    }
    let notes = notes(&note_list, span);
    let replace = replace(t.replace, span);
    quote_spanned!(span=> #p.thing(#id, #thing_name) #chain #notes #replace;)
}

fn event(e: &Event, span: Span) -> TokenStream {
    let list = |names: &[Name]| {
        let names: Vec<LitStr> = names.iter().map(name).collect();
        str_array(&names, span)
    };
    match &e.kind {
        EventKind::Start => quote_spanned!(span=> ::loremaster::dsl::on::start()),
        EventKind::Turn => quote_spanned!(span=> ::loremaster::dsl::on::turn()),
        EventKind::Enter(r) => {
            let r = name(r);
            quote_spanned!(span=> ::loremaster::dsl::on::enter(#r))
        }
        EventKind::Leave(r) => {
            let r = name(r);
            quote_spanned!(span=> ::loremaster::dsl::on::leave(#r))
        }
        EventKind::Time(t) => {
            let t = text(t);
            quote_spanned!(span=> ::loremaster::dsl::on::time(#t))
        }
        EventKind::Verb(v) => {
            let v = list(v);
            quote_spanned!(span=> ::loremaster::dsl::on::verb(#v))
        }
        EventKind::Check(v) => {
            let v = list(v);
            quote_spanned!(span=> ::loremaster::dsl::on::check(#v))
        }
        EventKind::Timer(t) => {
            let t = name(t);
            quote_spanned!(span=> ::loremaster::dsl::on::timer(#t))
        }
        EventKind::Event(t) => {
            let t = name(t);
            quote_spanned!(span=> ::loremaster::dsl::on::event(#t))
        }
        EventKind::Talk(v) => {
            let v = list(v);
            quote_spanned!(span=> ::loremaster::dsl::on::talk(#v))
        }
    }
}

/// A `Line`: one bare string as `Line::from`, which is what YAML makes of a
/// plain `say:`, or its segments in order.
fn line(segments: &[Segment], span: Span) -> TokenStream {
    if let [
        Segment {
            voice: SegmentVoice::Plain,
            text: t,
            when: None,
            ..
        },
    ] = segments
    {
        let t = text(t);
        return quote_spanned!(span=> ::loremaster::dsl::model::Line::from(#t));
    }
    let list = segments.iter().map(|segment| {
        let t = text(&segment.text);
        let mut seg = match &segment.voice {
            SegmentVoice::Plain => {
                quote_spanned!(span=> ::loremaster::dsl::model::LineSegment::plain(#t))
            }
            SegmentVoice::Narrate => {
                quote_spanned!(span=> ::loremaster::dsl::model::LineSegment::narrate(#t))
            }
            SegmentVoice::Says(by) => {
                let mut seg =
                    quote_spanned!(span=> ::loremaster::dsl::model::LineSegment::says(#t));
                if let Some(by) = by {
                    let by = name(by);
                    seg.extend(quote_spanned!(span=> .by(::loremaster::dsl::model::Id::new(#by))));
                }
                seg
            }
        };
        if let Some(when) = &segment.when {
            let c = s(&when.source, span);
            seg.extend(quote_spanned!(span=> .when(#c)));
        }
        seg
    });
    quote_spanned!(span=> ::loremaster::dsl::model::Line::new(::std::vec![#(#list),*]))
}

fn effects(list: &[Effect], span: Span) -> TokenStream {
    let mut out = TokenStream::new();
    for e in list {
        out.extend(match &e.kind {
            // Not `.say(t)`: on a machine, that is the inherent method for
            // the step's `say:` field.
            EffectKind::Line(segments) => {
                let l = line(segments, span);
                quote_spanned!(span=> .effect(::loremaster::dsl::model::Effect::Say(#l)))
            }
            EffectKind::Set(pairs) => pairs
                .iter()
                .map(|(n, v)| {
                    let (n, v) = (name(n), value(v, span));
                    quote_spanned!(span=> .set(#n, #v))
                })
                .collect(),
            EffectKind::Add(pairs) => pairs
                .iter()
                .map(|(n, v)| {
                    let n = name(n);
                    let v = lit(Literal::i64_suffixed(*v), span);
                    quote_spanned!(span=> .add(#n, #v))
                })
                .collect(),
            EffectKind::Move(pairs) => pairs
                .iter()
                .map(|(n, to)| {
                    let (n, to) = (name(n), s(to, span));
                    quote_spanned!(span=> .move_to(#n, #to))
                })
                .collect(),
            EffectKind::Goto(r) => {
                let r = name(r);
                quote_spanned!(span=> .goto(#r))
            }
            EffectKind::End(r) => {
                let r = name(r);
                quote_spanned!(span=> .end(#r))
            }
            EffectKind::Stop => quote_spanned!(span=> .stop()),
            EffectKind::Emit(r) => {
                let r = name(r);
                quote_spanned!(span=> .emit(#r))
            }
            EffectKind::Start(t, None) => {
                let t = name(t);
                quote_spanned!(span=> .start_timer(#t))
            }
            EffectKind::Start(t, Some(n)) => {
                let t = name(t);
                let n = lit(Literal::u32_suffixed(*n), span);
                quote_spanned!(span=> .start_timer_for(#t, #n))
            }
            EffectKind::Cancel(r) => {
                let r = name(r);
                quote_spanned!(span=> .cancel(#r))
            }
            EffectKind::Call(b) => {
                let b = name(b);
                quote_spanned!(span=> .effect(::loremaster::dsl::model::Effect::Call(
                    ::loremaster::dsl::model::Id::new(#b)
                )))
            }
            EffectKind::Rust(r) => {
                let b = s(&r.id(), span);
                quote_spanned!(span=> .effect(::loremaster::dsl::model::Effect::Call(
                    ::loremaster::dsl::model::Id::new(#b)
                )))
            }
        });
    }
    out
}

fn rule(r: &Rule) -> TokenStream {
    let span = r.id.span;
    let p = pack_ident(span);
    let id = name(&r.id);
    let on = event(&r.on, span);
    let mut chain = TokenStream::new();
    if let Some(when) = &r.when {
        let c = s(&when.source, span);
        chain.extend(quote_spanned!(span=> .when(#c)));
    }
    if r.once {
        chain.extend(quote_spanned!(span=> .once()));
    }
    if r.instead {
        chain.extend(quote_spanned!(span=> .instead()));
    }
    if let Some(w) = &r.within {
        let w = name(w);
        chain.extend(quote_spanned!(span=> .within(#w)));
    }
    if let Some(n) = r.priority {
        let n = lit(Literal::i32_suffixed(n), span);
        chain.extend(quote_spanned!(span=> .priority(#n)));
    }
    chain.extend(effects(&r.effects, span));
    let blocks = rust_blocks(&r.effects);
    quote_spanned!(span=> #p.rule(#id, #on) #chain; #blocks)
}

/// The behavior of each `rust` block, kept by the pack.
fn rust_blocks(list: &[Effect]) -> TokenStream {
    let mut out = TokenStream::new();
    for e in list {
        let EffectKind::Rust(r) = &e.kind else {
            continue;
        };
        let span = r.cx.span();
        let p = pack_ident(span);
        let owner = s(&r.owner, span);
        let (cx, body) = (&r.cx, &r.body);
        out.extend(quote_spanned!(span=>
            #p.register(::loremaster::dsl::RustBlock::new(
                #owner,
                |#cx: &mut ::loremaster::dsl::runtime::Cx<'_>| -> ::loremaster::dsl::runtime::Flow {
                    ::loremaster::dsl::IntoFlow::into_flow(#body)
                },
            ));
        ));
    }
    out
}

fn machine(m: &Machine) -> TokenStream {
    let span = m.id.span;
    let p = pack_ident(span);
    let id = name(&m.id);
    let start = name(&m.start);
    let mut chain = TokenStream::new();
    for (state, description) in &m.states {
        let state = name(state);
        chain.extend(match description {
            Some(d) => {
                let d = text(d);
                quote_spanned!(span=> .state_described(#state, #d))
            }
            None => quote_spanned!(span=> .state(#state)),
        });
    }
    for step in &m.steps {
        let from = if step.from_list {
            let list: Vec<LitStr> = step.from.iter().map(name).collect();
            str_array(&list, span)
        } else {
            let f = name(&step.from[0]);
            quote_spanned!(span=> #f)
        };
        let on = event(&step.on, span);
        let to = name(&step.to);
        chain.extend(quote_spanned!(span=> .step(#from, #on, #to)));
        if let Some(when) = &step.when {
            let c = s(&when.source, span);
            chain.extend(quote_spanned!(span=> .when(#c)));
        }
        if step.instead {
            chain.extend(quote_spanned!(span=> .instead()));
        }
        if let Some(say) = &step.say {
            let l = line(say, span);
            // The inherent `MachineBuilder::say`: the step's `say:` field.
            chain.extend(quote_spanned!(span=> .say(#l)));
        }
        chain.extend(effects(&step.effects, span));
    }
    let note_list: Vec<&Note> = m.notes.iter().collect();
    chain.extend(notes(&note_list, span));
    chain.extend(replace(m.replace, span));
    let blocks: TokenStream = m.steps.iter().map(|s| rust_blocks(&s.effects)).collect();
    quote_spanned!(span=> #p.machine(#id, #start) #chain; #blocks)
}

fn timer(t: &Timer) -> TokenStream {
    let span = t.id.span;
    let p = pack_ident(span);
    let id = name(&t.id);
    let turns = lit(Literal::u32_suffixed(t.turns), span);
    let mut chain = TokenStream::new();
    if t.running {
        chain.extend(quote_spanned!(span=> .running()));
    }
    if t.repeat {
        chain.extend(quote_spanned!(span=> .repeat()));
    }
    if let Some(c) = &t.runs_while {
        let c = s(&c.source, span);
        chain.extend(quote_spanned!(span=> .runs_while(#c)));
    }
    for (n, list) in &t.at {
        let n = lit(Literal::u32_suffixed(*n), span);
        chain.extend(quote_spanned!(span=> .at(#n)));
        chain.extend(effects(list, span));
    }
    let note_list: Vec<&Note> = t.notes.iter().collect();
    chain.extend(notes(&note_list, span));
    chain.extend(replace(t.replace, span));
    quote_spanned!(span=> #p.timer(#id, #turns) #chain;)
}

fn ending(e: &Ending) -> TokenStream {
    let span = e.id.span;
    let p = pack_ident(span);
    let id = name(&e.id);
    let title = text(&e.title);
    let kind = Ident::new(&camel(&e.kind.value), e.kind.span);
    let mut chain = TokenStream::new();
    if let Some(r) = e.rank {
        let r = lit(Literal::u32_suffixed(r), span);
        chain.extend(quote_spanned!(span=> .rank(#r)));
    }
    if let Some(t) = &e.text {
        let t = text(t);
        chain.extend(quote_spanned!(span=> .text(#t)));
    }
    let note_list: Vec<&Note> = e.notes.iter().collect();
    chain.extend(notes(&note_list, span));
    chain.extend(replace(e.replace, span));
    quote_spanned!(span=>
        #p.ending(#id, ::loremaster::dsl::model::EndingKind::#kind, #title) #chain;
    )
}

fn verb(v: &Verb) -> TokenStream {
    let span = v.id.span;
    let p = pack_ident(span);
    let id = name(&v.id);
    let label = text(&v.label);
    let mut chain = TokenStream::new();
    if !v.words.is_empty() {
        let list: Vec<LitStr> = v.words.iter().map(text).collect();
        let list = str_array(&list, span);
        chain.extend(quote_spanned!(span=> .words(#list)));
    }
    if let Some(slots) = &v.slots {
        let s = Ident::new(&camel(&slots.value), slots.span);
        chain.extend(quote_spanned!(span=> .slots(::loremaster::dsl::model::VerbSlots::#s)));
    }
    if let Some(p) = &v.preposition {
        let p = text(p);
        chain.extend(quote_spanned!(span=> .preposition(#p)));
    }
    if v.no_bar {
        chain.extend(quote_spanned!(span=> .parser_only()));
    }
    if let Some(d) = &v.default {
        let d = text(d);
        chain.extend(quote_spanned!(span=> .default_response(#d)));
    }
    chain.extend(replace(v.replace, span));
    quote_spanned!(span=> #p.verb(#id, #label) #chain;)
}

fn test(t: &Test) -> TokenStream {
    let span = t.id.span;
    let p = pack_ident(span);
    let id = name(&t.id);
    let test_name = text(&t.name);
    let mut chain = TokenStream::new();
    if let Some((phase, needs)) = &t.pending {
        let phase = lit(Literal::u8_suffixed(*phase), span);
        let needs = text(needs);
        chain.extend(quote_spanned!(span=> .pending(#phase, #needs)));
    }
    for step in &t.steps {
        let command = text(&step.command);
        for _ in 0..step.times {
            chain.extend(quote_spanned!(span=> .step(#command)));
        }
        for check in &step.checks {
            chain.extend(expectation(check, span, false));
        }
    }
    for e in &t.expect {
        chain.extend(expectation(e, span, true));
    }
    // The key of a test from world! is its id, as the file stem is for a
    // YAML test (`tests/<id>.yaml`).
    quote_spanned!(span=>
        #p.test(#test_name) #chain;
        if let ::core::option::Option::Some(last) = #p.walkthroughs.last_mut() {
            last.0 = ::std::string::String::from(#id);
        }
    )
}

fn expectation(e: &Expectation, span: Span, end: bool) -> TokenStream {
    let texts = |list: &[Text], method: &str| -> TokenStream {
        let m = Ident::new(method, span);
        list.iter()
            .map(|t| {
                let t = text(t);
                quote_spanned!(span=> .#m(#t))
            })
            .collect()
    };
    let m = |step: &str, at_end: &str| Ident::new(if end { at_end } else { step }, span);
    match e {
        Expectation::Sees(list) => texts(list, if end { "expect_see" } else { "sees" }),
        Expectation::NotSees(list) => texts(list, if end { "expect_not_see" } else { "not_sees" }),
        Expectation::Fact(n, v) => {
            let method = m("fact", "expect_fact");
            let (n, v) = (name(n), value(v, span));
            quote_spanned!(span=> .#method(#n, #v))
        }
        Expectation::Room(r) => {
            let method = m("in_room", "expect_room");
            let r = name(r);
            quote_spanned!(span=> .#method(#r))
        }
        Expectation::Ending(r) => {
            let method = m("ending", "expect_ending");
            let r = name(r);
            quote_spanned!(span=> .#method(#r))
        }
        Expectation::Ended(b) => {
            let method = m("ended", "expect_ended");
            quote_spanned!(span=> .#method(#b))
        }
    }
}

fn thread(t: &Thread) -> TokenStream {
    let span = t.id.span;
    let p = pack_ident(span);
    let id = name(&t.id);
    let thread_name = text(&t.name);
    let mut chain = TokenStream::new();
    if let Some(st) = &t.status {
        let v = Ident::new(&camel(&st.value), st.span);
        chain.extend(quote_spanned!(span=> .status(::loremaster::dsl::model::ThreadStatus::#v)));
    }
    if let Some(summary) = &t.summary {
        let summary = text(summary);
        chain.extend(quote_spanned!(span=> .summary(#summary)));
    }
    for f in &t.tracks {
        let f = name(f);
        chain.extend(quote_spanned!(span=> .tracks(#f)));
    }
    let note_list: Vec<&Note> = t.notes.iter().collect();
    chain.extend(notes(&note_list, span));
    chain.extend(replace(t.replace, span));
    quote_spanned!(span=> #p.thread(#id, #thread_name) #chain;)
}

/// A dialogue: the builder, node by node, each node's `when` and outcomes
/// before its options, as in `check::dialogue`.
fn dialogue(d: &Dialogue) -> TokenStream {
    let span = d.id.span;
    let p = pack_ident(span);
    let id = name(&d.id);
    let tree = &d.tree.dialogue;
    let with = s(&tree.with, span);
    let start: Vec<LitStr> = tree.start.iter().map(|k| s(k, span)).collect();
    let start = str_array(&start, span);
    let mut chain = TokenStream::new();
    let when = |w: &Option<Cond>| match w {
        Some(w) => {
            let c = s(&w.source, span);
            quote_spanned!(span=> .when(#c))
        }
        None => TokenStream::new(),
    };
    for node in &tree.nodes {
        let key = s(&node.key, span);
        let say = line(&node.say.segments, span);
        chain.extend(quote_spanned!(span=> .node(#key, #say)));
        if let Some(sp) = &node.speaker {
            let sp = s(sp, span);
            chain.extend(quote_spanned!(span=> .speaker(#sp)));
        }
        for l in &node.lines {
            let sp = s(&l.speaker, span);
            let say = line(&l.say.segments, span);
            chain.extend(quote_spanned!(span=> .line(#sp, #say)));
        }
        if let Some(next) = &node.next {
            let next = s(next.as_key(), span);
            chain.extend(quote_spanned!(span=> .next(#next)));
        }
        chain.extend(when(&node.when));
        chain.extend(outcomes(&node.outcomes, span));
        for option in &node.options {
            let say = line(
                option
                    .say
                    .as_ref()
                    .map(|s| s.segments.as_slice())
                    .unwrap_or(&[]),
                span,
            );
            let go = s(option.go.as_key(), span);
            chain.extend(quote_spanned!(span=> .option(#say, #go)));
            if let Some(oid) = &option.id {
                let oid = s(oid, span);
                chain.extend(quote_spanned!(span=> .option_id(#oid)));
            }
            chain.extend(when(&option.when));
            chain.extend(outcomes(&option.outcomes, span));
        }
    }
    chain.extend(replace(d.replace, span));
    quote_spanned!(span=> #p.dialogue(#id, #with).start(#start) #chain;)
}

fn outcomes(o: &dramaturge_syntax::Outcomes<Effect>, span: Span) -> TokenStream {
    let mut out = TokenStream::new();
    for (k, v) in &o.set {
        let k = s(k, span);
        let v = match v {
            dramaturge_syntax::Value::Bool(b) => quote_spanned!(span=> #b),
            dramaturge_syntax::Value::Int(i) => {
                let l = lit(Literal::i64_suffixed(*i), span);
                quote_spanned!(span=> #l)
            }
            dramaturge_syntax::Value::Text(t) => {
                let l = s(t, span);
                quote_spanned!(span=> #l)
            }
            // A kind of value that a later dramaturge adds.
            _ => quote_spanned!(span=> ::core::compile_error!("this value is not in `world!` yet")),
        };
        out.extend(quote_spanned!(span=> .set(#k, #v)));
    }
    for (k, n) in &o.add {
        let k = s(k, span);
        let n = lit(Literal::i64_suffixed(*n), span);
        out.extend(quote_spanned!(span=> .add(#k, #n)));
    }
    for (k, n) in &o.relation {
        let k = s(k, span);
        let n = lit(Literal::i32_suffixed(*n), span);
        out.extend(quote_spanned!(span=> .relation(#k, #n)));
    }
    for m in &o.remember {
        let m = s(m, span);
        out.extend(quote_spanned!(span=> .remember(#m)));
    }
    if !o.then.is_empty() {
        let list = effects(&o.then, span);
        out.extend(quote_spanned!(span=> .then_effects(
            ::std::vec::Vec::<::loremaster::dsl::model::Effect>::new() #list
        )));
    }
    out
}
