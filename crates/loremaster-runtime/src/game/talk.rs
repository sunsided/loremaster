//! Talks: a dialogue from `dialogues/` played through dramaturge.
//!
//! dramaturge picks the available options and applies their outcomes. It
//! writes facts, relations and memories into the save state through the
//! game's `NarrativeState` (see `narrative.rs`). The runtime keeps everything
//! else: it renders the lines (they are loremaster markup), runs the world
//! effects in `then`, logs what changed, and keeps the talk open until the
//! player leaves.

use super::{Game, rule_run};
use crate::output::{Choice, TalkInfo};
use crate::state::Talk;
use crate::text;
use crate::world::{CDialogue, CEffect, COption};
use crate::{BlockKind, LogEntry, Run, Source, Turn};
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, DialogTarget, DialogText, DialogueNodeId,
    EntityId, EventKind, Fact, FactId, FactScope, FactValue, Memory, MemoryId, MemoryQuery,
    RelationshipAxisId,
};
use dramaturge_runtime::{
    AvailableOptionsIter, CustomHandler, DefaultConditionEvaluator, DefaultOutcomeApplier,
    DefaultRuntime, DialogCursor, DialogueRuntime, HostHandler, NarrativeState, select_entry,
};
use loremaster_lang::CompiledLine;
use loremaster_model::{Id, Layer, Value};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

/// Answers derived values (`time.phase`, `here`, ...) in dialogue conditions.
#[derive(Clone, Copy)]
struct DerivedValues;

impl CustomHandler<Game> for DerivedValues {
    fn condition(&self, key: &str, args: &[FactValue], game: &Game) -> bool {
        game.eval().custom(key, args)
    }
}

impl CustomHandler<Turned<'_>> for DerivedValues {
    fn condition(&self, key: &str, args: &[FactValue], state: &Turned<'_>) -> bool {
        state.game.eval().custom(key, args)
    }
}

/// The game during one turn of a talk: dramaturge applies outcomes to it.
/// Each read goes to the game. Each change goes to the game too, and the turn
/// log gets a line with the new value, in the order of the changes. A change
/// that stores nothing (a 0 delta, a fact outside the world scope) gets no line.
struct Turned<'a> {
    game: &'a mut Game,
    turn: &'a mut Turn,
}

impl Turned<'_> {
    fn log(&mut self, kind: &str, text: String) {
        self.turn.log.push(LogEntry {
            kind: kind.into(),
            text,
        });
    }
}

