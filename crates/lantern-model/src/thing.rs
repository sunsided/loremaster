use crate::{Fragment, Frames, Id, Note, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where a thing is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Room(Id),
    Carried,
    Nowhere,
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
