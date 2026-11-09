//! The checks at compile time. The macro builds the pack of the block with
//! the builder, as the expansion will at run time, and runs the checks of
//! `loremaster_project::check_packs` on it, together with a pack of stubs for the `extern`
//! ids. Each error becomes a `compile_error!` at the tokens of the entity
//! and the field that it names.
//!
//! The origin of each entity in this pack has the entity key as its file
//! (`room:kitchen`). So a diagnostic names its entity by its file, and its
//! `at` (`exits[0].to`) names the field; [`Spans`] finds the tokens of both.

use crate::ast::*;
use crate::emit::camel;
use dramaturge_syntax::{OutcomePlace, Outcomes, Place};
use loremaster_dsl::{
    BuildPack, DialogueBuilder, Effects, EndingBuilder, FragmentExt, HasFrames, HasNotes,
    RoomBuilder, ThingBuilder, frag, on, variants,
};
use loremaster_model::{
    EndingKind, EventPattern, Fragment, FragmentMode, Id, Layer, Line, LineSegment,
    NameStage as ModelNameStage, ThreadStatus, TimeOfDay, VarDef, VarType as ModelVarType,
    VerbSlots,
};
use loremaster_project::{Origin, Pack, Severity, assemble};
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
            Item::Dialogue(d) => {
                defined.insert(("dialogue".into(), d.id.value.clone()));
                if let Err(e) = d.tree.check(false) {
                    errors.push(e);
                }
            }
            Item::Thread(t) => {
                defined.insert(("thread".into(), t.id.value.clone()));
            }
            Item::Behavior(b) => {
                defined.insert(("behavior".into(), b.value.clone()));
            }
            // Its id is known only at run time (`Behavior::id`).
            Item::Register(_) => {}
            Item::Test(t) => {
                if !tests.insert(t.id.value.clone()) {
                    errors.push(Error::new(
                        t.id.span,
                        format!("the test `{}` is there twice", t.id.value),
                    ));
                }
            }
            Item::Extern(_) | Item::Interface(..) => {}
        }
    }
    if let Some(at) = two_interfaces(world) {
        errors.push(Error::new(at, "a block has one `interface` section"));
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
                    if story_seen && matches!(e.kind, ExternKind::Relation) {
                        errors.push(Error::new(
                            e.id.span,
                            format!(
                                "the relations live in the story header, and this block has it, so no other pack can define `{}`; write `relations: [{}]` in the story header",
                                e.id.value, e.id.value
                            ),
                        ));
                    }
                    if story_seen && matches!(e.kind, ExternKind::Var(_)) {
                        errors.push(Error::new(
                            e.id.span,
                            format!(
                                "the variables live in the story header, and this block has it, so no other pack can define `{}`; write `var {}: <type> = <value>;`",
                                e.id.value, e.id.value
                            ),
                        ));
                    }
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
        ExternKind::Thing(_) => "thing",
        ExternKind::Var(_) => "var",
        ExternKind::Machine(_) => "machine",
        ExternKind::Timer => "timer",
        ExternKind::Ending => "ending",
        ExternKind::Verb => "verb",
        ExternKind::Behavior => "behavior",
        ExternKind::Relation => "relation",
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
            hash: String::new(),
        };
    }
    let stubs = stubs(world, &mut pack, &mut spans);
    // check_packs, with one change: the rule of each machine step gets its
    // own file, so that its diagnostics point at the step and not at the
    // machine.
    let diagnostics = match assemble(vec![stubs, pack]) {
        Ok(mut project) => {
            for item in &world.items {
                let Item::Machine(m) = item else { continue };
                for i in 0..m.steps.len() {
                    let rule = format!("rule:{}#{}", m.id.value, i + 1);
                    if let Some(origin) = project.origins.get_mut(&rule) {
                        *origin = Origin::Rust {
                            file: step_key(&m.id.value, i),
                            line: 0,
                            column: 0,
                            pack: "world".into(),
                            hash: String::new(),
                        };
                    }
                }
            }
            loremaster_project::check(&project)
        }
        Err(diagnostics) => diagnostics,
    };
    let has_story = world.items.iter().any(|i| matches!(i, Item::Story(_)));
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    for d in &diagnostics {
        if d.severity != Severity::Error || d.file == EXTERN {
            continue;
        }
        if !seen.insert((d.file.clone(), d.at.clone(), d.message.clone())) {
            continue;
        }
        let message = rust_hint(&d.message, has_story);
        errors.push(match spans.find(&d.file, &d.at) {
            Some(tokens) => Error::new_spanned(tokens, message),
            None => Error::new(Span::call_site(), message),
        });
    }
    combine(errors)
}

