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
use crate::world::CDialogue;
use crate::{BlockKind, LogEntry, Run, Source, Turn};
use dramaturge_model::{
    AlignmentAxisId, AlignmentPoleId, DialogTagKind, DialogTarget, DialogueNodeId, DialogueOption,
    EntityId, EventKind, Fact, FactId, FactScope, FactValue, Memory, MemoryId, MemoryQuery,
    Outcome, RelationshipAxisId,
};
use dramaturge_runtime::{
    ConditionEvaluator, CustomHandler, DefaultConditionEvaluator, DefaultOutcomeApplier,
    DefaultRuntime, DialogueRuntime, NarrativeState, OutcomeApplier,
};
use loremaster_lang::{ADD_KEY, THEN_KEY};
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

/// Applies the outcomes that dramaturge can not: `add`, and the world effects
/// of `then`. They write into the turn of the [`Turned`] state.
struct WorldEffects {
    dialogue: Arc<CDialogue>,
}

impl CustomHandler<Turned<'_>> for WorldEffects {
    fn outcome(&mut self, key: &str, args: &[FactValue], state: &mut Turned<'_>) {
        let Turned { game, turn } = state;
        match (key, args) {
            (ADD_KEY, [FactValue::Text(name), FactValue::Int(n)]) => {
                let now = game.state.facts.get(name).and_then(Value::as_int);
                game.set_fact(turn, name, Value::Int(now.unwrap_or(0) + n));
            }
            (THEN_KEY, [FactValue::Int(i)]) => {
                let owner = Id::new(format!("dialogue:{}", self.dialogue.id));
                let Some(effects) = usize::try_from(*i)
                    .ok()
                    .and_then(|i| self.dialogue.effects.get(i))
                else {
                    return;
                };
                // A `stop` (or `Flow::Stop` of a called behavior) ends the
                // node's effects, as it ends a rule.
                let outer = std::mem::replace(&mut game.stopped, false);
                for effect in effects {
                    game.apply(turn, &owner, effect, 1);
                    if game.stopped || game.state.ended {
                        break;
                    }
                }
                game.stopped = outer;
            }
            _ => {}
        }
    }
}

