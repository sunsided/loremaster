//! Talks: a dialogue from `dialogues/` played through dramaturge.
//!
//! dramaturge picks the available options and applies their outcomes. It
//! writes facts, relations and memories into the save state through the
//! game's `NarrativeState` (see `narrative.rs`). The runtime keeps everything
//! else: it renders the lines (they are loremaster markup), runs the world
//! effects in `then`, logs what changed, and keeps the talk open until the
//! player leaves. A node without an option to choose goes on to its `next`
//! node in the same turn (auto-continue, dramaturge R2).
//!
//! Next to this module: `turned` (the game as dramaturge's state during a
//! turn, with the log of each change), `view` (the talk for the inspector)
//! and `input` (typed text during a talk).

mod input;
mod turned;
mod view;

pub use view::{TalkOptionView, TalkView};

use super::{Game, rule_run};
use crate::output::{Choice, TalkInfo};
use crate::state::Talk;
use crate::text;
use crate::world::{CompiledDialogue, CompiledEffect, CompiledOption};
use crate::{BlockKind, LogEntry, Run, Source, Turn};
use dramaturge_model::{DialogTarget, DialogText, DialogueNodeId, SymbolTable};
use dramaturge_runtime::{
    ADVANCE_LIMIT, AvailableOptionsIter, DefaultConditionEvaluator, DefaultOutcomeApplier,
    DefaultRuntime, DialogCursor, DialogueRuntime, select_entry,
};
use loremaster_lang::CompiledLine;
use loremaster_model::{Id, Layer};
use std::sync::Arc;
use turned::{DerivedValues, Turned, WorldEffects};

fn evaluator() -> DefaultConditionEvaluator<DerivedValues> {
    DefaultConditionEvaluator::with_custom(DerivedValues)
}

type Runtime =
    DefaultRuntime<DefaultConditionEvaluator<DerivedValues>, DefaultOutcomeApplier<WorldEffects>>;

/// The compiled effects of a `then` list: the host outcome of a dialogue.
type Then = Arc<[CompiledEffect]>;

/// dramaturge for a turn of a talk, with the world effects of the dialogue.
fn runtime(d: &CompiledDialogue) -> Runtime {
    DefaultRuntime::with_parts(
        evaluator(),
        DefaultOutcomeApplier::with_custom(WorldEffects {
            owner: Id::new(format!("dialogue:{}", d.id)),
        }),
    )
}

/// The talk as a dramaturge cursor. The save keeps node keys, which stay
/// valid when a story changes (see `save.rs`); the cursor uses node ids.
fn cursor(d: &CompiledDialogue, talk: &Talk) -> Option<DialogCursor> {
    let id = |key: &str| d.node_by_key(key).map(|n| n.id);
    let mut cursor = DialogCursor::new(id(&talk.node)?);
    cursor.back = talk.back.iter().filter_map(|k| id(k)).collect();
    Some(cursor)
}

/// Store the cursor in the talk, as node keys. An id without a node is
/// not stored: the talk keeps its node, and the return stack skips it.
fn keep(d: &CompiledDialogue, cursor: &DialogCursor, talk: &mut Talk) {
    let key = |id| d.key_of(id).map(str::to_string);
    let Some(node) = key(cursor.current) else {
        return;
    };
    talk.node = node;
    talk.back = cursor.back.iter().filter_map(|id| key(*id)).collect();
}

impl Game {
    /// The talk in progress, if any.
    pub fn talk(&self) -> Option<&Talk> {
        self.state.talk.as_ref()
    }

    fn current(&self) -> Option<Arc<CompiledDialogue>> {
        let talk = self.state.talk.as_ref()?;
        self.world.dialogue(talk.dialogue.as_str()).cloned()
    }

