//! The syntax tree of a `world!` block. Each node keeps the tokens it came
//! from, so that the checks can point at them.

use proc_macro2::{Span, TokenStream};

/// An id or a word, with the place it was written.
#[derive(Clone)]
pub struct Name {
    pub value: String,
    pub span: Span,
}

/// A string literal, with the place it was written.
#[derive(Clone)]
pub struct Text {
    pub value: String,
    pub span: Span,
}

/// A condition, as text for the model and as tokens for the error span.
#[derive(Clone)]
pub struct Cond {
    pub source: String,
    pub tokens: TokenStream,
}

/// A value of a variable, a property or a fact.
#[derive(Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    /// An enum value or a text.
    Text(String),
}

pub struct World {
    /// `pack <name>;`: the name of the function, `pack` without it.
    pub name: Option<Name>,
    pub items: Vec<Item>,
}

pub enum Item {
    Extern(Vec<Extern>),
    Story(Story),
    Var(Var),
    Room(Room),
    Thing(Thing),
    Rule(Rule),
    Machine(Machine),
    Timer(Timer),
    Ending(Ending),
    Verb(Verb),
    Test(Test),
    Dialogue(Dialogue),
    Thread(Thread),
    /// `behavior <id>;`: a behavior in Rust, by id.
    Behavior(Name),
    /// `behavior <expr>;`: a behavior in Rust with its instance, for
    /// example `behavior Thief::zork();`. The pack keeps the instance.
    Register(syn::Expr),
}

/// An id from another pack, for the checks at compile time.
pub struct Extern {
    pub kind: ExternKind,
    pub id: Name,
    pub tokens: TokenStream,
}

pub enum ExternKind {
    Room,
    /// The thing's properties, when given: conditions read them.
    Thing(Vec<(Name, PropType)>),
    Var(VarType),
    /// The machine's states, when given: conditions compare it with them.
    Machine(Vec<Name>),
    Timer,
    Ending,
    Verb,
    Behavior,
    /// A relation axis of the story header.
    Relation,
}

/// The type of an extern thing's property.
#[derive(Clone, Copy)]
pub enum PropType {
    Bool,
    Int,
    Text,
}

pub struct Story {
    pub title: Text,
    pub start: Option<Name>,
    pub author: Option<Text>,
    pub seed: Option<u64>,
    pub intro: Vec<Frag>,
    pub death_prompt: Option<bool>,
    pub clock_start: Option<Text>,
    pub minutes_per_turn: Option<u16>,
    pub phases: Option<Vec<(Name, Text)>>,
    pub relations: Vec<Name>,
    pub score: Option<i64>,
    pub score_var: Option<Name>,
    pub ranks: Vec<(i64, Text)>,
    pub tokens: TokenStream,
}

#[derive(Clone)]
pub enum VarType {
    Bool,
    Int,
    Enum(Vec<Name>),
}

pub struct Var {
    pub id: Name,
    pub ty: VarType,
    pub initial: Value,
    /// `min <n|var>` and `max <n|var>`: the number or the variable id as written.
    pub min: Option<Name>,
    pub max: Option<Name>,
}

/// A fragment of text: `[ambient] [cycle] [when <cond> =>] "text"` or a
/// list of variants.
#[derive(Clone)]
pub struct Frag {
    pub layer: Option<Name>,
    pub mode: Option<Name>,
    pub when: Option<Cond>,
    pub texts: Vec<Text>,
    /// `[..]`: the texts are variants.
    pub variants: bool,
    /// `id <name>` after the text.
    pub id: Option<Name>,
    /// `replaces` after the text.
    pub replaces: bool,
    /// `note "…"` after the text.
    pub notes: Vec<Text>,
    pub tokens: TokenStream,
}

/// `before <verb> <frag>`, `instead ..` or `after ..`: text around the
/// response to a verb.
pub struct Frame {
    pub when: FrameWhen,
    pub verb: Name,
    pub frag: Frag,
}

