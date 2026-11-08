use crate::{
    Agenda, Asset, ClockConfig, Dialogue, DialogueId, Ending, Fragment, Frames, Id, Machine, Note,
    Room, Rule, Thing, Thread, Timer, Value, VerbDef,
};
use serde::{Deserialize, Serialize};

/// The type of a story variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarType {
    Bool,
    Int,
    Enum(Vec<String>),
}

/// A declared story variable such as `bridge_collapsed` or `weather`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VarDef {
    pub id: Id,
    #[serde(rename = "type")]
    pub ty: VarType,
    pub initial: Value,
    /// The lowest value of an `int` variable. The engine clamps each write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Limit>,
    /// The highest value of an `int` variable. The engine clamps each write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Limit>,
}

/// A limit of an `int` variable: a number, or the id of another `int`
/// variable, for example `max: stamina_initial`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Limit {
    Value(i64),
    Var(Id),
}

impl From<i64> for Limit {
    fn from(n: i64) -> Self {
        Limit::Value(n)
    }
}

impl From<&str> for Limit {
    fn from(var: &str) -> Self {
        Limit::Var(Id::new(var))
    }
}

/// A line of the adventure sheet (`stats:` in story.yaml). A number shows as
/// a value and a true flag shows as a codeword.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stat {
    /// The variable, or a property such as `codeword.hearth`.
    pub var: String,
    pub label: String,
    /// The short name in the one-line form of the sheet, for example `ST`.
    /// A stat without it does not show in that line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// The value to show after the slash, for a number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Limit>,
}

/// The score of the story: a number variable, its maximum and the ranks.
/// With a score, the `score` command shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    /// The number variable that holds the score. The loader declares it when
    /// story.yaml does not.
    #[serde(default = "score_var")]
    pub var: Id,
    pub max: i64,
    /// Rank names, each from a score up, for example `{ from: 26, name: Amateur Adventurer }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranks: Vec<Rank>,
}

fn score_var() -> Id {
    Id::new("score")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    pub from: i64,
    pub name: String,
}

impl Score {
    /// The name of the rank for a score: the rank with the highest `from` that the score reaches.
    pub fn rank(&self, score: i64) -> Option<&str> {
        self.ranks
            .iter()
            .filter(|r| r.from <= score)
            .max_by_key(|r| r.from)
            .map(|r| r.name.as_str())
    }
}

/// A whole story: everything a project folder holds, and what the bundle carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub start: Id,
    /// Printed before the first room description.
    #[serde(default)]
    pub intro: Vec<Fragment>,
    #[serde(default)]
    pub clock: ClockConfig,
    #[serde(default)]
    pub vars: Vec<VarDef>,
    /// Custom verbs in addition to the standard library.
    #[serde(default)]
    pub verbs: Vec<VerbDef>,
    #[serde(default)]
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub things: Vec<Thing>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<Score>,
    /// Puzzles with named states. The loader also makes their variables and rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub machines: Vec<Machine>,
    /// Behaviors in Rust that the game registers, by id. A build without one
    /// of them plays the story without its logic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Id>,
    /// The ways the story can end, from `endings/`. The `end` effect names one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endings: Vec<Ending>,
    /// An ending of kind `death` asks the player to undo, restart or restore
    /// (SE-9). `death_prompt: false` in story.yaml turns the prompt off.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub death_prompt: bool,
    /// How many things the player can hold in the hands (WO-3). Things
    /// inside a carried container, and concealed things, do not count.
    /// Without it, any number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carry_limit: Option<u32>,
    /// The actors the player steers (WO-5), each a thing; the first is the
    /// active one at the start. Empty: the player is the active actor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub playable: Vec<Id>,
    /// The most `on: next` machine steps in one turn. Without it, 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_limit: Option<u32>,
    /// The adventure sheet of the player. Without it, the player shows no sheet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stats: Vec<Stat>,
    /// Counters that go down each turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timers: Vec<Timer>,
    /// What actors do on their own, from `agendas/` (D6). Each agenda's
    /// actor is a thing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agendas: Vec<Agenda>,
    /// Relation axes between the player and the characters, such as `trust`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Id>,
    /// The media files of the project, from `assets/` (ME-3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<Asset>,
    /// Conversations with things, from `dialogues/`.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "crate::dialogue::story_form"
    )]
    pub dialogues: Vec<Dialogue>,
    /// Named text pools, from `texts/`. Each has its file name as id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<crate::TextPool>,
    /// Author text around the responses to verbs, for the whole story.
    #[serde(default, skip_serializing_if = "Frames::is_empty")]
    pub frames: Frames,
    /// Seed for shuffled and sticky fragments; fixed so playthroughs replay.
    #[serde(default)]
    pub seed: u64,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// Long-range threads: promises to the reader, with their beats in notes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Thread>,
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

impl Story {
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id.as_str() == id)
    }

    /// The named text pool `id`.
    pub fn text_pool(&self, id: &str) -> Option<&crate::TextPool> {
        self.texts.iter().find(|p| p.id.as_deref() == Some(id))
    }

    pub fn room(&self, id: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id.as_str() == id)
    }

    pub fn thing(&self, id: &str) -> Option<&Thing> {
        self.things.iter().find(|t| t.id.as_str() == id)
    }

    pub fn thread(&self, id: &str) -> Option<&Thread> {
        self.threads.iter().find(|t| t.id.as_str() == id)
    }

    /// The dialogue with the given id.
    pub fn dialogue(&self, id: &str) -> Option<&Dialogue> {
        self.dialogues.iter().find(|d| d.id_str() == id)
    }

    /// The dialogue of a thing, if it has one.
    pub fn dialogue_with(&self, thing: &str) -> Option<&Dialogue> {
        self.dialogues.iter().find(|d| d.with.as_str() == thing)
    }

    pub fn ending(&self, id: &str) -> Option<&Ending> {
        self.endings.iter().find(|e| e.id.as_str() == id)
    }

    pub fn timer(&self, id: &str) -> Option<&Timer> {
        self.timers.iter().find(|t| t.id.as_str() == id)
    }

    pub fn agenda(&self, id: &str) -> Option<&Agenda> {
        self.agendas.iter().find(|a| a.id.as_str() == id)
    }

    /// The agenda of an actor, if it has one.
    pub fn agenda_of(&self, actor: &str) -> Option<&Agenda> {
        self.agendas.iter().find(|a| a.actor.as_str() == actor)
    }

    /// Things that act: the actors of the agendas. The player acts too, but
    /// is not a thing.
    pub fn actors(&self) -> impl Iterator<Item = &Id> {
        self.agendas.iter().map(|a| &a.actor)
    }

    pub fn var(&self, id: &str) -> Option<&VarDef> {
        self.vars.iter().find(|v| v.id.as_str() == id)
    }
}
