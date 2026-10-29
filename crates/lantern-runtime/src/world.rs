use dramaturge_model::Condition;
use lantern_lang::{LangError, Scope, Segment, compile_condition, compile_markup};
use lantern_model::{
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
    pub(crate) frames: BTreeMap<Id, CFrame>,
    room_index: BTreeMap<Id, usize>,
    thing_index: BTreeMap<Id, usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct CFragment {
    pub key: String,
    pub owner: String,
    pub index: usize,
    pub variants: Vec<Vec<Segment>>,
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
}

#[derive(Debug, Clone)]
pub(crate) enum CEffect {
    Set(BTreeMap<String, Value>),
    Add(BTreeMap<String, i64>),
    Say(Vec<Segment>),
    Move(BTreeMap<Id, Id>),
    Goto(Id),
    End(Vec<Segment>),
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
    fn frames(
        &self,
        owner: &str,
        frames: &lantern_model::Frames,
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
        let rules = story
            .rules
            .iter()
            .map(|r| {
                let at = format!("rule:{}", r.id);
                Ok(CRule {
                    when: c.cond(&at, &r.when)?,
                    then: r
                        .then
                        .iter()
                        .map(|e| {
                            Ok(match e {
                                Effect::Set(m) => CEffect::Set(m.clone()),
                                Effect::Add(m) => CEffect::Add(m.clone()),
                                Effect::Say(s) => CEffect::Say(c.markup(&at, s)?),
                                Effect::Move(m) => CEffect::Move(m.clone()),
                                Effect::Goto(r) => CEffect::Goto(r.clone()),
                                Effect::End(s) => CEffect::End(c.markup(&at, s)?),
                            })
                        })
                        .collect::<Result<_, CompileError>>()?,
                })
            })
            .collect::<Result<_, CompileError>>()?;
        let frames = c.frames("story", &story.frames)?;
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
        Ok(Self {
            story,
            verbs,
            intro,
            rooms,
            things,
            rules,
            frames,
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

    pub fn verb(&self, id: &str) -> Option<&VerbDef> {
        self.verbs.iter().find(|v| v.id.as_str() == id)
    }
}
