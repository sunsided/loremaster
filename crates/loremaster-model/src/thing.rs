use crate::{Fragment, Frames, Id, Note, Rule, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where a thing is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Room(Id),
    Carried,
    Nowhere,
    /// Inside a container: `{ in: trophy_case }`.
    In(Id),
    /// On a supporter: `{ on: table }`.
    On(Id),
}

impl Location {
    /// The location as one word for effects and views: a room id, `carried`,
    /// `nowhere`, `in:box` or `on:table`.
    pub fn to_word(&self) -> String {
        match self {
            Location::Room(r) => r.to_string(),
            Location::Carried => "carried".into(),
            Location::Nowhere => "nowhere".into(),
            Location::In(t) => format!("in:{t}"),
            Location::On(t) => format!("on:{t}"),
        }
    }

    /// The inverse of [`Location::to_word`]. Any other word is a room id.
    pub fn from_word(word: &str) -> Self {
        match word {
            "carried" => Location::Carried,
            "nowhere" => Location::Nowhere,
            _ => match word.split_once(':') {
                Some(("in", t)) => Location::In(Id::new(t)),
                Some(("on", t)) => Location::On(Id::new(t)),
                _ => Location::Room(Id::new(word)),
            },
        }
    }

    /// The thing that holds this one, for `in` and `on`.
    pub fn holder(&self) -> Option<&Id> {
        match self {
            Location::In(t) | Location::On(t) => Some(t),
            _ => None,
        }
    }
}

/// An object in the world.
///
/// Behaviour comes from properties: a thing with an `open` property can be
/// opened and closed, one with `locked` and a `key` can be unlocked, one with
/// `lit` is a light source. Every property is world state, readable in
/// conditions as `thing.prop`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thing {
    pub id: Id,
    pub name: String,
    /// Extra nouns the parser accepts for this thing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub location: Location,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub portable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<Id>,
    /// Things can be put in it. With an `open` property, it shows and gives
    /// its contents only while open; with `transparent: true` in its
    /// properties, the contents show when it is closed too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub container: bool,
    /// Things can be put on it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub supporter: bool,
    /// How many things fit in or on it. Without it, any number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    /// More rooms or regions where the thing is, for scenery such as a forest
    /// or a river. The thing is in these while its location is not `nowhere`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also_in: Vec<Id>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, Value>,
    /// Text for `examine`.
    #[serde(default)]
    pub description: Vec<Fragment>,
    /// Line in the room's object layer. Empty means a generated "There is a … here."
    /// Fixed scenery usually describes itself in the room text and sets `listed: false`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presence: Vec<Fragment>,
    #[serde(default = "yes", skip_serializing_if = "Clone::clone")]
    pub listed: bool,
    /// The name is a proper name, such as "Blackbeard": no article.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub proper: bool,
    /// Names that change with the state. The last stage whose condition holds
    /// is the current name; without stages, `name` and `aliases` apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<NameStage>,
    /// Author text around the responses to verbs used on this thing.
    #[serde(default, skip_serializing_if = "Frames::is_empty")]
    pub frames: Frames,
    /// Rules that belong to this thing. The loader moves them to the story's
    /// rules and sets their `within`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// One stage of a thing's name, for example "shadow", then "grim-looking man",
/// then "Blackbeard".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NameStage {
    pub text: String,
    /// Extra nouns for this stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Condition source. A stage without a condition always applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// A proper name: no article.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub proper: bool,
}

fn yes() -> bool {
    true
}

impl Thing {
    pub fn prop(&self, name: &str) -> Option<&Value> {
        self.props.get(name)
    }

    /// Words that name this thing, lowercased: the name and every alias.
    pub fn nouns(&self) -> impl Iterator<Item = String> + '_ {
        std::iter::once(self.name.to_lowercase())
            .chain(self.aliases.iter().map(|a| a.to_lowercase()))
    }
}
