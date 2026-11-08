//! Read-only queries of the Session API (ADR 0003, point 8): what the player
//! can see and reach, the verbs, and the answers of a talk. They change
//! nothing.

use super::Session;
use crate::{Choice, JournalEntry, Preview, Scene, Talk, VerbInfo};
use loremaster_model::Limit;

impl Session {
    /// The room, the time, the things here, the inventory and the exits, as
    /// the player sees them now: for side panels and a verb bar.
    pub fn scene(&self) -> Scene {
        self.game.scene()
    }

    /// The verbs of the story, for a verb bar.
    pub fn verbs(&self) -> Vec<VerbInfo> {
        self.game.verbs()
    }

    /// What a command that is still being typed refers to.
    pub fn preview(&self, text: &str) -> Preview {
        self.game.preview(text)
    }

    /// The quest journal (HG-3): each quest whose current state has journal
    /// text, open ones first. It takes no step.
    pub fn journal(&self) -> Vec<JournalEntry> {
        self.game.journal()
    }

    /// The combat in progress, or `None`: the enemies, the attack strengths
    /// of the last round and the last lines of the log (P5-03).
    pub fn combat(&self) -> Option<crate::CombatView> {
        self.game.combat_view()
    }

    /// The adventure sheet: the current value of each stat of the story.
    /// Empty when the story has no `stats:`.
    pub fn stats(&self) -> Vec<crate::StatView> {
        let facts = &self.game.state().facts;
        (self.game.world().story.stats.iter())
            .filter_map(|s| {
                Some(crate::StatView {
                    var: s.var.clone(),
                    label: s.label.clone(),
                    short: s.short.clone(),
                    value: facts.get(s.var.as_str())?.clone(),
                    max: s.max.as_ref().and_then(|max| match max {
                        Limit::Value(n) => Some(*n),
                        Limit::Var(v) => facts.get(v.as_str())?.as_int(),
                    }),
                })
            })
            .collect()
    }

    /// The active illustration, portrait, music and ambience. They come
    /// from the state each time and are not saved (ME-2).
    pub fn media(&self) -> crate::MediaView {
        self.game.media()
    }

    /// The talk in progress, or `None`.
    pub fn talk(&self) -> Option<&Talk> {
        self.game.talk()
    }

    /// The answers the player can choose now: those of the talk in progress,
    /// or of a rule that waits for a choice. Empty otherwise, also while the
    /// death prompt waits: its answers are in the `Await` status of the step.
    pub fn choices(&self) -> Vec<Choice> {
        self.game.choices()
    }
}
