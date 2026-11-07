//! Read-only queries of the Session API (ADR 0003, point 8): what the player
//! can see and reach, the verbs, and the answers of a talk. They change
//! nothing.

use super::Session;
use crate::{Choice, Preview, Scene, Talk, VerbInfo};

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
