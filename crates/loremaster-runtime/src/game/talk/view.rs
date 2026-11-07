//! The talk in progress, for the inspector and the in-place editor.

use crate::game::Game;
use loremaster_model::Line;
use serde::{Deserialize, Serialize};

/// The state of an option in the current node, for the inspector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkOptionView {
    /// The index to choose it with, when it shows.
    pub choice: Option<usize>,
    /// The player's line as written.
    pub say: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// `node`, `end` or `back`.
    pub go: String,
    /// What choosing it changes, in short: `set known = true`, `trust +1`.
    pub outcomes: Vec<String>,
}

/// The talk in progress, for the inspector and the in-place editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkView {
    pub dialogue: String,
    pub with: String,
    pub name: String,
    pub node: String,
    /// The file that holds the dialogue is `dialogues/<dialogue>.yaml`; this is
    /// the line as written.
    pub say: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// What entering the node changed.
    pub outcomes: Vec<String>,
    /// All options of the node, also the hidden ones.
    pub options: Vec<TalkOptionView>,
    /// The player's relations to the thing, by axis.
    pub relations: Vec<(String, i32)>,
    /// The memories the thing keeps.
    pub memories: Vec<String>,
}

impl Game {
    /// The talk in progress, for the inspector: the node, its outcomes, and
    /// every option with the reason it is hidden.
    pub(crate) fn talk_view(&self) -> Option<TalkView> {
        let talk = self.state.talk.as_ref()?;
        let d = self.world.dialogue(talk.dialogue.as_str())?;
        let src = self.world.story.dialogue(d.id.as_str())?.node(&talk.node)?;
        let node = d.node_by_key(&talk.node)?;
        let available: Vec<_> = self.available().iter().map(|o| o.id).collect();
        let options = node
            .options
            .iter()
            .zip(&src.options)
            .map(|(o, s)| TalkOptionView {
                choice: available.iter().position(|a| *a == o.id),
                say: s.say.as_ref().map(Line::source).unwrap_or_default(),
                when: s.when.clone(),
                go: s.go.as_key().to_string(),
                outcomes: crate::outline::describe(&s.outcomes),
            })
            .collect();
        let with = d.with.as_str();
        let relations = self
            .world
            .story
            .relations
            .iter()
            .map(|axis| (axis.to_string(), self.eval().relation(with, axis.as_str())))
            .collect();
        let memories = self
            .state
            .memories
            .get(with)
            .map(|m| m.iter().cloned().collect())
            .unwrap_or_default();
        Some(TalkView {
            dialogue: d.id.to_string(),
            with: with.to_string(),
            name: self.the(&d.with, true),
            node: talk.node.clone(),
            say: src.say.source(),
            when: src.when.clone(),
            outcomes: crate::outline::describe(&src.outcomes),
            options,
            relations,
            memories,
        })
    }
}
