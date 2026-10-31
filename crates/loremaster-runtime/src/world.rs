use dramaturge_model::{Condition, DialogueNodeId, SymbolTable};
use loremaster_lang::{
    CompiledDialogue, LangError, Scope, Segment, compile_condition, compile_dialogue,
    compile_markup,
};
use loremaster_model::{
    Effect, Fragment, FragmentMode, Id, Layer, Story, Value, VerbDef, standard_verbs,
};
use std::collections::BTreeMap;
use std::fmt;

/// A story with every condition and piece of markup compiled. Immutable
/// during play and shared by any number of games.
#[derive(Debug, Clone)]
pub struct World {
    pub story: Story,
    pub verbs: Vec<VerbDef>,
    pub(crate) intro: Vec<CFragment>,
    pub(crate) rooms: Vec<CRoom>,
    pub(crate) things: Vec<CThing>,
    pub(crate) rules: Vec<CRule>,
    /// Rule indexes in the order they run: more specific scope first, then
    /// higher priority, then file order.
    pub(crate) rule_order: Vec<usize>,
    pub(crate) timers: Vec<CTimer>,
    pub(crate) frames: BTreeMap<Id, CFrame>,
    pub(crate) dialogues: Vec<std::sync::Arc<CDialogue>>,
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
    pub index: usize,
    pub variants: Vec<Vec<Segment>>,
    /// The variants as written, for the editor.
    pub sources: Vec<String>,
    pub when: Option<Condition>,
    pub when_src: Option<String>,
    pub mode: FragmentMode,
    pub layer: Layer,
    pub replaces: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CRoom {
    pub description: Vec<CFragment>,
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
    pub description: Vec<CFragment>,
    pub presence: Vec<CFragment>,
    /// The condition of each name stage.
    pub names: Vec<Option<Condition>>,
    pub frames: BTreeMap<Id, CFrame>,
}

/// A compiled action frame.
#[derive(Debug, Clone, Default)]
pub(crate) struct CFrame {
    pub before: Vec<CFragment>,
    pub instead: Vec<CFragment>,
    pub after: Vec<CFragment>,
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

/// A dialogue lowered to a dramaturge graph, with its world effects compiled.
#[derive(Debug, Clone)]
pub(crate) struct CDialogue {
    pub id: Id,
    pub with: Id,
    pub graph: CompiledDialogue,
    /// The compiled `then` lists, by the index in their custom outcome.
    pub effects: Vec<Vec<CEffect>>,
    /// The node keys, in the order of the graph's nodes.
    pub keys: Vec<String>,
}

impl CDialogue {
    pub fn node(&self, id: DialogueNodeId) -> Option<&dramaturge_model::DialogueNode> {
        self.graph.graph.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_by_key(&self, key: &str) -> Option<&dramaturge_model::DialogueNode> {
        let i = self.keys.iter().position(|k| k == key)?;
        self.graph.graph.nodes.get(i)
    }

    pub fn key_of(&self, id: DialogueNodeId) -> Option<&str> {
        let i = self.graph.graph.nodes.iter().position(|n| n.id == id)?;
        self.keys.get(i).map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CEffect {
    Set(BTreeMap<String, Value>),
    Add(BTreeMap<String, i64>),
    Say(Vec<Segment>),
    Move(BTreeMap<Id, Id>),
    Goto(Id),
    End(Vec<Segment>),
    Stop,
    Emit(Id),
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

struct Compiler {
    scope: Scope,
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

    fn fragments(&self, owner: &str, list: &[Fragment]) -> Result<Vec<CFragment>, CompileError> {
        list.iter()
            .enumerate()
            .map(|(i, f)| {
                let at = format!("{owner}#{i}");
                Ok(CFragment {
                    key: at.clone(),
                    owner: owner.to_string(),
                    index: i,
                    variants: f
                        .variants
                        .iter()
                        .map(|v| self.markup(&at, v))
                        .collect::<Result<_, _>>()?,
                    sources: f.variants.clone(),
                    when: self.cond(&at, &f.when)?,
                    when_src: f.when.clone(),
                    mode: f.mode,
                    layer: f.layer,
                    replaces: f.replaces,
                })
            })
            .collect()
    }
}

impl Compiler {
    fn effect(&self, at: &str, e: &Effect) -> Result<CEffect, CompileError> {
        Ok(match e {
            Effect::Set(m) => CEffect::Set(m.clone()),
            Effect::Add(m) => CEffect::Add(m.clone()),
            Effect::Say(s) => CEffect::Say(self.markup(at, s)?),
            Effect::Move(m) => CEffect::Move(m.clone()),
            Effect::Goto(r) => CEffect::Goto(r.clone()),
            Effect::End(s) => CEffect::End(self.markup(at, s)?),
            Effect::Stop => CEffect::Stop,
            Effect::Emit(e) => CEffect::Emit(e.clone()),
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
        let frames = c.frames("story", &story.frames)?;
        let mut symbols = c.scope.symbols.clone();
        let dialogues = story
            .dialogues
            .iter()
            .map(|d| {
                let owner = format!("dialogue:{}", d.id);
                let graph =
                    compile_dialogue(d, &c.scope, &mut symbols).map_err(|e| CompileError {
                        at: format!("{owner}/{}", e.at),
                        error: e.error,
                    })?;
                let effects = graph
                    .effects
                    .iter()
                    .map(|list| list.iter().map(|e| c.effect(&owner, e)).collect())
                    .collect::<Result<_, CompileError>>()?;
                Ok(std::sync::Arc::new(CDialogue {
                    id: d.id.clone(),
                    with: d.with.clone(),
                    graph,
                    effects,
                    keys: d.nodes.iter().map(|n| n.key.clone()).collect(),
                }))
            })
            .collect::<Result<_, CompileError>>()?;
        let scope = c.scope.clone();
        let mut verbs = standard_verbs();
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
            frames,
            dialogues,
            symbols,
            room_index,
            thing_index,
        })
    }

    pub(crate) fn room_idx(&self, id: &str) -> Option<usize> {
        self.room_index.get(id).copied()
    }

    pub(crate) fn thing_idx(&self, id: &str) -> Option<usize> {
        self.thing_index.get(id).copied()
    }

    /// The dialogue of a thing.
    pub(crate) fn dialogue_with(&self, thing: &str) -> Option<&std::sync::Arc<CDialogue>> {
        self.dialogues.iter().find(|d| d.with.as_str() == thing)
    }

    pub(crate) fn dialogue(&self, id: &str) -> Option<&std::sync::Arc<CDialogue>> {
        self.dialogues.iter().find(|d| d.id.as_str() == id)
    }

    pub fn verb(&self, id: &str) -> Option<&VerbDef> {
        self.verbs.iter().find(|v| v.id.as_str() == id)
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