#[derive(Clone, Copy)]
pub enum FrameWhen {
    Before,
    Instead,
    After,
}

pub struct Room {
    /// `replace room ..`: it replaces the room of an earlier pack.
    pub replace: bool,
    pub id: Name,
    pub name: Text,
    pub parts: Vec<RoomPart>,
}

pub enum RoomPart {
    At(f32, f32),
    Text(Frag),
    Exit(Exit),
    Tag(Text),
    Region(Name),
    Level(i32),
    Note(Note),
    Frame(Frame),
}

pub struct Exit {
    pub direction: Name,
    pub to: Option<Name>,
    pub door: Option<Name>,
    pub visible: Option<Cond>,
    pub passable: Option<Cond>,
    pub blocked: Option<Text>,
    pub traversal: Option<Text>,
    pub tokens: TokenStream,
}

pub struct Note {
    pub todo: bool,
    pub text: Text,
}

pub struct Thing {
    pub replace: bool,
    pub id: Name,
    pub name: Text,
    pub location: Option<Location>,
    pub parts: Vec<ThingPart>,
}

pub struct Location {
    pub kind: LocationKind,
    pub tokens: TokenStream,
}

pub enum LocationKind {
    /// `in <room>`.
    Room(Name),
    /// `inside <container>`.
    Inside(Name),
    /// `upon <supporter>`.
    Upon(Name),
    Carried,
    Nowhere,
}

pub enum ThingPart {
    Aliases(Vec<Text>),
    Portable,
    Container,
    Supporter,
    Unlisted,
    Proper,
    Capacity(u32),
    Key(Name),
    Prop(Name, Value),
    Examine(Frag),
    Presence(Frag),
    AlsoIn(Name),
    Note(Note),
    Frame(Frame),
    /// `name [when <cond> =>] "text" [aliases [..]] [proper]`: a stage of
    /// the name.
    Name(NameStage),
}

pub struct NameStage {
    pub when: Option<Cond>,
    pub text: Text,
    pub aliases: Vec<Text>,
    pub proper: bool,
    pub tokens: TokenStream,
}

pub struct Event {
    pub kind: EventKind,
    pub tokens: TokenStream,
}

pub enum EventKind {
    Start,
    Turn,
    Enter(Name),
    Leave(Name),
    Time(Text),
    Verb(Vec<Name>),
    Check(Vec<Name>),
    Timer(Name),
    Event(Name),
    Talk(Vec<Name>),
}

pub struct Rule {
    pub id: Name,
    pub on: Event,
    pub instead: bool,
    pub once: bool,
    pub priority: Option<i32>,
    pub within: Option<Name>,
    pub when: Option<Cond>,
    pub effects: Vec<Effect>,
}

pub struct Effect {
    pub kind: EffectKind,
    pub tokens: TokenStream,
}

/// One segment of a line: narration, `narrate` or speech, maybe with a
/// condition.
#[derive(Clone)]
pub struct Segment {
    pub voice: SegmentVoice,
    pub text: Text,
    pub when: Option<Cond>,
    pub tokens: TokenStream,
}

#[derive(Clone)]
pub enum SegmentVoice {
    /// A bare string.
    Plain,
    /// `narrate "…"`.
    Narrate,
    /// `says "…"`, or `<speaker> says "…"`.
    Says(Option<Name>),
}

pub enum EffectKind {
    /// `line <segments>`.
    Line(Vec<Segment>),
    Set(Vec<(Name, Value)>),
    Add(Vec<(Name, i64)>),
    /// `calc total = r + skill`: the target and the sum as text.
    Calc(Name, String),
    /// The thing and the target word: a room, `in:box`, `on:table`,
    /// `carried` or `nowhere`.
    Move(Vec<(Name, String)>),
    Goto(Name),
    End(Name),
    Stop,
    Emit(Name),
    Start(Name, Option<u32>),
    Cancel(Name),
    Call(Name),
    /// `rust |cx| { .. }`: Rust code at this place of a rule.
    Rust(RustBlock),
}

