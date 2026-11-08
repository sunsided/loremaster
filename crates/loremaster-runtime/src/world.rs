use crate::text::pool::{Leaf, LeafText, Lore, LorePool};
use dramaturge_model::{
    Candidate, Condition, DialogueGraph, DialogueNodeId, Entry, FragmentMode as DMode, Pick, Pools,
    SymbolTable,
};
use loremaster_lang::{
    CompiledLine, LangError, Scope, Segment, compile_condition, compile_dialogue, compile_markup,
};
use loremaster_model::{
    DialogueId, Effect, Fragment, FragmentMode, Id, Layer, Story, Value, VerbDef, standard_verbs,
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// A story with every condition and piece of markup compiled. Immutable
/// during play and shared by any number of games.
#[derive(Debug, Clone)]
pub struct World {
    pub story: Story,
    pub verbs: Vec<VerbDef>,
    pub(crate) intro: CText,
    pub(crate) rooms: Vec<CRoom>,
    pub(crate) things: Vec<CThing>,
    pub(crate) rules: Vec<CRule>,
    /// Rule indexes in the order they run: more specific scope first, then
    /// higher priority, then file order.
    pub(crate) rule_order: Vec<usize>,
    pub(crate) timers: Vec<CTimer>,
    /// The agendas, in the order of the story's agendas.
    pub(crate) agendas: Vec<CAgenda>,
    /// Registered behaviors that the story names, in story order.
    pub(crate) behaviors: Vec<std::sync::Arc<dyn crate::Behavior>>,
    /// The manifests of `behaviors`, in the same order.
    pub(crate) manifests: Vec<crate::Manifest>,
    /// Every registered behavior, also the ones that the story does not name.
    registered: Vec<std::sync::Arc<dyn crate::Behavior>>,
    pub(crate) frames: BTreeMap<Id, CFrame>,
    pub(crate) dialogues: Vec<std::sync::Arc<CompiledDialogue>>,
    /// The machines with `quest: true`, with the journal text of their states.
    pub(crate) quests: Vec<CQuest>,
    /// The named text pools of `texts/`.
    pub(crate) pools: Pools<Lore>,
    /// dramaturge ids of the player, things, memories, dialogue nodes and options.
    pub symbols: SymbolTable,
    room_index: BTreeMap<Id, usize>,
    thing_index: BTreeMap<Id, usize>,
    /// The names that conditions and markup can use, for checks in the editor.
    pub(crate) scope: Scope,
    /// The level of each room, in the order of the rooms.
    levels: Vec<i32>,
}

#[derive(Debug, Clone)]
pub(crate) struct CFragment {
    pub key: String,
    pub owner: String,
    pub variants: Vec<Vec<Segment>>,
    /// The variants as written, for the editor.
    pub sources: Vec<String>,
    pub when: Option<Condition>,
    pub when_src: Option<String>,
    /// The conditions of a group's candidates (`one_of`), for the views.
    pub inner_when: Vec<Condition>,
    pub mode: FragmentMode,
    pub layer: Layer,
    pub replaces: bool,
}

/// A list of fragments (a description, a presence text, a part of a frame)
/// and the pool that dramaturge's core chooses from. The pool id is the
/// owner (`room:shore`); each fragment is a candidate of a `pick: all` pool.
#[derive(Debug, Clone)]
pub(crate) struct CText {
    pub fragments: Vec<CFragment>,
    pub pool: LorePool,
}

impl Default for CText {
    fn default() -> Self {
        Self {
            fragments: Vec::new(),
            pool: LorePool::new("", Pick::All),
        }
    }
}

impl std::ops::Deref for CText {
    type Target = [CFragment];

    fn deref(&self) -> &[CFragment] {
        &self.fragments
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CRoom {
    pub description: CText,
    pub frames: BTreeMap<Id, CFrame>,
    pub exits: Vec<CExit>,
}

#[derive(Debug, Clone)]
pub(crate) struct CExit {
    pub visible: Option<Condition>,
    pub passable: Option<Condition>,
    pub blocked: Option<Vec<Segment>>,
    pub traversal: Option<Vec<Segment>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CThing {
    pub description: CText,
    pub presence: CText,
    /// The condition of each name stage.
    pub names: Vec<Option<Condition>>,
    pub frames: BTreeMap<Id, CFrame>,
}

/// A compiled action frame.
#[derive(Debug, Clone, Default)]
pub(crate) struct CFrame {
    pub before: CText,
    pub instead: CText,
    pub after: CText,
}

#[derive(Debug, Clone)]
pub(crate) struct CRule {
    pub when: Option<Condition>,
    pub then: Vec<CEffect>,
    pub within: Option<Within>,
}

/// What a rule belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Within {
    Thing(Id),
    Room(Id),
    Region(Id),
}

impl Within {
    /// Higher is more specific.
    fn rank(within: &Option<Within>) -> u8 {
        match within {
            Some(Within::Thing(_)) => 3,
            Some(Within::Room(_)) => 2,
            Some(Within::Region(_)) => 1,
            None => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CTimer {
    pub runs_while: Option<Condition>,
    pub at: BTreeMap<u32, Vec<CEffect>>,
}

/// A compiled agenda (D6).
#[derive(Debug, Clone)]
pub(crate) struct CAgenda {
    pub id: Id,
    pub actor: Id,
    pub steps: Vec<CStep>,
}

/// A compiled agenda step.
#[derive(Debug, Clone)]
pub(crate) struct CStep {
    /// The key in the save: the step's id, or its place from 1.
    pub key: String,
    /// The step has a time `at`; else it is due when the step before it is over.
    pub timed: bool,
    /// The minute (since midnight of day one) from which the step is due:
    /// the first time the clock reaches `at` at or after the time the step
    /// before it was due, or the story's start.
    pub due: u32,
    pub place: Option<Id>,
    pub when: Option<Condition>,
    pub action: Option<loremaster_model::CommandPattern>,
    pub then: Vec<CEffect>,
}

/// A quest machine, compiled: its title and the journal text per state, with
/// `done` for a finished state.
#[derive(Debug, Clone)]
pub(crate) struct CQuest {
    pub machine: Id,
    pub title: String,
    pub states: BTreeMap<String, (CompiledLine, bool)>,
}

/// A dialogue lowered to a dramaturge graph, with its world effects compiled.
#[derive(Debug, Clone)]
pub(crate) struct CompiledDialogue {
    pub id: Id,
    pub with: Id,
    /// The graph; a `then` list is a host outcome with the compiled effects,
    /// and a line is host text, the compiled line.
    pub graph: DialogueGraph<Arc<[CEffect]>, CompiledLine>,
    /// The node keys, in the order of the graph's nodes.
    pub keys: Vec<String>,
}

impl CompiledDialogue {
    pub fn node(&self, id: DialogueNodeId) -> Option<&CNode> {
        self.graph.node(id)
    }

    pub fn node_by_key(&self, key: &str) -> Option<&CNode> {
        let i = self.keys.iter().position(|k| k == key)?;
        self.graph.nodes.get(i)
    }

    pub fn key_of(&self, id: DialogueNodeId) -> Option<&str> {
        let i = self.graph.nodes.iter().position(|n| n.id == id)?;
        self.keys.get(i).map(String::as_str)
    }
}

/// A dialogue node with its compiled `then` lists.
pub(crate) type CNode = dramaturge_model::DialogueNode<Arc<[CEffect]>, CompiledLine>;

/// A dialogue option with its compiled `then` list and line.
pub(crate) type COption = dramaturge_model::DialogueOption<Arc<[CEffect]>, CompiledLine>;

#[derive(Debug, Clone)]
pub(crate) enum CEffect {
    Set(BTreeMap<String, Value>),
    Add(BTreeMap<String, i64>),
    Say(loremaster_lang::CompiledLine),
    /// `say: { text: <id> }`: show a named text pool.
    SayPool(Id),
    Move(BTreeMap<Id, Id>),
    Goto(Id),
    /// End the game with an ending: its id, title and text.
    End {
        ending: Id,
        title: Vec<Segment>,
        text: Vec<Segment>,
    },
    Stop,
    /// A pause for the host; the fire loop handles it, not `apply`.
    Await(Box<loremaster_model::AwaitSpec>),
    Emit(Id),
    Call(Id),
    Start(Vec<(Id, Option<u32>)>),
    Cancel(Id),
}

/// The first problem found while compiling a story.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub at: String,
    pub error: LangError,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.error.message)
    }
}

impl std::error::Error for CompileError {}

/// A compile error as a diagnostic, without a file yet: `Project::locate`
/// finds the file of the entity that `at` names.
impl From<&CompileError> for loremaster_model::Diagnostic {
    fn from(e: &CompileError) -> Self {
        loremaster_model::Diagnostic::error("", e.at.clone(), e.error.message.clone())
    }
}

/// The texts and conditions of the candidates of a group, nested groups too.
fn group_texts(
    one_of: &[loremaster_model::Candidate],
    texts: &mut Vec<String>,
    whens: &mut Vec<String>,
) {
    use loremaster_model::dramaturge_syntax::Variant;
    for c in one_of {
        whens.extend(c.when.iter().cloned());
        for v in &c.variants {
            match v {
                Variant::Text(t) => texts.push(t.clone()),
                Variant::Pool(p) => group_texts(&p.one_of, texts, whens),
                _ => {}
            }
        }
    }
}

/// Compiles a text of a candidate, with its place and its `when` as written.
type LeafFn<'a, T> = dyn Fn(&str, &T, Option<&String>) -> Result<Entry<Lore>, CompileError> + 'a;

struct Compiler {
    scope: Scope,
    endings: BTreeMap<Id, loremaster_model::Ending>,
}

impl Compiler {
    fn cond(&self, at: &str, src: &Option<String>) -> Result<Option<Condition>, CompileError> {
        src.as_deref()
            .map(|s| compile_condition(s, &self.scope))
            .transpose()
            .map_err(|error| CompileError {
                at: at.to_string(),
                error,
            })
    }

    fn markup(&self, at: &str, src: &str) -> Result<Vec<Segment>, CompileError> {
        compile_markup(src, &self.scope).map_err(|error| CompileError {
            at: at.to_string(),
            error,
        })
    }

    /// A line of a rule or a timer. Its speech has no speaker unless a
    /// segment names one.
    fn line(
        &self,
        at: &str,
        line: &loremaster_model::Line,
    ) -> Result<loremaster_lang::CompiledLine, CompileError> {
        loremaster_lang::compile_line(line, None, &self.scope).map_err(|e| CompileError {
            at: if e.at.is_empty() {
                at.to_string()
            } else {
                format!("{at}/say{}", e.at)
            },
            error: e.error,
        })
    }

    fn fragments(&self, owner: &str, list: &[Fragment]) -> Result<CText, CompileError> {
        // `first_visit` is the first visit of a room, for a room and the
        // things in it; for a thing that is examined, the intro and a frame
        // it means the first time the text shows.
        let rooms =
            (owner.starts_with("room:") && !owner.contains('/')) || owner.ends_with("/presence");
        let mut pool = LorePool::new(owner, Pick::All);
        let fragments = list
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let at = format!("{owner}#{i}");
                let when = self.cond(&at, &f.when)?;
                let leaf = |text| Leaf {
                    text,
                    layer: f.layer,
                    replaces: f.replaces,
                    fragment: i,
                    when_src: f.when.clone(),
                };
                let variants = if f.one_of.is_empty() {
                    f.variants
                        .iter()
                        .map(|v| Ok(Entry::Leaf(leaf(LeafText::Markup(self.markup(&at, v)?)))))
                        .collect::<Result<_, CompileError>>()?
                } else {
                    let index = i.to_string();
                    let id = f.id.as_ref().map_or(index.as_str(), Id::as_str);
                    let markup = |at: &str, text: &String, when: Option<&String>| {
                        Ok(Entry::Leaf(Leaf {
                            when_src: when.or(f.when.as_ref()).cloned(),
                            ..leaf(LeafText::Markup(self.markup(at, text)?))
                        }))
                    };
                    let pick = match f.pick {
                        loremaster_model::Pick::All => Pick::All,
                        loremaster_model::Pick::First => Pick::First,
                        loremaster_model::Pick::Best => Pick::Best,
                    };
                    vec![Entry::Pool(self.pool(&at, id, pick, &f.one_of, &markup)?)]
                };
                let mut candidate = Candidate::new(variants).mode(match f.mode {
                    FragmentMode::Always => DMode::Always,
                    FragmentMode::FirstVisit if rooms => DMode::FirstTime,
                    FragmentMode::FirstVisit | FragmentMode::Once => DMode::Once,
                    FragmentMode::Cycle => DMode::Cycle,
                    FragmentMode::Shuffle => DMode::Shuffle,
                    FragmentMode::Sticky => DMode::Sticky,
                });
                candidate.key = f.id.as_ref().map(|id| id.as_str().into());
                candidate.when = when.clone();
                candidate.learn = f.learn.iter().map(|v| v.to_string()).collect();
                pool.candidates.push(candidate);
                // A group shows the texts of its candidates: the views (the
                // things that a room text mentions, the matrix) read them all.
                let (mut sources, mut whens) = (f.variants.clone(), Vec::new());
                group_texts(&f.one_of, &mut sources, &mut whens);
                Ok(CFragment {
                    key: format!(
                        "{owner}/{}",
                        f.id.as_ref().map_or(i.to_string(), Id::to_string)
                    ),
                    owner: owner.to_string(),
                    variants: sources
                        .iter()
                        .map(|v| self.markup(&at, v))
                        .collect::<Result<_, _>>()?,
                    sources,
                    when,
                    when_src: f.when.clone(),
                    inner_when: whens
                        .iter()
                        .map(|w| compile_condition(w, &self.scope))
                        .collect::<Result<_, _>>()
                        .map_err(|error| CompileError {
                            at: at.clone(),
                            error,
                        })?,
                    mode: f.mode,
                    layer: f.layer,
                    replaces: f.replaces,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        Ok(CText { fragments, pool })
    }

    /// A pool of `one_of` candidates: a group in a description, or a named
    /// pool of `texts/`. `leaf` compiles a text of a candidate, with its place
    /// and its condition as written.
    fn pool<T>(
        &self,
        at: &str,
        id: &str,
        pick: Pick,
        one_of: &[loremaster_model::Candidate<T>],
        leaf: &LeafFn<'_, T>,
    ) -> Result<LorePool, CompileError> {
        use loremaster_model::dramaturge_syntax::{FragmentMode as SMode, Pick as SPick, Variant};
        let mut pool = LorePool::new(id, pick);
        for (i, c) in one_of.iter().enumerate() {
            let at = format!("{at}/one_of[{i}]");
            let mut variants = Vec::new();
            for v in &c.variants {
                variants.push(match v {
                    Variant::Text(text) => leaf(&at, text, c.when.as_ref())?,
                    Variant::Pool(inner) => {
                        let index = i.to_string();
                        let id = inner.id.as_deref().or(c.key.as_deref()).unwrap_or(&index);
                        let pick = match inner.pick.unwrap_or_default() {
                            SPick::All => Pick::All,
                            SPick::Best => Pick::Best,
                            _ => Pick::First,
                        };
                        Entry::Pool(self.pool(&at, id, pick, &inner.one_of, leaf)?)
                    }
                    // A kind of variant that a later dramaturge adds.
                    _ => continue,
                });
            }
            let mut candidate = Candidate::new(variants).mode(match c.mode {
                SMode::FirstTime => DMode::FirstTime,
                SMode::Once => DMode::Once,
                SMode::Cycle => DMode::Cycle,
                SMode::Shuffle => DMode::Shuffle,
                SMode::Sticky => DMode::Sticky,
                _ => DMode::Always,
            });
            candidate.key = c.key.as_deref().map(Into::into);
            candidate.speaker = c.speaker.as_deref().map(Id::new);
            candidate.topic = c.topic.as_deref().map(Id::new);
            candidate.when = self.cond(&format!("{at}.when"), &c.when)?;
            candidate.learn = c.learn.clone();
            pool.candidates.push(candidate);
        }
        Ok(pool)
    }

    /// The named pools of `texts/`. A line that says `{ text: <id> }` names
    /// another pool.
    fn pools(&self, story: &Story) -> Result<Pools<Lore>, CompileError> {
        let mut pools = Pools::new();
        for (n, p) in story.texts.iter().enumerate() {
            let id = p.id.clone().unwrap_or_default();
            let at = format!("text:{id}");
            let line = |at: &str, line: &loremaster_model::Line, when: Option<&String>| {
                if let Some(pool) = &line.pool {
                    return Ok(Entry::Named(pool.as_str().into()));
                }
                Ok(Entry::Leaf(Leaf {
                    text: LeafText::Line(self.line(at, line)?),
                    layer: Layer::Base,
                    replaces: false,
                    fragment: n,
                    when_src: when.cloned(),
                }))
            };
            let pick = match p.pick.unwrap_or_default() {
                loremaster_model::dramaturge_syntax::Pick::All => Pick::All,
                loremaster_model::dramaturge_syntax::Pick::Best => Pick::Best,
                _ => Pick::First,
            };
            pools.insert(self.pool(&at, &id, pick, &p.one_of, &line)?);
        }
        Ok(pools)
    }
}

impl Compiler {
    fn effect(&self, at: &str, e: &Effect) -> Result<CEffect, CompileError> {
        Ok(match e {
            Effect::Await(spec) => {
                let shape = if spec.kinds() != 1 {
                    Some(
                        "an `await` asks for exactly one of `choice`, `roll`, `walk_to` or `action`",
                    )
                } else if spec.action.is_none()
                    && (!spec.args.is_empty() || !spec.results.is_empty())
                {
                    Some("`args` and `results` belong to an `action`")
                } else {
                    None
                };
                if let Some(message) = shape {
                    return Err(CompileError {
                        at: at.to_string(),
                        error: loremaster_lang::LangError::new(message, 0..0),
                    });
                }
                if spec
                    .roll
                    .as_deref()
                    .is_some_and(|d| loremaster_model::Dice::parse(d).is_none())
                {
                    return Err(CompileError {
                        at: at.to_string(),
                        error: loremaster_lang::LangError::new("dice such as `2d6` expected", 0..0),
                    });
                }
                CEffect::Await(spec.clone())
            }
            Effect::Set(m) => CEffect::Set(m.clone()),
            Effect::Add(m) => CEffect::Add(m.clone()),
            Effect::Say(line) => match &line.pool {
                Some(pool) => CEffect::SayPool(pool.clone()),
                None => CEffect::Say(self.line(at, line)?),
            },
            Effect::Move(m) => CEffect::Move(m.clone()),
            Effect::Goto(r) => CEffect::Goto(r.clone()),
            Effect::End(id) => {
                let Some(ending) = self.endings.get(id) else {
                    return Err(CompileError {
                        at: at.to_string(),
                        error: loremaster_lang::LangError::new(
                            format!("unknown ending `{id}`"),
                            0..id.as_str().len(),
                        ),
                    });
                };
                CEffect::End {
                    ending: id.clone(),
                    title: self.markup(&format!("ending:{id}/title"), &ending.title)?,
                    text: self.markup(&format!("ending:{id}/text"), &ending.text)?,
                }
            }
            Effect::Stop => CEffect::Stop,
            Effect::Emit(e) => CEffect::Emit(e.clone()),
            Effect::Call(b) => CEffect::Call(b.clone()),
            Effect::Start(t) => CEffect::Start(t.entries()),
            Effect::Cancel(t) => CEffect::Cancel(t.clone()),
        })
    }

    fn frames(
        &self,
        owner: &str,
        frames: &loremaster_model::Frames,
    ) -> Result<BTreeMap<Id, CFrame>, CompileError> {
        frames
            .iter()
            .map(|(verb, f)| {
                let at = format!("{owner}/frames/{verb}");
                Ok((
                    verb.clone(),
                    CFrame {
                        before: self.fragments(&format!("{at}/before"), &f.before)?,
                        instead: self.fragments(&format!("{at}/instead"), &f.instead)?,
                        after: self.fragments(&format!("{at}/after"), &f.after)?,
                    },
                ))
            })
            .collect()
    }
}

impl World {
    pub fn compile(story: Story) -> Result<Self, CompileError> {
        let c = Compiler {
            scope: Scope::from_story(&story),
            endings: story
                .endings
                .iter()
                .map(|e| (e.id.clone(), e.clone()))
                .collect(),
        };
        let intro = c.fragments("story:intro", &story.intro)?;
        let rooms = story
            .rooms
            .iter()
            .map(|room| {
                let owner = format!("room:{}", room.id);
                Ok(CRoom {
                    description: c.fragments(&owner, &room.description)?,
                    frames: c.frames(&owner, &room.frames)?,
                    exits: room
                        .exits
                        .iter()
                        .enumerate()
                        .map(|(i, e)| {
                            let at = format!("{owner}/exits[{i}]");
                            Ok(CExit {
                                visible: c.cond(&at, &e.visible)?,
                                passable: c.cond(&at, &e.passable)?,
                                blocked: e
                                    .blocked
                                    .as_deref()
                                    .map(|s| c.markup(&at, s))
                                    .transpose()?,
                                traversal: e
                                    .traversal
                                    .as_deref()
                                    .map(|s| c.markup(&at, s))
                                    .transpose()?,
                            })
                        })
                        .collect::<Result<_, CompileError>>()?,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        let things = story
            .things
            .iter()
            .map(|t| {
                let owner = format!("thing:{}", t.id);
                Ok(CThing {
                    description: c.fragments(&owner, &t.description)?,
                    presence: c.fragments(&format!("{owner}/presence"), &t.presence)?,
                    names: t
                        .names
                        .iter()
                        .enumerate()
                        .map(|(i, n)| c.cond(&format!("{owner}/names[{i}]"), &n.when))
                        .collect::<Result<_, _>>()?,
                    frames: c.frames(&owner, &t.frames)?,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        let rules: Vec<CRule> = story
            .rules
            .iter()
            .map(|r| {
                let at = format!("rule:{}", r.id);
                let within = r.within.as_ref().map(|w| {
                    if story.thing(w.as_str()).is_some() {
                        Within::Thing(w.clone())
                    } else if story.room(w.as_str()).is_some() {
                        Within::Room(w.clone())
                    } else {
                        Within::Region(w.clone())
                    }
                });
                Ok(CRule {
                    within,
                    when: c.cond(&at, &r.when)?,
                    then: r
                        .then
                        .iter()
                        .map(|e| c.effect(&at, e))
                        .collect::<Result<_, CompileError>>()?,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        let mut rule_order: Vec<usize> = (0..story.rules.len()).collect();
        rule_order.sort_by_key(|&i| {
            let rank: &Option<Within> = &rules[i].within;
            (
                std::cmp::Reverse(Within::rank(rank)),
                std::cmp::Reverse(story.rules[i].priority),
                i,
            )
        });
        let timers = story
            .timers
            .iter()
            .map(|t| {
                let at = format!("timer:{}", t.id);
                Ok(CTimer {
                    runs_while: c.cond(&at, &t.runs_while)?,
                    at: t
                        .at
                        .iter()
                        .map(|(n, effects)| {
                            let list = effects.iter().map(|e| c.effect(&at, e)).collect::<Result<
                                Vec<_>,
                                CompileError,
                            >>(
                            )?;
                            Ok((*n, list))
                        })
                        .collect::<Result<_, CompileError>>()?,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        let agendas = story
            .agendas
            .iter()
            .map(|a| {
                let at = format!("agenda:{}", a.id);
                let mut due = u32::from(story.clock.start.0);
                let steps = a
                    .keyed_steps()
                    .map(|(key, step)| {
                        if let Some(t) = step.at {
                            due = next_at(due, t);
                        }
                        let at = format!("{at}/steps/{key}");
                        Ok(CStep {
                            key,
                            timed: step.at.is_some(),
                            due,
                            place: step.place.clone(),
                            when: c.cond(&at, &step.when)?,
                            action: step.action.clone(),
                            then: step
                                .then
                                .iter()
                                .map(|e| c.effect(&at, e))
                                .collect::<Result<_, CompileError>>()?,
                        })
                    })
                    .collect::<Result<_, CompileError>>()?;
                Ok(CAgenda {
                    id: a.id.clone(),
                    actor: a.actor.clone(),
                    steps,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        let frames = c.frames("story", &story.frames)?;
        let pools = c.pools(&story)?;
        let mut symbols = c.scope.symbols.clone();
        let dialogues = story
            .dialogues
            .iter()
            .map(|d| {
                let id = Id::new(d.id_str());
                let owner = format!("dialogue:{id}");
                let lowered =
                    compile_dialogue(d, &c.scope, &mut symbols).map_err(|e| CompileError {
                        at: format!("{owner}/{}", e.at),
                        error: e.error,
                    })?;
                let mut failed = None;
                let graph = lowered.map_host(|then| {
                    then.iter()
                        .map(|e| c.effect(&owner, e))
                        .collect::<Result<_, _>>()
                        .unwrap_or_else(|e| {
                            failed.get_or_insert(e);
                            Arc::from([])
                        })
                });
                if let Some(e) = failed {
                    return Err(e);
                }
                Ok(Arc::new(CompiledDialogue {
                    id,
                    with: Id::new(d.with.as_str()),
                    graph,
                    keys: d.nodes.iter().map(|n| n.key.clone()).collect(),
                }))
            })
            .collect::<Result<_, CompileError>>()?;
        let scope = c.scope.clone();
        let mut verbs = standard_verbs();
        if story.score.is_some() {
            verbs.push(VerbDef {
                id: Id::new("score"),
                label: "Score".into(),
                words: vec!["score".into()],
                slots: loremaster_model::VerbSlots::None,
                preposition: None,
                bar: false,
                default: None,
            });
        }
        verbs.extend(story.verbs.iter().cloned());
        let room_index = story
            .rooms
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        let thing_index = story
            .things
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), i))
            .collect();
        let quests = story
            .machines
            .iter()
            .filter(|m| m.quest)
            .map(|m| {
                let states = (m.states.iter())
                    .filter_map(|s| Some((s.name(), s.journal()?, s.done())))
                    .map(|(name, line, done)| {
                        let at = format!("machine:{}/states/{name}/journal", m.id);
                        Ok((name.to_string(), (c.line(&at, line)?, done)))
                    })
                    .collect::<Result<_, CompileError>>()?;
                Ok(CQuest {
                    machine: m.id.clone(),
                    title: m.title.clone().unwrap_or_else(|| m.id.to_string()),
                    states,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        let levels = levels(&story);
        Ok(Self {
            levels,
            scope,
            story,
            verbs,
            intro,
            rooms,
            things,
            rules,
            rule_order,
            timers,
            agendas,
            behaviors: Vec::new(),
            manifests: Vec::new(),
            registered: Vec::new(),
            frames,
            dialogues,
            pools,
            quests,
            symbols,
            room_index,
            thing_index,
        })
    }

    /// Add a behavior. It runs only if story.yaml names its id under `behaviors:`.
    pub fn register(&mut self, behavior: impl crate::Behavior + 'static) -> &mut Self {
        self.registered.push(std::sync::Arc::new(behavior));
        self.behaviors = self
            .story
            .behaviors
            .iter()
            .filter_map(|id| {
                self.registered
                    .iter()
                    .find(|b| b.id() == id.as_str())
                    .cloned()
            })
            .collect();
        self.manifests = self.behaviors.iter().map(|b| b.manifest()).collect();
        self
    }

    /// Is a behavior with this id registered, named by the story or not?
    pub fn is_registered(&self, id: &str) -> bool {
        self.registered.iter().any(|b| b.id() == id)
    }

    /// The manifest of a behavior, as `register` took it.
    pub fn manifest(&self, id: &str) -> Option<&crate::Manifest> {
        self.behaviors
            .iter()
            .position(|b| b.id() == id)
            .and_then(|i| self.manifests.get(i))
    }

    /// Does a behavior of the story react to the passing of time (turn or
    /// time events)? An opaque one may.
    pub fn behaviors_watch_time(&self) -> bool {
        self.manifests
            .iter()
            .any(|m| m.handles(crate::EventKind::Turn) || m.handles(crate::EventKind::Time))
    }

    /// The behaviors that the story names and that this build does not have.
    /// Their logic does not run.
    pub fn missing_behaviors(&self) -> Vec<&Id> {
        self.story
            .behaviors
            .iter()
            .filter(|id| !self.behaviors.iter().any(|b| b.id() == id.as_str()))
            .collect()
    }

    /// The behavior with this id, if the story names it and this build has it.
    pub fn behavior(&self, id: &str) -> Option<&dyn crate::Behavior> {
        self.behaviors.iter().find(|b| b.id() == id).map(|b| &**b)
    }

    pub(crate) fn room_idx(&self, id: &str) -> Option<usize> {
        self.room_index.get(id).copied()
    }

    pub(crate) fn thing_idx(&self, id: &str) -> Option<usize> {
        self.thing_index.get(id).copied()
    }

    /// The story's thing with this id, by index instead of a linear search.
    pub(crate) fn thing(&self, id: &str) -> Option<&loremaster_model::Thing> {
        self.thing_index
            .get(id)
            .and_then(|&i| self.story.things.get(i))
    }

    /// The story's room with this id, by index instead of a linear search.
    pub(crate) fn room(&self, id: &str) -> Option<&loremaster_model::Room> {
        self.room_index
            .get(id)
            .and_then(|&i| self.story.rooms.get(i))
    }

    /// The dialogue of a thing.
    pub(crate) fn dialogue_with(&self, thing: &str) -> Option<&std::sync::Arc<CompiledDialogue>> {
        self.dialogues.iter().find(|d| d.with.as_str() == thing)
    }

    pub(crate) fn dialogue(&self, id: &str) -> Option<&std::sync::Arc<CompiledDialogue>> {
        self.dialogues.iter().find(|d| d.id.as_str() == id)
    }

    pub fn verb(&self, id: &str) -> Option<&VerbDef> {
        self.verbs.iter().find(|v| v.id.as_str() == id)
    }
}

/// The first minute at or after `from` whose time of day is `at`.
fn next_at(from: u32, at: loremaster_model::TimeOfDay) -> u32 {
    let day = loremaster_model::TimeOfDay::MINUTES_PER_DAY;
    let today = from - from % day + u32::from(at.0);
    if today >= from { today } else { today + day }
}

impl World {
    /// The first move on the shortest way from one room to another, along
    /// exits that lead somewhere: the direction and the next room. Doors and
    /// conditions on exits do not stop an actor. None when there is no way,
    /// or the rooms are the same.
    pub(crate) fn route(&self, from: &str, to: &str) -> Option<(String, Id)> {
        let start = self.room_idx(from)?;
        let goal = self.room_idx(to)?;
        if start == goal {
            return None;
        }
        // For each room reached, the first move from `start` towards it.
        let mut first: Vec<Option<(String, Id)>> = vec![None; self.story.rooms.len()];
        let mut seen = vec![false; self.story.rooms.len()];
        seen[start] = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            for exit in &self.story.rooms[i].exits {
                let Some(j) = exit.to.as_ref().and_then(|t| self.room_idx(t.as_str())) else {
                    continue;
                };
                if seen[j] {
                    continue;
                }
                seen[j] = true;
                first[j] = if i == start {
                    Some((
                        exit.direction.name().to_string(),
                        self.story.rooms[j].id.clone(),
                    ))
                } else {
                    first[i].clone()
                };
                if j == goal {
                    return first[j].clone();
                }
                queue.push_back(j);
            }
        }
        None
    }
}

/// The level of each room: the `level` field, else counted from the start room (up is +1, down is −1).
/// Rooms with a `level` field fix the count; rooms that the walk does not reach are on level 0.
fn levels(story: &Story) -> Vec<i32> {
    use loremaster_model::Direction;
    let index: BTreeMap<&Id, usize> = story
        .rooms
        .iter()
        .enumerate()
        .map(|(i, r)| (&r.id, i))
        .collect();
    let mut level: Vec<Option<i32>> = story.rooms.iter().map(|r| r.level).collect();
    let mut queue: std::collections::VecDeque<usize> = (0..story.rooms.len())
        .filter(|&i| level[i].is_some())
        .collect();
    if let Some(&start) = index.get(&story.start)
        && level[start].is_none()
    {
        level[start] = Some(0);
        queue.push_back(start);
    }
    while let Some(i) = queue.pop_front() {
        let here = level[i].unwrap_or(0);
        for exit in &story.rooms[i].exits {
            let Some(&j) = exit.to.as_ref().and_then(|to| index.get(to)) else {
                continue;
            };
            if level[j].is_some() {
                continue;
            }
            level[j] = Some(match exit.direction {
                Direction::Up => here + 1,
                Direction::Down => here - 1,
                _ => here,
            });
            queue.push_back(j);
        }
    }
    level.into_iter().map(|l| l.unwrap_or(0)).collect()
}

impl World {
    /// The level of a room. Up exits go one level higher.
    pub fn level_of(&self, room: &str) -> i32 {
        self.room_idx(room).map_or(0, |i| self.levels[i])
    }
}
