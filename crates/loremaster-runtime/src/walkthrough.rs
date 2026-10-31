//! Walkthrough tests: play commands and check what shows and what holds.

use crate::{Game, World};
use loremaster_model::{Expect, Walkthrough};
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
}

impl WalkResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Play a walkthrough from the start of the story.
pub fn run_walkthrough(world: Arc<World>, walk: &Walkthrough, name: &str) -> WalkResult {
    let mut game = Game::new(world);
    let mut transcript = game.start().plain_text();
    let mut failures = Vec::new();
    for (i, step) in walk.steps.iter().enumerate() {
        let command = step.command();
        let text = game.input(command).plain_text();
        transcript.push_str(&format!("\n\n> {command}\n{text}"));
        if let Some(expect) = step.expect() {
            let at = format!("step {} (`{command}`)", i + 1);
            failures.extend(check(&game, expect, &text, &at));
        }
    }
    failures.extend(check(&game, &walk.expect, &transcript, "at the end"));
    WalkResult {
        name: walk.name.clone().unwrap_or_else(|| name.to_string()),
        failures,
        transcript,
    }
}

fn check(game: &Game, expect: &Expect, text: &str, at: &str) -> Vec<String> {
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
    let state = game.state();
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
    out
}