    /// The options the player can choose now.
    fn available(&self) -> Vec<CompiledOption> {
        let (Some(talk), Some(d)) = (&self.state.talk, self.current()) else {
            return Vec::new();
        };
        d.node_by_key(&talk.node)
            .map(|node| {
                AvailableOptionsIter::new(node, &evaluator(), self)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The answers the player can choose now, numbered from 0: the answers
    /// of a rule that waits, or else the answers of the talk.
    pub fn choices(&self) -> Vec<Choice> {
        if self.state.awaiting.is_some() {
            return self.await_choices();
        }
        self.available()
            .iter()
            .enumerate()
            .map(|(index, o)| Choice {
                index,
                text: o
                    .text
                    .as_host()
                    .map(|l| text::line_text(&self.eval(), l))
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub(super) fn talk_info(&self) -> Option<TalkInfo> {
        let talk = self.state.talk.as_ref()?;
        let with = self.world.dialogue(talk.dialogue.as_str())?.with.clone();
        Some(TalkInfo {
            dialogue: talk.dialogue.to_string(),
            name: self.the(&with, true),
            with: with.to_string(),
            node: talk.node.clone(),
        })
    }

    /// The thing has a dialogue with a start node that applies now.
    pub(crate) fn can_talk(&self, thing: &str) -> bool {
        (self.world.dialogue_with(thing))
            .is_some_and(|d| select_entry(&d.graph, &evaluator(), self).is_some())
    }

    /// Start the dialogue of a thing at its first start node that applies.
    /// False when the thing has no dialogue or no start node applies.
    pub(super) fn start_talk(&mut self, turn: &mut Turn, thing: &Id) -> bool {
        let Some(d) = self.world.dialogue_with(thing.as_str()).cloned() else {
            return false;
        };
        let Some(start) = select_entry(&d.graph, &evaluator(), &*self) else {
            return false;
        };
        let node = d.key_of(start).unwrap_or_default().to_string();
        let log = turn.log.len();
        turn.log.push(LogEntry {
            kind: "talk".into(),
            text: format!("{} at {node}", d.id),
        });
        self.state.talk = Some(Talk {
            dialogue: d.id.clone(),
            node,
            back: Vec::new(),
        });
        // select_entry checked the node's `when`; should `enter` still refuse
        // the node, no talk started.
        let entered = self.enter(turn, &d, start);
        if !entered {
            turn.log.truncate(log);
        }
        entered
    }

    /// Enter a node, then the nodes that `next` leads to while a node has no
    /// option to choose (auto-continue). Like dramaturge's `advance`, but each
    /// node shows its lines before its outcomes apply, and a loop of `next`
    /// stops after [`ADVANCE_LIMIT`] nodes and ends the talk. True when the
    /// first node was entered.
    fn enter(&mut self, turn: &mut Turn, d: &Arc<CompiledDialogue>, id: DialogueNodeId) -> bool {
        let mut cursor = (self.state.talk.as_ref())
            .and_then(|t| cursor(d, t))
            .unwrap_or_else(|| DialogCursor::new(id));
        cursor.current = id;
        for entered in 0..ADVANCE_LIMIT {
            if !self.enter_node(turn, d, cursor.current) {
                return entered > 0;
            }
            // A world effect can end the story or take the player away from
            // the thing: then the talk ends, also at a node with options.
            if self.state.ended || !self.eval().present(d.with.as_str()) {
                self.end_talk();
                return true;
            }
            let go_on = self.state.talk.is_some() && self.available().is_empty();
            let next = (d.node(cursor.current))
                .and_then(|n| n.next.as_ref())
                .filter(|_| go_on);
            let Some(next) = next.filter(|t| cursor.follow(t).is_some()) else {
                self.end_without_options();
                return true;
            };
            if let Some(talk) = &mut self.state.talk {
                keep(d, &cursor, talk);
            }
            if *next == DialogTarget::Return {
                self.back_to(turn, d, cursor.current);
                return true;
            }
        }
        turn.log.push(LogEntry {
            kind: "author".into(),
            text: format!(
                "{}: {ADVANCE_LIMIT} nodes in a row go on by `next` without a choice \
                 (a loop?); the talk ends",
                d.id
            ),
        });
        self.end_talk();
        true
    }

    /// Enter one node through dramaturge: show its lines, apply its outcomes
    /// and let world rules hear about it. A node whose `when` does not hold
    /// can not be entered, and the talk ends. True when the node was entered.
    fn enter_node(
        &mut self,
        turn: &mut Turn,
        d: &Arc<CompiledDialogue>,
        id: DialogueNodeId,
    ) -> bool {
        let Some(node) = d.node(id) else {
            self.end_talk();
            return false;
        };
        let mut runtime = runtime(d);
        let state = Turned { game: self, turn };
        if !DialogueRuntime::<_, Then>::conditions_hold(&runtime, &node.conditions, &state) {
            self.end_talk();
            return false;
        }
        // The line shows the state before the node's outcomes.
        self.show(turn, d, id);
        runtime.enter_node(node, &mut Turned { game: self, turn });
        // World rules hear about the node, after its outcomes.
        let event = crate::Event::Talk {
            dialogue: d.id.clone(),
            node: d.key_of(id).unwrap_or_default().to_string(),
        };
        self.fire(turn, &event, 1);
        true
    }

    /// Show a node's lines, in order: its `say`, then its `lines`. A line
    /// of the player is said (like an answer), the others are speech.
    fn show(&mut self, turn: &mut Turn, d: &CompiledDialogue, id: DialogueNodeId) {
        let Some(node) = d.node(id) else { return };
        let key = d.key_of(id).unwrap_or_default();
        let base = Run {
            text: String::new(),
            source: Some(Source {
                owner: format!("dialogue:{}", d.id),
                fragment: d.keys.iter().position(|k| k == key).unwrap_or(0),
                layer: Layer::Characters,
                condition: (self.world.story.dialogue(d.id.as_str()))
                    .and_then(|x| x.node(key)?.when.clone()),
                variant: 0,
            }),
            link: None,
            speaker: None,
            emphasis: false,
        };
        for (speaker, text) in node.all_lines() {
            let player = self.world.symbols.entity_key(speaker.0) == Some(SymbolTable::PLAYER_KEY);
            let kind = if player {
                BlockKind::Said
            } else {
                BlockKind::Speech
            };
            // `say: { text: <id> }`: the core chooses the line of the pool.
            let runs = match text {
                DialogText::Pool(id) => self.say_pool(turn, id, &base),
                text => self.line(text, &base),
            };
            turn.push(kind, runs);
        }
    }

    /// Back to a node the talk was at (`go: back`, `next: back`): its lines
    /// show again, its `when` is not checked and its outcomes do not apply
    /// again. Without an option to choose, the talk ends there.
    fn back_to(&mut self, turn: &mut Turn, d: &CompiledDialogue, id: DialogueNodeId) {
        self.show(turn, d, id);
        self.end_without_options();
    }

    /// A node with no option to choose ends the talk.
    fn end_without_options(&mut self) {
        if self.state.talk.is_some() && self.available().is_empty() {
            self.end_talk();
        }
    }

    /// A line of the dialogue, rendered now, as runs like `base`.
    fn line(&self, text: &DialogText<CompiledLine>, base: &Run) -> Vec<Run> {
        text.as_host()
            .map(|l| text::line_runs(&self.eval(), l, base))
            .unwrap_or_default()
    }

    pub(super) fn end_talk(&mut self) {
        self.state.talk = None;
    }

    /// Choose an answer: the player says it, dramaturge applies its outcomes,
    /// and the talk goes where the option leads. One turn passes.
    pub(super) fn choose(&mut self, turn: &mut Turn, index: usize) {
        let options = self.available();
        let Some(option) = options.get(index) else {
            self.refuse(turn);
            return;
        };
        let Some(d) = self.current() else {
            self.end_talk();
            return;
        };
        let base = rule_run(String::new(), &Id::new(format!("dialogue:{}", d.id)));
        let runs = self.line(&option.text, &base);
        turn.push(BlockKind::Said, runs);
        let target = runtime(&d).choose_option(option, &mut Turned { game: self, turn });
        // A world effect can end the story or take the player away.
        let here = self.eval().present(d.with.as_str());
        let mut cursor = self.state.talk.as_ref().and_then(|t| cursor(&d, t));
        let next = match &mut cursor {
            Some(cursor) if here && !self.state.ended => cursor.follow(&target),
            _ => None,
        };
        match (next.filter(|id| d.node(*id).is_some()), cursor) {
            (Some(next), Some(cursor)) => {
                if let Some(talk) = &mut self.state.talk {
                    keep(&d, &cursor, talk);
                }
                if target == DialogTarget::Return {
                    self.back_to(turn, &d, next);
                } else {
                    self.enter(turn, &d, next);
                }
            }
            _ => self.end_talk(),
        }
        self.advance(turn, self.world.story.clock.minutes_per_turn);
    }
}
