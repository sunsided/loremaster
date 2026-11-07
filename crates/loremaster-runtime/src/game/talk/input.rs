//! Typed text during a talk, and the answers to it that are no choice.

use crate::game::Game;
use crate::{BlockKind, Turn};
use loremaster_model::Id;

impl Game {
    /// Typed text during a talk: a number, `bye` or `leave`, or the first
    /// word of an answer.
    pub(in crate::game) fn talk_input(&mut self, turn: &mut Turn, typed_text: &str) {
        let text = typed_text.trim().to_lowercase();
        let echo = |turn: &mut Turn| turn.say(BlockKind::Command, typed_text.trim());
        if let Ok(n) = text.parse::<usize>() {
            match n.checked_sub(1).filter(|i| *i < self.choices().len()) {
                Some(i) => self.choose(turn, i),
                None => {
                    echo(turn);
                    self.refuse(turn);
                }
            }
            return;
        }
        if matches!(text.as_str(), "bye" | "goodbye" | "leave" | "farewell") {
            echo(turn);
            self.leave(turn);
            return;
        }
        let word = |s: &str| {
            s.split_whitespace()
                .next()
                .map(|w| {
                    w.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_lowercase()
                })
                .unwrap_or_default()
        };
        let typed = word(&text);
        let matching: Vec<usize> = self
            .choices()
            .iter()
            .filter(|c| !typed.is_empty() && word(&c.text) == typed)
            .map(|c| c.index)
            .collect();
        match matching.as_slice() {
            [i] => self.choose(turn, *i),
            _ => {
                echo(turn);
                self.refuse(turn);
            }
        }
    }

    pub(in crate::game) fn leave(&mut self, turn: &mut Turn) {
        if let Some(info) = self.talk_info() {
            turn.say(
                BlockKind::Response,
                format!(
                    "You turn away from {}.",
                    self.the(&Id::new(info.with), false)
                ),
            );
        }
        self.end_talk();
    }

    pub(in crate::game) fn refuse(&self, turn: &mut Turn) {
        if let Some(info) = self.talk_info() {
            turn.say(
                BlockKind::Response,
                format!(
                    "You are talking to {}. Choose an answer, or leave.",
                    self.the(&Id::new(info.with), false)
                ),
            );
        }
    }
}