/// The checks write their hints for YAML files. An unknown id in a block
/// gets the hint for Rust: define it, or declare it as an id of another
/// pack. A block with a story header has its relations there (`validate`
/// rejects an `extern` relation in it).
fn rust_hint(message: &str, has_story: bool) -> String {
    let message = message.split(": add it as ").next().unwrap_or(message);
    if let Some(rest) = message.strip_prefix("unknown relation `")
        && let Some((axis, _)) = rest.split_once('`')
    {
        return if has_story {
            format!("unknown relation `{axis}`; add it to `relations: [..]` in the story header")
        } else {
            format!(
                "unknown relation `{axis}`; add it to `relations: [..]` in the story header, or write `extern {{ relation {axis}; }}` for one of another pack"
            )
        };
    }
    if let Some(rest) = message.strip_prefix("`call: ")
        && let Some((id, _)) = rest.split_once("` names no behavior")
    {
        return format!(
            "`call: {id}` names no behavior of this block; `behavior <expr>;` does not tell the macro its id, so write `extern {{ behavior {id}; }}` for that or for one of another pack"
        );
    }
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
    let mut relations = Vec::new();
    for item in &world.items {
        let Item::Extern(list) = item else { continue };
        for e in list {
            let id = e.id.value.as_str();
            match &e.kind {
                ExternKind::Room => {
                    stub.room(id, id);
                }
                ExternKind::Thing(props) => {
                    // Any holder: the block may put a thing inside or upon it.
                    let mut t = stub.thing(id, id).container().supporter();
                    for (prop, ty) in props {
                        let prop = prop.value.as_str();
                        t = match ty {
                            PropType::Bool => t.prop(prop, false),
                            PropType::Int => t.prop(prop, 0i64),
                            PropType::Text => t.prop(prop, ""),
                        };
                    }
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
                        min: None,
                        max: None,
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
                ExternKind::Relation => relations.push(Id::new(id)),
            }
        }
    }
    // A `replace` item replaces an entity of an earlier pack: a stub gives
    // it one here.
    for item in &world.items {
        match item {
            Item::Room(r) if r.replace => {
                stub.room(r.id.value.as_str(), "");
            }
            Item::Thing(t) if t.replace => {
                stub.thing(t.id.value.as_str(), "");
            }
            Item::Machine(m) if m.replace => {
                let start = m.start.value.as_str();
                stub.machine(m.id.value.as_str(), start).state(start);
            }
            Item::Timer(t) if t.replace => {
                stub.timer(t.id.value.as_str(), 1);
            }
            Item::Ending(e) if e.replace => {
                stub.ending(e.id.value.as_str(), EndingKind::Neutral, "");
            }
            Item::Verb(v) if v.replace => {
                stub.verb(v.id.value.as_str(), "");
            }
            Item::Dialogue(d) if d.replace => {
                stub.dialogue(d.id.value.as_str(), d.tree.dialogue.with.as_str());
            }
            Item::Thread(t) if t.replace => {
                stub.thread(t.id.value.as_str(), "");
            }
            _ => {}
        }
    }
    match &mut pack.header {
        Some(header) => {
            header.vars.extend(vars);
            header.relations.extend(relations);
        }
        None => {
            stub.story("extern", "extern");
            if let Some(header) = &mut stub.header {
                header.vars.extend(vars);
                header.relations.extend(relations);
            }
        }
    }
    for origin in stub.origins.values_mut() {
        *origin = Origin::Rust {
            file: EXTERN.into(),
            line: 0,
            column: 0,
            pack: "extern".into(),
            hash: String::new(),
        };
    }
    stub
}

fn two_interfaces(world: &World) -> Option<proc_macro2::Span> {
    (world.items.iter())
        .filter_map(|i| match i {
            Item::Interface(_, at) => Some(*at),
            _ => None,
        })
        .nth(1)
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
            Item::Extern(_) | Item::Var(_) | Item::Interface(..) => {}
            Item::Story(s) => story(&mut p, s, &vars, world, spans),
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
                    b = b.default_response(d.value.as_str());
                }
                if v.replace {
                    b.replace();
                }
                spans.add(&format!("verb:{}", v.id.value), "", name_tokens(&v.id));
            }
            // Tests have no checks of the schema.
            Item::Test(_) => {}
            Item::Dialogue(d) => dialogue(&mut p, d, spans),
            Item::Thread(t) => {
                let mut b = p.thread(t.id.value.as_str(), t.name.value.as_str());
                if let Some(s) = &t.status {
                    b = b.status(thread_status(&s.value));
                }
                if let Some(s) = &t.summary {
                    b = b.summary(s.value.as_str());
                }
                for f in &t.tracks {
                    b = b.tracks(f.value.as_str());
                }
                for n in &t.notes {
                    b = if n.todo {
                        b.todo(n.text.value.as_str())
                    } else {
                        b.note(n.text.value.as_str())
                    };
                }
                if t.replace {
                    b.replace();
                }
                spans.add(&format!("thread:{}", t.id.value), "", name_tokens(&t.id));
            }
            Item::Behavior(id) => {
                p.behavior(id.value.as_str());
                spans.add(&format!("behavior:{}", id.value), "", name_tokens(id));
            }
            Item::Register(_) => {}
        }
    }
    p
}

