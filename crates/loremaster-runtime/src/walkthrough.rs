//! Walkthrough tests: play commands and check what shows and what holds.

use crate::{Input, Session, State, World};
use loremaster_model::{Expect, Pending, Walkthrough};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The result of one walkthrough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkResult {
    pub name: String,
    /// One line for each expectation that failed. Empty when the walkthrough passed.
    pub failures: Vec<String>,
    /// The whole game as text, with each command after `> `.
    pub transcript: String,
    /// The `pending:` mark of the walkthrough: it is expected to fail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

/// What a walkthrough result means, with its `pending:` mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// All expectations hold.
    Passed,
    /// An expectation failed.
    Failed,
    /// The walkthrough is pending and fails, as expected.
    Pending,
    /// The walkthrough is pending, but all expectations hold: remove the mark.
    UnexpectedPass,
}

impl WalkResult {
    /// All expectations hold. A pending walkthrough can pass too; see [`WalkResult::verdict`].
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn verdict(&self) -> Verdict {
        match (self.passed(), self.pending.is_some()) {
            (true, false) => Verdict::Passed,
            (false, false) => Verdict::Failed,
            (false, true) => Verdict::Pending,
            (true, true) => Verdict::UnexpectedPass,
        }
    }

    /// The result is as expected: passed, or pending and failed.
    pub fn ok(&self) -> bool {
        matches!(self.verdict(), Verdict::Passed | Verdict::Pending)
    }
}

/// Play a walkthrough from the start of the story, through the Session API.
pub fn run_walkthrough(world: Arc<World>, walk: &Walkthrough, name: &str) -> WalkResult {
    let mut session = Session::new(world);
    let mut transcript = session.start().plain_text();
    let mut failures = Vec::new();
    for (i, step) in walk.steps.iter().enumerate() {
        let command = step.command();
        let text = session.submit(Input::Text(command.into())).plain_text();
        transcript.push_str(&format!("\n\n> {command}\n{text}"));
        if let Some(expect) = step.expect() {
            let at = format!("step {} (`{command}`)", i + 1);
            failures.extend(check(session.state(), expect, &text, &at));
        }
    }
    failures.extend(check(
        session.state(),
        &walk.expect,
        &transcript,
        "at the end",
    ));
    WalkResult {
        name: walk.name.clone().unwrap_or_else(|| name.to_string()),
        failures,
        transcript,
        pending: walk.pending.clone(),
    }
}

fn check(state: &State, expect: &Expect, text: &str, at: &str) -> Vec<String> {
    let mut out = Vec::new();
    for s in &expect.see {
        if !text.contains(s.as_str()) {
            out.push(format!("{at}: does not show \"{s}\""));
        }
    }
    for s in &expect.not_see {
        if text.contains(s.as_str()) {
            out.push(format!("{at}: shows \"{s}\""));
        }
    }
    for (name, want) in &expect.facts {
        match state.facts.get(name) {
            Some(v) if v == want => {}
            Some(v) => out.push(format!("{at}: `{name}` is {v}, not {want}")),
            None => out.push(format!("{at}: there is no `{name}`")),
        }
    }
    if let Some(room) = &expect.room
        && state.room != *room
    {
        out.push(format!(
            "{at}: the player is in `{}`, not `{room}`",
            state.room
        ));
    }
    if let Some(ended) = expect.ended
        && state.ended != ended
    {
        let word = if ended { "has not ended" } else { "has ended" };
        out.push(format!("{at}: the game {word}"));
    }
    if let Some(want) = &expect.ending
        && state.ending.as_ref() != Some(want)
    {
        match &state.ending {
            Some(got) => out.push(format!("{at}: the ending is `{got}`, not `{want}`")),
            None => out.push(format!("{at}: the game has no ending, not `{want}`")),
        }
    }
    out
}