fn evaluator() -> DefaultConditionEvaluator<DerivedValues> {
    DefaultConditionEvaluator::with_custom(DerivedValues)
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
    fn available(&self) -> Vec<DialogueOption> {
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
        let runtime = DefaultRuntime::with_parts(evaluator(), DefaultOutcomeApplier::new());
        runtime.available_options(node, self).cloned().collect()
    }

    /// The answers the player can choose now, numbered from 0: the answers
    /// of a rule that waits, or else the answers of the talk.
    pub fn choices(&self) -> Vec<Choice> {
        if self.state.awaiting.is_some() {
            return self.await_choices();
        }
        let Some(d) = self
            .state
            .talk
            .as_ref()
            .and_then(|t| self.world.dialogue(t.dialogue.as_str()))
        else {
            return Vec::new();
        };
        self.available()
            .iter()
            .enumerate()
            .map(|(index, o)| Choice {
                index,
                text: d
                    .graph
                    .option_say
                    .get(&o.id)
                    .map(|s| self.render(s))
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
        let ev = evaluator();
        let start = d.graph.starts.iter().copied().find(|id| {
            d.node(*id)
                .is_some_and(|n| n.conditions.iter().all(|c| ev.evaluate(c, &*self)))
        });
        let Some(start) = start else {
            return false;
        };
        self.state.talk = Some(Talk {
            dialogue: d.id.clone(),
            node: d.key_of(start).unwrap_or_default().to_string(),
            back: Vec::new(),
        });
        turn.log.push(LogEntry {
            kind: "talk".into(),
            text: format!(
                "{} at {}",
                d.id,
                self.state.talk.as_ref().map_or("", |t| &t.node)
            ),
        });
        self.enter(turn, &d, start, true);
        true
    }

    /// Show a node's line and, the first time, apply its outcomes. A node
    /// with no option to choose ends the talk.
    fn enter(&mut self, turn: &mut Turn, d: &Arc<CDialogue>, id: DialogueNodeId, outcomes: bool) {
        let Some(node) = d.node(id).cloned() else {
            self.end_talk();
            return;
        };
        let key = d.key_of(id).unwrap_or_default().to_string();
        if let Some(say) = d.graph.node_say.get(&id) {
            let text = self.render(say);
            turn.push(
                BlockKind::Speech,
                vec![Run {
                    text,
                    source: Some(Source {
                        owner: format!("dialogue:{}", d.id),
                        fragment: d.keys.iter().position(|k| *k == key).unwrap_or(0),
                        layer: Layer::Characters,
                        condition: self.node_when(d, &key),
                        variant: 0,
                    }),
                    link: None,
                }],
            );
        }
        if outcomes {
            self.apply_outcomes(turn, d, &node.outcomes_on_enter);
            // World rules hear about the node, after its outcomes.
            let event = crate::Event::Talk {
                dialogue: d.id.clone(),
                node: key.clone(),
            };
            self.fire(turn, &event, 1);
        }
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

    /// Apply outcomes through dramaturge. The [`Turned`] state logs each change.
    fn apply_outcomes(&mut self, turn: &mut Turn, d: &Arc<CDialogue>, outcomes: &[Outcome]) {
        let mut applier = DefaultOutcomeApplier::with_custom(WorldEffects {
            dialogue: d.clone(),
        });
        let mut state = Turned { game: self, turn };
        for o in outcomes {
            applier.apply(o, &mut state);
        }
    }

    pub(super) fn end_talk(&mut self) {
        self.state.talk = None;
    }

    /// Choose an answer: the player says it, dramaturge applies its outcomes,
    /// and the talk goes where the option leads. One turn passes.
    pub(super) fn choose(&mut self, turn: &mut Turn, index: usize) {
        let options = self.available();
        let Some(option) = options.get(index).cloned() else {
            self.refuse(turn);
            return;
        };
        let Some(d) = self.current() else {
            self.end_talk();
            return;
        };
        let line = d
            .graph
            .option_say
            .get(&option.id)
            .map(|s| self.render(s))
            .unwrap_or_default();
        turn.push(
            BlockKind::Said,
            vec![rule_run(line, &Id::new(format!("dialogue:{}", d.id)))],
        );
        let mut runtime = DefaultRuntime::with_parts(
            evaluator(),
            DefaultOutcomeApplier::with_custom(WorldEffects {
                dialogue: d.clone(),
            }),
        );
        let target = runtime.choose_option(&option, &mut Turned { game: self, turn });
        // A world effect can end the story or take the player away.
        let with = d.with.clone();
        let here = self.eval().present(with.as_str());
        if self.state.ended || !here {
            self.end_talk();
        } else {
            match target {
                DialogTarget::Node(next) if d.node(next).is_some() => {
                    if let Some(talk) = &mut self.state.talk {
                        let from = std::mem::take(&mut talk.node);
                        talk.back.push(from);
                        talk.node = d.key_of(next).unwrap_or_default().to_string();
                    }
                    self.enter(turn, &d, next, true);
                }
                DialogTarget::Return => {
                    let previous = self.state.talk.as_mut().and_then(|t| t.back.pop());
                    match previous.and_then(|key| d.node_by_key(&key).map(|n| (key, n.id))) {
                        Some((key, id)) => {
                            if let Some(talk) = &mut self.state.talk {
                                talk.node = key;
                            }
                            self.enter(turn, &d, id, false);
                        }
                        None => self.end_talk(),
                    }
                }
                _ => self.end_talk(),
            }
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
    pub fn talk_view(&self) -> Option<TalkView> {
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
                say: s.say.clone(),
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
            say: src.say.clone(),
            when: src.when.clone(),
            outcomes: crate::outline::describe(&src.outcomes),
            options,
            relations,
            memories,
        })
    }
}