/// The tokens for the errors of a whole line: the string, when the line is
/// one bare string (the checks report its markup at the line, not at a
/// segment), else the whole `line ..;` effect.
fn line_tokens(segments: &[Segment], effect: &TokenStream) -> TokenStream {
    match segments {
        [
            Segment {
                voice: SegmentVoice::Plain,
                when: None,
                text,
                ..
            },
        ] => text_tokens(text),
        _ => effect.clone(),
    }
}

/// The spans of the segments of a line at `at`: the checks name a segment
/// `<at>.say[j]`, and its condition and speaker below it.
fn segment_spans(spans: &mut Spans, key: &str, at: &str, segments: &[Segment]) {
    for (j, s) in segments.iter().enumerate() {
        spans.add(key, format!("{at}.say[{j}]"), s.tokens.clone());
        if let Some(w) = &s.when {
            spans.add(key, format!("{at}.say[{j}].when"), w.tokens.clone());
        }
        if let SegmentVoice::Says(Some(by)) = &s.voice {
            spans.add(key, format!("{at}.say[{j}].by"), name_tokens(by));
        }
    }
}

/// A `Line`, as the expansion makes it (`crate::emit`).
fn line(segments: &[Segment]) -> Line {
    if let [
        Segment {
            voice: SegmentVoice::Plain,
            text,
            when: None,
            ..
        },
    ] = segments
    {
        return Line::from(text.value.as_str());
    }
    Line::new(
        segments
            .iter()
            .map(|s| {
                let mut seg = match &s.voice {
                    SegmentVoice::Plain => LineSegment::plain(s.text.value.as_str()),
                    SegmentVoice::Narrate => LineSegment::narrate(s.text.value.as_str()),
                    SegmentVoice::Says(None) => LineSegment::says(s.text.value.as_str()),
                    SegmentVoice::Says(Some(by)) => {
                        LineSegment::says(s.text.value.as_str()).by(Id::new(by.value.as_str()))
                    }
                };
                if let Some(when) = &s.when {
                    seg = seg.when(when.source.as_str());
                }
                seg
            })
            .collect(),
    )
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
    if let Some(id) = &f.id {
        out = out.id(id.value.as_str());
    }
    if f.replaces {
        out = out.replaces();
    }
    for n in &f.notes {
        out = FragmentExt::note(out, n.value.as_str());
    }
    out
}

pub fn thread_status(word: &str) -> ThreadStatus {
    match word {
        "resolved" => ThreadStatus::Resolved,
        "dropped" => ThreadStatus::Dropped,
        _ => ThreadStatus::Open,
    }
}

