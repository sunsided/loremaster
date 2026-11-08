//! Walkthrough tests: play commands and check what shows and what holds.

use crate::{AwaitRequest, Effect, Input, Session, State, Status, Step, World};
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
    let seed = walk.seed.unwrap_or(world.story.seed);
    let mut session = Session::with_seed(world, seed);
    let first = session.start();
    let mut transcript = first.plain_text();
    let mut played = sounds(&first);
    let mut failures = Vec::new();
    for (i, step) in walk.steps.iter().enumerate() {
        let command = step.command();
        // `at`: the room where the player does the step (CK-4).
        if let Some(at) = step.plan().and_then(|p| p.at.as_ref())
            && session.state().room != *at
        {
            failures.push(format!(
                "step {} (`{command}`): the player is in `{}`, not at `{at}`",
                i + 1,
                session.state().room
            ));
        }
        session.force_rolls(step.rolls());
        let host_event = step.plan().and_then(|p| Some((p.event.as_ref()?, &p.args)));
        // An event step is no answer to an action.
        if host_event.is_none()
            && let Some(why) = undeclared_result(&session, command)
        {
            failures.push(format!("step {} (`{command}`): {why}", i + 1));
        }
        let step_result = match host_event {
            Some((event, args)) => match session.host_event(event.as_str(), args.clone()) {
                Ok(step) => Some(step),
                Err(why) => {
                    failures.push(format!("step {} (`{command}`): {why}", i + 1));
                    None
                }
            },
            None => Some(session.submit(Input::Text(command.into()))),
        };
        let text = step_result
            .as_ref()
            .map(Step::plain_text)
            .unwrap_or_default();
        let step_sounds = step_result.as_ref().map(sounds).unwrap_or_default();
        transcript.push_str(&format!("\n\n> {command}\n{text}"));
        for why in session.forced_roll_errors() {
            failures.push(format!("step {} (`{command}`): {why}", i + 1));
        }
        if let Some(expect) = step.expect() {
            let at = format!("step {} (`{command}`)", i + 1);
            failures.extend(check(session.state(), expect, &text, &step_sounds, &at));
        }
        played.extend(step_sounds);
    }
    failures.extend(check(
        session.state(),
        &walk.expect,
        &transcript,
        &played,
        "at the end",
    ));
    WalkResult {
        name: walk.name.clone().unwrap_or_else(|| name.to_string()),
        failures,
        transcript,
        pending: walk.pending.clone(),
    }
}

/// In a story with an interface, the answer to an action must be one of its
/// declared results by name (IF-6): a number or a label is not.
fn undeclared_result(session: &Session, command: &str) -> Option<String> {
    let interface = session.world().story.interface.as_ref()?;
    let Status::Await { awaiting } = session.status() else {
        return None;
    };
    let AwaitRequest::Action { name, results, .. } = awaiting.request else {
        return None;
    };
    interface.action(name.as_str())?;
    let answer = command.trim();
    (!results.is_empty()
        && !results
            .iter()
            .any(|r| r.as_str().eq_ignore_ascii_case(answer)))
    .then(|| format!("`{answer}` is not a declared result of the action `{name}`"))
}

/// The ids of the sounds of a step.
fn sounds(step: &Step) -> Vec<String> {
    (step.effects.iter())
        .filter_map(|e| match e {
            Effect::Sound { sound } => Some(sound.id.clone()),
            _ => None,
        })
        .collect()
}

fn check(state: &State, expect: &Expect, text: &str, sounds: &[String], at: &str) -> Vec<String> {
    let mut out = Vec::new();
    for s in &expect.sound {
        if !sounds.contains(s) {
            out.push(format!("{at}: does not play the sound `{s}`"));
        }
    }
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