impl NarrativeState for Turned<'_> {
    fn fact(&self, scope: &FactScope, id: &FactId) -> Option<Cow<'_, FactValue>> {
        NarrativeState::fact(&*self.game, scope, id)
    }

    fn set_fact(&mut self, scope: FactScope, fact: Fact) {
        let stored = scope == FactScope::World;
        let line = match &fact.value {
            FactValue::Bool(b) => Some(Value::Bool(*b)),
            FactValue::Int(i) => Some(Value::Int(*i)),
            FactValue::Text(t) => Some(Value::Text(t.clone())),
            _ => None,
        }
        .map(|value| format!("{} = {value}", fact.id.0));
        NarrativeState::set_fact(&mut *self.game, scope, fact);
        if stored && let Some(line) = line {
            self.log("set", line);
        }
    }

    fn remove_fact(&mut self, scope: &FactScope, id: &FactId) -> bool {
        let removed = NarrativeState::remove_fact(&mut *self.game, scope, id);
        if removed {
            self.log("set", format!("{} removed", id.0));
        }
        removed
    }

    fn alignment(&self, axis: &AlignmentAxisId, pole: &AlignmentPoleId) -> Option<i32> {
        NarrativeState::alignment(&*self.game, axis, pole)
    }

    fn add_alignment(&mut self, axis: AlignmentAxisId, pole: AlignmentPoleId, delta: i32) {
        NarrativeState::add_alignment(&mut *self.game, axis, pole, delta);
    }

    fn relationship(
        &self,
        source: EntityId,
        target: EntityId,
        axis: &RelationshipAxisId,
    ) -> Option<i32> {
        NarrativeState::relationship(&*self.game, source, target, axis)
    }

    fn add_relationship(
        &mut self,
        source: EntityId,
        target: EntityId,
        axis: RelationshipAxisId,
        delta: i32,
    ) {
        NarrativeState::add_relationship(&mut *self.game, source, target, axis.clone(), delta);
        // The game keeps only relations from the player; others store nothing.
        if delta == 0 || source != self.game.world.scope.player() {
            return;
        }
        let Some(thing) = self
            .game
            .world
            .symbols
            .entity_key(target)
            .map(str::to_string)
        else {
            return;
        };
        let value = NarrativeState::relationship(&*self.game, source, target, &axis).unwrap_or(0);
        self.log(
            "relation",
            format!("{thing}.{} {delta:+} = {value}", axis.0),
        );
    }

    fn add_memory(&mut self, owner: EntityId, memory: Memory) {
        let symbols = &self.game.world.symbols;
        let line = symbols
            .entity_key(owner)
            .zip(symbols.memory_key(memory.id))
            .map(|(thing, key)| format!("{thing} remembers {key}"));
        NarrativeState::add_memory(&mut *self.game, owner, memory);
        if let Some(line) = line {
            self.log("memory", line);
        }
    }

    fn remove_memory(&mut self, owner: EntityId, id: MemoryId) -> bool {
        let symbols = &self.game.world.symbols;
        let line = symbols
            .entity_key(owner)
            .zip(symbols.memory_key(id))
            .map(|(thing, key)| format!("{thing} forgets {key}"));
        let removed = NarrativeState::remove_memory(&mut *self.game, owner, id);
        if removed && let Some(line) = line {
            self.log("memory", line);
        }
        removed
    }

    fn matches_memory_query(&self, owner: EntityId, query: &MemoryQuery) -> bool {
        NarrativeState::matches_memory_query(&*self.game, owner, query)
    }

    fn has_tag_available(&self, tag: &DialogTagKind) -> bool {
        NarrativeState::has_tag_available(&*self.game, tag)
    }

    fn emit_event(&mut self, kind: EventKind) {
        NarrativeState::emit_event(&mut *self.game, kind);
    }
}

/// Applies the world effects of `then` (a host outcome), which dramaturge
/// can not. They write into the turn of the [`Turned`] state.
struct WorldEffects {
    owner: Id,
}

impl CustomHandler<Turned<'_>> for WorldEffects {}

impl HostHandler<Then, Turned<'_>> for WorldEffects {
    fn host_outcome(&mut self, effects: &Then, state: &mut Turned<'_>) {
        let Turned { game, turn } = state;
        // A `stop` (or `Flow::Stop` of a called behavior) ends the
        // node's effects, as it ends a rule.
        let outer = std::mem::replace(&mut game.stopped, false);
        for effect in effects.iter() {
            game.apply(turn, &self.owner, effect, 1);
            if game.stopped || game.state.ended {
                break;
            }
        }
        game.stopped = outer;
    }
}

fn evaluator() -> DefaultConditionEvaluator<DerivedValues> {
    DefaultConditionEvaluator::with_custom(DerivedValues)
}

type Runtime =
    DefaultRuntime<DefaultConditionEvaluator<DerivedValues>, DefaultOutcomeApplier<WorldEffects>>;

/// The compiled effects of a `then` list: the host outcome of a dialogue.
type Then = Arc<[CEffect]>;

/// dramaturge for a turn of a talk, with the world effects of the dialogue.
fn runtime(d: &CDialogue) -> Runtime {
    DefaultRuntime::with_parts(
        evaluator(),
        DefaultOutcomeApplier::with_custom(WorldEffects {
            owner: Id::new(format!("dialogue:{}", d.id)),
        }),
    )
}

/// The talk as a dramaturge cursor. The save keeps node keys, which stay
/// valid when a story changes (see `save.rs`); the cursor uses node ids.
fn cursor(d: &CDialogue, talk: &Talk) -> Option<DialogCursor> {
    let id = |key: &str| d.node_by_key(key).map(|n| n.id);
    let mut cursor = DialogCursor::new(id(&talk.node)?);
    cursor.back = talk.back.iter().filter_map(|k| id(k)).collect();
    Some(cursor)
}