/// A frame on a room or a thing: the fragment and its span at
/// `frames.<verb>.<when>[i]`; `count` counts the fragments of each list.
fn frame<B: HasFrames>(
    b: B,
    f: &Frame,
    key: &str,
    count: &mut std::collections::BTreeMap<String, usize>,
    spans: &mut Spans,
) -> B {
    let verb = f.verb.value.as_str();
    let when = match f.when {
        FrameWhen::Before => "before",
        FrameWhen::Instead => "instead",
        FrameWhen::After => "after",
    };
    let at = format!("frames.{verb}.{when}");
    let i = count.entry(at.clone()).or_default();
    spans.add(key, format!("frames.{verb}"), name_tokens(&f.verb));
    spans.add(key, format!("{at}[{i}]"), f.frag.tokens.clone());
    *i += 1;
    let fr = fragment(&f.frag);
    match f.when {
        FrameWhen::Before => b.frame_before(verb, fr),
        FrameWhen::Instead => b.frame_instead(verb, fr),
        FrameWhen::After => b.frame_after(verb, fr),
    }
}

fn name_stage(n: &NameStage) -> ModelNameStage {
    ModelNameStage {
        text: n.text.value.clone(),
        aliases: n.aliases.iter().map(|a| a.value.clone()).collect(),
        head: None,
        when: n.when.as_ref().map(|w| w.source.clone()),
        proper: n.proper,
    }
}

fn story(p: &mut Pack, s: &Story, vars: &[&Var], world: &World, spans: &mut Spans) {
    let start = s.start.as_ref().expect("parse requires start");
    let mut b = p.story(s.title.value.as_str(), start.value.as_str());
    if let Some(i) = crate::emit::interface_of(world) {
        b = b.interface(i.clone());
    }
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
    if s.fresh_rolls == Some(true) {
        b = b.fresh_rolls();
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
    for r in &s.relations {
        b = b.relation(r.value.as_str());
    }
    if let Some(max) = s.score {
        b = b.score(max);
        if let Some(v) = &s.score_var {
            b = b.score_var(v.value.as_str());
            spans.add("story", "score.var", name_tokens(v));
        }
        for (i, (from, rank)) in s.ranks.iter().enumerate() {
            b = b.rank(*from, rank.value.as_str());
            spans.add("story", format!("score.ranks[{i}]"), text_tokens(rank));
        }
    }
    for (i, (var, stat)) in s.stats.iter().enumerate() {
        b = b.stat(stat.var.as_str(), stat.label.as_str());
        if let Some(short) = &stat.short {
            b = b.stat_short(&stat.var, short.as_str());
        }
        if let Some(max) = &stat.max {
            b = b.stat_max(&stat.var, max.clone());
        }
        spans.add("story", format!("stats[{i}]"), name_tokens(var));
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
        let as_limit = |n: &Name| {
            n.value
                .parse::<i64>()
                .map_or_else(|_| n.value.as_str().into(), loremaster_model::Limit::Value)
        };
        if let Some(min) = &v.min {
            b = b.var_min(id, as_limit(min));
            spans.add("story", format!("vars.{id}.min"), name_tokens(min));
        }
        if let Some(max) = &v.max {
            b = b.var_max(id, as_limit(max));
            spans.add("story", format!("vars.{id}.max"), name_tokens(max));
        }
    }
    spans.add("story", "", s.tokens.clone());
    spans.add("story", "start", name_tokens(start));
}

fn room(p: &mut Pack, r: &Room, spans: &mut Spans) {
    let key = format!("room:{}", r.id.value);
    let mut b: RoomBuilder = p.room(r.id.value.as_str(), r.name.value.as_str());
    let (mut texts, mut exits) = (0, 0);
    let mut frames = std::collections::BTreeMap::new();
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
            RoomPart::Frame(f) => frame(b, f, &key, &mut frames, spans),
        };
    }
    if r.replace {
        b.replace();
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
    let (mut texts, mut presence, mut stages) = (0, 0, 0);
    let mut frames = std::collections::BTreeMap::new();
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
            ThingPart::Frame(f) => frame(b, f, &key, &mut frames, spans),
            ThingPart::Name(n) => {
                spans.add(&key, format!("names[{stages}]"), n.tokens.clone());
                if let Some(w) = &n.when {
                    spans.add(&key, format!("names[{stages}].when"), w.tokens.clone());
                }
                stages += 1;
                b.name_stage(name_stage(n))
            }
        };
    }
    if t.replace {
        b.replace();
    }
    spans.add(&key, "", name_tokens(&t.id));
}

