use crate::{CommandPattern, Effect, Id, Note, TimeOfDay};
use serde::{Deserialize, Serialize};

/// What a character does on its own (WO-6, D6): steps at clock times or on
/// conditions, from `agendas/`. The character is an actor: a thing that acts
/// and has a place. Each actor has at most one agenda.
///
/// The steps run in order. A step is due from its time `at` (the first time
/// the clock reaches it after the step before it was due); a step without
/// `at` is due as soon as the step before it is over. A due step walks the
/// actor to its `place`, one room per turn, then runs: its `do` command, then
/// its `then` effects. An open talk with the actor holds the step back; it
/// runs late, after the talk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agenda {
    /// The file name in YAML, or `id:` in the file.
    pub id: Id,
    /// The thing that acts.
    pub actor: Id,
    pub steps: Vec<AgendaStep>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// One step of an agenda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgendaStep {
    /// A name for the save and the checks. Without it, the step's key is its
    /// place in the list, from 1 (see [`AgendaStep::key`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// The time of day from which the step is due.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<TimeOfDay>,
    /// The room where the step happens. The actor walks there along exits,
    /// one room per turn, before the step runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Id>,
    /// A condition, tested in each turn while the step is due (also on the
    /// walk to `place`). A step with `at` is skipped in the first such turn
    /// in which it does not hold; a step without `at` waits until it holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// A command that the actor runs, such as `{ verb: open, direct: stove }`.
    /// Its `check` and `verb` rules fire with the actor in the Command
    /// record; the default actions of the standard verbs are the player's
    /// and do not run. A `check` rule with `stop` refuses it, and then the
    /// step's `then` does not run; the refused try takes the actor's turn.
    #[serde(default, rename = "do", skip_serializing_if = "Option::is_none")]
    pub action: Option<CommandPattern>,
    /// What the step does, as the effects of a rule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<Effect>,
    /// The endings that this step closes on purpose: a point of no return.
    /// After the step, these endings can no longer be reached, and the ending
    /// checks (P4-04) accept that drop instead of warning about it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<Id>,
    /// Why the step closes the endings in `closes`, for the author and the
    /// checks' report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

impl AgendaStep {
    /// The key of the step at `index` (from 0) in its agenda: its `id`, or
    /// its place in the list from 1, such as `1`. The save keeps steps by
    /// this key.
    pub fn key(&self, index: usize) -> String {
        self.id
            .as_ref()
            .map_or_else(|| (index + 1).to_string(), Id::to_string)
    }
}

impl Agenda {
    /// The steps with their keys, in order.
    pub fn keyed_steps(&self) -> impl Iterator<Item = (String, &AgendaStep)> {
        self.steps.iter().enumerate().map(|(i, s)| (s.key(i), s))
    }
}