/// A `rust |cx| { .. }` block. The macro makes the behavior
/// `<owner>#rust` of it, and the effect `call: <owner>#rust`.
pub struct RustBlock {
    /// The rule: its id, or the rule of a machine step (`lock#1`). The
    /// parser of the rule or machine sets it.
    pub owner: String,
    pub cx: proc_macro2::Ident,
    pub body: syn::Block,
}

impl RustBlock {
    pub fn id(&self) -> String {
        format!("{}#rust", self.owner)
    }
}

pub struct Machine {
    pub replace: bool,
    pub id: Name,
    pub start: Name,
    pub states: Vec<(Name, Option<Text>)>,
    pub steps: Vec<Step>,
    pub notes: Vec<Note>,
}

pub struct Step {
    pub from: Vec<Name>,
    /// `[a, b] -> c`: a list, as `from: [a, b]` in YAML.
    pub from_list: bool,
    pub to: Name,
    pub on: Event,
    pub instead: bool,
    pub when: Option<Cond>,
    /// A first `line` is the step's `say`; the rest are its effects.
    pub say: Option<Vec<Segment>>,
    /// The tokens of the step's first `line`, for the spans of its `say`.
    pub say_tokens: TokenStream,
    pub effects: Vec<Effect>,
    pub tokens: TokenStream,
}

pub struct Timer {
    pub replace: bool,
    pub id: Name,
    pub turns: u32,
    pub running: bool,
    pub repeat: bool,
    pub runs_while: Option<Cond>,
    /// Effects by the value of the timer; 0 is when it runs out.
    pub at: Vec<(u32, Vec<Effect>)>,
    pub notes: Vec<Note>,
}

pub struct Ending {
    pub replace: bool,
    pub id: Name,
    pub title: Text,
    pub kind: Name,
    pub rank: Option<u32>,
    pub text: Option<Text>,
    pub notes: Vec<Note>,
}

pub struct Verb {
    pub replace: bool,
    pub id: Name,
    pub label: Text,
    pub words: Vec<Text>,
    pub slots: Option<Name>,
    pub preposition: Option<Text>,
    pub no_bar: bool,
    pub default: Option<Text>,
}

pub struct Test {
    pub id: Name,
    pub name: Text,
    pub pending: Option<(u8, Text)>,
    /// `seed 42;`: the seed of the RNG for this test.
    pub seed: Option<u64>,
    pub steps: Vec<TestStep>,
    pub expect: Vec<Expectation>,
}

pub struct TestStep {
    pub command: Text,
    /// `wait * 16`: the step 16 times.
    pub times: u32,
    pub checks: Vec<Expectation>,
    /// `rolls 4, 11`: the totals of the rolls in the step.
    pub rolls: Vec<i64>,
}

pub enum Expectation {
    Sees(Vec<Text>),
    NotSees(Vec<Text>),
    Fact(Name, Value),
    Room(Name),
    Ending(Name),
    Ended(bool),
}

/// A line in a dialogue: its segments.
#[derive(Clone)]
pub struct DialogueLine {
    pub segments: Vec<Segment>,
}

/// The dialogue tree of dramaturge's token form, with loremaster's
/// conditions, lines and effects, and the span of each place.
pub type DialogueTree = dramaturge_syntax::tokens::Spanned<Cond, DialogueLine, Effect, Note>;

/// `dialogue [<id>] with <thing> start [..] { .. }`.
pub struct Dialogue {
    pub replace: bool,
    /// The id, or the thing's id when the block names none.
    pub id: Name,
    pub tree: DialogueTree,
}

/// `thread <id> "<name>" { .. }`: a promise to the reader.
pub struct Thread {
    pub replace: bool,
    pub id: Name,
    pub name: Text,
    pub status: Option<Name>,
    pub summary: Option<Text>,
    pub tracks: Vec<Name>,
    pub notes: Vec<Note>,
}