fn event(e: &Event) -> EventPattern {
    let ids = |list: &[Name]| list.iter().map(|n| n.value.clone()).collect::<Vec<_>>();
    match &e.kind {
        EventKind::Start => on::start(),
        EventKind::Turn => on::turn(),
        EventKind::Next => on::next(),
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

/// Where the spans of a list of effects go: the entity key, the `at` of
/// the list, and the index of its first effect there.
struct Then<'a> {
    key: &'a str,
    at: &'a str,
    offset: usize,
}

/// Apply the effects in order, and record their spans: the builder joins a
/// `set` into the one before, so the index of an effect is the length of
/// the list after it.
fn effects<B: Effects>(mut b: B, list: &[Effect], then: &Then, spans: &mut Spans) -> B {
    for e in list {
        b = match &e.kind {
            EffectKind::Line(segments) => b.effect(loremaster_model::Effect::Say(line(segments))),
            EffectKind::Set(pairs) => pairs
                .iter()
                .fold(b, |b, (n, v)| b.set(n.value.as_str(), value(v))),
            EffectKind::Add(pairs) => pairs
                .iter()
                .fold(b, |b, (n, v)| b.add(n.value.as_str(), *v)),
            EffectKind::Calc(target, sum) => b.calc(target.value.as_str(), sum.as_str()),
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
            EffectKind::Rust(r) => b.effect(loremaster_model::Effect::Call(Id::new(r.id()))),
            EffectKind::Await(spec) => b.awaits(spec.clone()),
            EffectKind::Combat(spec) => b.combat(spec.clone()),
        };
        let i = then.offset + b.effects_mut().len().saturating_sub(1);
        let tokens = match &e.kind {
            EffectKind::Line(segments) => line_tokens(segments, &e.tokens),
            _ => e.tokens.clone(),
        };
        spans.add(then.key, format!("{}[{i}]", then.at), tokens);
        if let EffectKind::Line(segments) = &e.kind {
            segment_spans(spans, then.key, &format!("{}[{i}]", then.at), segments);
        }
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
    let then = Then {
        key: &key,
        at: "then",
        offset: 0,
    };
    effects(b, &r.effects, &then, spans);
    rust_behaviors(p, &r.effects);
    spans.add(&key, "", name_tokens(&r.id));
}

/// The behaviors of the `rust` blocks of a list of effects.
fn rust_behaviors(p: &mut Pack, list: &[Effect]) {
    for e in list {
        if let EffectKind::Rust(r) = &e.kind {
            p.behavior(r.id());
        }
    }
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
            b = b.say(line(say));
        }
        // The checks see each step as the rule that `assemble` makes of it
        // (`machine_rules`): its `then` is the new state, the `say`, then
        // the step's effects. `check` gives that rule its own file.
        let rule = step_key(&m.id.value, i);
        spans.add(&rule, "", step.tokens.clone());
        spans.add(&rule, "on", step.on.tokens.clone());
        if let Some(w) = &step.when {
            spans.add(&rule, "when", w.tokens.clone());
        }
        if let Some(say) = &step.say {
            spans.add(&rule, "then[1]", line_tokens(say, &step.say_tokens));
            segment_spans(spans, &rule, "then[1]", say);
        }
        let then = Then {
            key: &rule,
            at: "then",
            offset: 1 + usize::from(step.say.is_some()),
        };
        b = effects(b, &step.effects, &then, spans);
    }
    for n in &m.notes {
        b = if n.todo {
            b.todo(n.text.value.as_str())
        } else {
            b.note(n.text.value.as_str())
        };
    }
    if m.replace {
        b.replace();
    }
    for step in &m.steps {
        rust_behaviors(p, &step.effects);
    }
    if let Some((first, _)) = m.states.first() {
        spans.add(&key, format!("{at}.states"), name_tokens(first));
    }
    spans.add(&key, format!("{at}.start"), name_tokens(&m.start));
    spans.add(&key, "", name_tokens(&m.id));
}