/// Store the cursor in the talk, as node keys. An id without a node is
/// not stored: the talk keeps its node, and the return stack skips it.
fn keep(d: &CDialogue, cursor: &DialogCursor, talk: &mut Talk) {
    let key = |id| d.key_of(id).map(str::to_string);
    let Some(node) = key(cursor.current) else {
        return;
    };
    talk.node = node;
    talk.back = cursor.back.iter().filter_map(|id| key(*id)).collect();
}

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
    /// The talk in progress, if any.
    pub fn talk(&self) -> Option<&Talk> {
        self.state.talk.as_ref()
    }

    fn current(&self) -> Option<Arc<CDialogue>> {
        let talk = self.state.talk.as_ref()?;
        self.world.dialogue(talk.dialogue.as_str()).cloned()
    }

    /// The options the player can choose now.
    fn available(&self) -> Vec<COption> {
        let Some(talk) = &self.state.talk else {
            return Vec::new();
        };
        let Some(node) = self
            .world
            .dialogue(talk.dialogue.as_str())
            .and_then(|d| d.node_by_key(&talk.node))
        else {
            return Vec::new();
        };
        AvailableOptionsIter::new(node, &evaluator(), self)
            .cloned()
            .collect()
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

    /// Enter a node through dramaturge: show its line, apply its outcomes
    /// and let world rules hear about it. A node whose `when` does not hold
    /// can not be entered, and the talk ends. True when the node was entered.
    fn enter(&mut self, turn: &mut Turn, d: &Arc<CDialogue>, id: DialogueNodeId) -> bool {
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
        self.end_without_options();
        true
    }

    /// Show a node's line.
    fn show(&mut self, turn: &mut Turn, d: &CDialogue, id: DialogueNodeId) {
        let Some(node) = d.node(id) else { return };
        let key = d.key_of(id).unwrap_or_default();
        let base = Run {
            text: String::new(),
            source: Some(Source {
                owner: format!("dialogue:{}", d.id),
                fragment: d.keys.iter().position(|k| k == key).unwrap_or(0),
                layer: Layer::Characters,
                condition: self.node_when(d, key),
                variant: 0,
            }),
            link: None,
            speaker: None,
        };
        let runs = self.line(&node.text, &base);
        turn.push(BlockKind::Speech, runs);
    }

    /// A node with no option to choose ends the talk.
    fn end_without_options(&mut self) {
        if self.state.talk.is_some() && self.available().is_empty() {
            self.end_talk();
        }
    }

    fn node_when(&self, d: &CDialogue, key: &str) -> Option<String> {
        self.world
            .story
            .dialogues
            .iter()
            .find(|x| x.id == d.id)
            .and_then(|x| x.node(key))
            .and_then(|n| n.when.clone())
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
                    // Back to a node the talk was at: its line shows again,
                    // its outcomes do not apply again.
                    self.show(turn, &d, next);
                    self.end_without_options();
                } else {
                    self.enter(turn, &d, next);
                }
            }
            _ => self.end_talk(),
        }
        self.advance(turn);
    }

    pub(super) fn leave(&mut self, turn: &mut Turn) {
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

    pub(super) fn refuse(&self, turn: &mut Turn) {
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

    /// Typed text during a talk: a number, `bye` or `leave`, or the first
    /// word of an answer.
    pub(super) fn talk_input(&mut self, turn: &mut Turn, typed_text: &str) {
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

    /// The talk in progress, for the inspector: the node, its outcomes, and
    /// every option with the reason it is hidden.
    pub(crate) fn talk_view(&self) -> Option<TalkView> {
        let talk = self.state.talk.as_ref()?;
        let d = self.world.dialogue(talk.dialogue.as_str())?;
        let src = self
            .world
            .story
            .dialogues
            .iter()
            .find(|x| x.id == d.id)?
            .node(&talk.node)?;
        let node = d.node_by_key(&talk.node)?;
        let available: Vec<_> = self.available().iter().map(|o| o.id).collect();
        let options = node
            .options
            .iter()
            .zip(&src.options)
            .map(|(o, s)| TalkOptionView {
                choice: available.iter().position(|a| *a == o.id),
                say: s.say.source(),
                when: s.when.clone(),
                go: s.go.clone(),
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