/// The file of the rule that `assemble` makes of step `i` of a machine.
fn step_key(machine: &str, i: usize) -> String {
    format!("machine:{machine}#{}", i + 1)
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
        let list_at = format!("{at}.at.{n}");
        let then = Then {
            key: &key,
            at: &list_at,
            offset: 0,
        };
        b = effects(b, list, &then, spans);
    }
    for n in &t.notes {
        b = if n.todo {
            b.todo(n.text.value.as_str())
        } else {
            b.note(n.text.value.as_str())
        };
    }
    if t.replace {
        b.replace();
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
    if e.replace {
        b.replace();
    }
    spans.add(&key, "", name_tokens(&e.id));
}

/// A dialogue, as the expansion makes it: the builder, node by node, with
/// the outcomes of each node before its options. The spans come from the
/// token form: each place of dramaturge's tree has a path
/// (`nodes.stove.options[0].when`), and the checks name the same paths.
fn dialogue(p: &mut Pack, d: &Dialogue, spans: &mut Spans) {
    let key = format!("dialogue:{}", d.id.value);
    let tree = &d.tree.dialogue;
    let mut b = p
        .dialogue(d.id.value.as_str(), tree.with.as_str())
        .start(tree.start.iter());
    for node in &tree.nodes {
        let at = format!("nodes.{}", node.key);
        b = b.node(node.key.as_str(), line(&node.say.segments));
        segment_spans(spans, &key, &at, &node.say.segments);
        if let Some(s) = &node.speaker {
            b = b.speaker(s.as_str());
        }
        for (i, l) in node.lines.iter().enumerate() {
            b = b.line(l.speaker.as_str(), line(&l.say.segments));
            segment_spans(spans, &key, &format!("{at}.lines[{i}]"), &l.say.segments);
        }
        if let Some(next) = &node.next {
            b = b.next(next.as_key());
        }
        if let Some(w) = &node.when {
            spans.add(&key, format!("{at}.when"), w.tokens.clone());
            b = b.when(w.source.as_str());
        }
        b = outcomes(b, &node.outcomes, &key, &at, spans);
        for (o, option) in node.options.iter().enumerate() {
            let at = format!("{at}.options[{o}]");
            let say = option
                .say
                .as_ref()
                .map(|s| s.segments.as_slice())
                .unwrap_or(&[]);
            b = b.option(line(say), option.go.as_key());
            segment_spans(spans, &key, &at, say);
            if let Some(id) = &option.id {
                b = b.option_id(id.as_str());
            }
            if let Some(w) = &option.when {
                spans.add(&key, format!("{at}.when"), w.tokens.clone());
                b = b.when(w.source.as_str());
            }
            b = outcomes(b, &option.outcomes, &key, &at, spans);
        }
    }
    if d.replace {
        b.replace();
    }
    for (place, span) in &d.tree.spans {
        let path = place.path(tree);
        // The effects of `then` get their spans from `effects`, which
        // knows the index of each effect in the model.
        if matches!(
            place,
            Place::NodeWhen(_)
                | Place::OptionWhen(..)
                | Place::NodeOutcome(_, OutcomePlace::Then(_))
                | Place::OptionOutcome(_, _, OutcomePlace::Then(_))
        ) {
            continue;
        }
        spans.add(
            &key,
            path,
            proc_macro2::Ident::new("x", *span).into_token_stream(),
        );
    }
    spans.add(&key, "", name_tokens(&d.id));
}

/// The outcomes of a node or an option: dramaturge's `set`, `add`,
/// `relation` and `remember`, and loremaster's effects in `then`.
fn outcomes<'a>(
    mut b: DialogueBuilder<'a>,
    o: &Outcomes<Effect>,
    key: &str,
    at: &str,
    spans: &mut Spans,
) -> DialogueBuilder<'a> {
    for (name, v) in &o.set {
        b = b.set(name.as_str(), loremaster_model::Value::from(v));
    }
    for (name, n) in &o.add {
        b = b.add(name.as_str(), *n);
    }
    for (axis, n) in &o.relation {
        b = b.relation(axis.as_str(), *n);
    }
    for m in &o.remember {
        b = b.remember(m.as_str());
    }
    if !o.then.is_empty() {
        let then_at = format!("{at}.then");
        let then = Then {
            key,
            at: &then_at,
            offset: 0,
        };
        let list = effects(
            Vec::<loremaster_model::Effect>::new(),
            &o.then,
            &then,
            spans,
        );
        b = b.then_effects(list);
    }
    b
}
