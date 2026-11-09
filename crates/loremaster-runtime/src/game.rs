use crate::eval::Eval;
use crate::parser::{ParseScope, parse};
use crate::text::{self, article};
use crate::world::{CompiledEffect, CompiledText, Within};
use crate::{Block, BlockKind, Command, Link, LinkKind, LogEntry, Run, State, Turn, World};
use crate::{Event, Input, Phase};
use loremaster_model::{Id, Limit, Location, Term, TimeOfDay, Value, VerbSlots};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod agendas;
mod awaits;
mod combat;
mod machines;
mod media;
mod orders;
pub use awaits::{ARRIVED, REFUSED};
pub(crate) use combat::{ATTACK_TEXT, LUCK_TEXT};
pub(crate) use orders::SWITCH;
mod narrative;
mod talk;
pub use talk::{TalkOptionView, TalkView};

/// A game in progress: the engine behind [`Session`](crate::Session).
///
/// Hosts use the Session API, and authoring tools the crate
/// `loremaster-workbench` (ADR 0003).
#[derive(Debug, Clone)]
pub(crate) struct Game {
    world: Arc<World>,
    state: State,
    /// The room fragments in the last description the player saw, so the
    /// assembly panel matches the screen even after once-only text is used up.
    last_shown: Option<(Id, Vec<usize>)>,
    /// Rules that fired in the last turn, for the rules view.
    last_fired: Vec<Id>,
    /// The frame used for the last action, for the assembly panel.
    last_frame: Option<FrameUse>,
    /// A `stop` effect ran in the rules that fire now.
    stopped: bool,
    /// A rule with `free: true` fired in the command now: it takes no time.
    free: bool,
    /// The host answers rolls itself (SE-7). Without it, the session rolls.
    pub(crate) host_rolls: bool,
    /// The host walks the player to a thing or an exit before a command.
    pub(crate) host_walks: bool,
    /// The totals that the next rolls take, for tests. Not in the save.
    pub(crate) forced: ForcedRolls,
    /// The room of an `end_combat` that a combat event's rule ran. The
    /// combat takes it in the same step, so it is not in the save.
    pub(crate) combat_exit: Option<Id>,
}

/// Forced rolls of a walkthrough step, and the totals that did not fit
/// their dice.
#[derive(Debug, Clone, Default)]
pub(crate) struct ForcedRolls {
    pub(crate) totals: std::collections::VecDeque<i64>,
    pub(crate) misfits: Vec<String>,
}

/// What a run of rules did.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Fired {
    /// An `instead` rule fired.
    pub instead: bool,
    /// A rule ran `stop`.
    pub stopped: bool,
}

/// A verb for the verb bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbInfo {
    pub id: String,
    pub label: String,
    /// The words that the parser accepts for the verb, for example `take` and `pick up`.
    pub words: Vec<String>,
    pub slots: VerbSlots,
    pub preposition: Option<String>,
    pub bar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThingInfo {
    pub id: String,
    /// The current name: the name stage that holds now, or the name.
    pub name: String,
    /// Where the thing is now: a room, carried, in or on another thing, or
    /// nowhere.
    pub location: Location,
    /// The thing's aliases now, lowercased: its aliases, and the names and
    /// aliases of earlier name stages. The parser also takes single words
    /// of `name` ("reed" for "Hester Reed").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitInfo {
    pub direction: String,
    pub to: String,
    pub to_name: String,
}

/// What the player can see and reach right now, for side panels and the verb bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    pub room: String,
    pub room_name: String,
    /// The time of day, `HH:MM`.
    pub time: String,
    /// The clock as a number: minutes since midnight of day 1. The time of
    /// day is `minutes % 1440`.
    pub minutes: u32,
    pub phase: String,
    pub day: u32,
    pub turns: u32,
    pub dark: bool,
    pub things: Vec<ThingInfo>,
    pub inventory: Vec<ThingInfo>,
    pub exits: Vec<ExitInfo>,
    pub ended: bool,
}

/// One fragment of the current room, for the assembly panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentView {
    pub index: usize,
    pub layer: loremaster_model::Layer,
    pub mode: loremaster_model::FragmentMode,
    pub condition: Option<String>,
    pub replaces: bool,
    /// The fragment's source text; variants joined with ` / `.
    pub text: String,
    /// The source of each variant in pieces, with inline branches marked as active or not.
    pub variants: Vec<Vec<crate::SourcePiece>>,
    pub shown: bool,
    /// Why it is hidden right now.
    pub why: Option<String>,
}

/// One line of the generated objects or exits layer, for the assembly panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartView {
    /// A thing id, or an exit direction.
    pub id: String,
    /// The thing name, or the direction and the room it leads to.
    pub label: String,
    /// The source text of the line: the thing's presence text, or the generated sentence.
    pub text: String,
    /// More facts, for example "door: cottage door, closed".
    pub detail: Option<String>,
    pub shown: bool,
    pub why: Option<String>,
}

/// The generated layers of the current room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomParts {
    pub objects: Vec<PartView>,
    pub exits: Vec<PartView>,
}

/// A command for the author, not the player: it changes the game state
/// directly. The command palette and a cheat console use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AuthorCommand {
    /// Move the player to a room. Leave and enter rules do not fire.
    Goto { room: String },
    /// Set a story variable or a thing property, for example `lamp.lit`.
    Set { name: String, value: Value },
    /// Set the time of day, `HH:MM`. The day stays the same.
    Time { time: String },
    /// Put a thing in the player's hands.
    Give { thing: String },
    /// Put the game into one combination of the room matrix: go to the room and set each named value.
    /// The names and values are the ones of [`RoomMatrix`](crate::authoring::RoomMatrix).
    Situation {
        room: String,
        values: Vec<(String, String)>,
    },
}

/// The action frame used for the last action, as it was evaluated then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameUse {
    /// The verb id, for example `look`.
    pub verb: String,
    /// Where the frame is: `story`, `room:<id>` or `thing:<id>`.
    pub scope: String,
    /// `action.repeat`, `action.count` and `action.total` for that action.
    pub repeat: u32,
    pub count: u32,
    pub total: u32,
    pub parts: Vec<FramePart>,
}

/// One part of a frame: before, instead or after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FramePart {
    pub part: String,
    pub fragments: Vec<FrameFragment>,
}

/// One fragment of a frame part, with its state for that action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameFragment {
    pub index: usize,
    pub mode: loremaster_model::FragmentMode,
    pub condition: Option<String>,
    pub text: String,
    pub shown: bool,
    pub why: Option<String>,
}

const MAX_RULE_DEPTH: u8 = 8;

impl Game {
    /// A fresh game at the story's start. Call [`Game::start`] for the opening text.
    pub fn new(world: Arc<World>) -> Self {
        let story = &world.story;
        let mut state = State {
            room: story.start.clone(),
            facts: Default::default(),
            locations: story
                .things
                .iter()
                .map(|t| (t.id.clone(), t.location.clone()))
                .collect(),
            visits: Default::default(),
            minutes: u32::from(story.clock.start.0),
            turns: 0,
            progress: Default::default(),
            fired: Default::default(),
            rng: story.seed,
            ended: false,
            ending: None,
            awaiting: None,
            await_seq: 0,
            combat: None,
            action: None,
            last_command: None,
            repeat: 0,
            previous: None,
            action_counts: Default::default(),
            interactions: Default::default(),
            talked: Default::default(),
            talk: None,
            relations: Default::default(),
            memories: Default::default(),
            steps_done: Default::default(),
            timers: story
                .timers
                .iter()
                .filter(|t| t.running)
                .map(|t| (t.id.clone(), t.turns))
                .collect(),
            actors: Default::default(),
            active: None,
            orders: Default::default(),
        };
        for v in &story.vars {
            state.facts.insert(v.id.to_string(), v.initial.clone());
        }
        for t in &story.things {
            for (prop, value) in &t.props {
                state
                    .facts
                    .insert(format!("{}.{prop}", t.id), value.clone());
            }
        }
        for e in story.interface.iter().flat_map(|i| &i.events) {
            for (arg, ty) in &e.args {
                state
                    .facts
                    .insert(format!("{}.{arg}", e.id), default_arg(ty));
            }
        }
        // The first playable actor starts as the active one. The others hold
        // what they carry as closed containers; see `orders.rs`.
        if let Some(first) = story.playable.first() {
            for a in &story.playable {
                state
                    .facts
                    .insert(format!("{a}.active"), Value::Bool(a == first));
                state.facts.insert(format!("{a}.open"), Value::Bool(false));
            }
            state.active = Some(first.clone());
            if let Some(Location::Room(r)) = state.locations.get(first) {
                state.room = r.clone();
            }
            // What the first actor holds at the start, the player carries.
            for at in state.locations.values_mut() {
                if at.holder() == Some(first) {
                    *at = Location::Carried;
                }
            }
        }
        Self::restore(world, state)
    }

    /// Continue the game of `self` in a changed world, for example after an edit of the story.
    ///
    /// The state carries over. A value that is still at its start value in the old world takes
    /// the start value of the new world, so an edit of a start value shows at once. Values and
    /// things that the new world does not have are dropped; new ones start at their start value.
    /// A current room that is gone becomes the start room. Rules, endings, relations and
    /// memories that the new world does not have are dropped too, and a rule that waits for the
    /// host keeps waiting only when the new world has the same rule.
    pub(crate) fn carry_over(&self, world: Arc<World>) -> Self {
        let old_start = Game::new(self.world.clone()).state;
        let new_start = Game::new(world.clone()).state;
        let mut state = self.state.clone();
        state.facts = new_start
            .facts
            .iter()
            .map(|(k, start)| {
                let value = match (self.state.facts.get(k), old_start.facts.get(k)) {
                    (Some(now), Some(was)) if now != was => now.clone(),
                    _ => start.clone(),
                };
                (k.clone(), value)
            })
            .collect();
        let has_room = |id: &Id| world.room(id.as_str()).is_some();
        state.locations = new_start
            .locations
            .iter()
            .map(|(k, start)| {
                let value = match (self.state.locations.get(k), old_start.locations.get(k)) {
                    (Some(now), Some(was)) if now != was => match now {
                        Location::Room(r) if !has_room(r) => start.clone(),
                        _ => now.clone(),
                    },
                    _ => start.clone(),
                };
                (k.clone(), value)
            })
            .collect();
        if !has_room(&state.room) {
            state.room = world.story.start.clone();
        }
        state
            .timers
            .retain(|id, _| world.story.timer(id.as_str()).is_some());
        for (id, turns) in &new_start.timers {
            if !old_start.timers.contains_key(id) {
                state.timers.entry(id.clone()).or_insert(*turns);
            }
        }
        state.visits.retain(|r, _| has_room(r));
        let talk_ok = state.talk.as_ref().is_some_and(|t| {
            world
                .story
                .dialogue(t.dialogue.as_str())
                .is_some_and(|d| d.node(&t.node).is_some())
        });
        if !talk_ok {
            state.talk = None;
        }
        // A paused rule goes on only when the rule is the same in the new
        // world: its effects may have moved, as in `save::reconcile`.
        let rule_of = |w: &World, id: &Id| w.story.rules.iter().find(|r| r.id == *id).cloned();
        let int_prop =
            |t: &loremaster_model::Thing, p: &str| matches!(t.props.get(p), Some(Value::Int(_)));
        let combat_ok = |c: &crate::CombatState| {
            c.spec.enemies.iter().all(|e| {
                world
                    .story
                    .thing(e.as_str())
                    .is_some_and(|t| int_prop(t, "skill") && int_prop(t, "stamina"))
            }) && [
                Some(&c.spec.win),
                Some(&c.spec.loss),
                c.spec.escape.as_ref(),
            ]
            .into_iter()
            .flatten()
            .all(has_room)
        };
        // A walk before a command has no rule: the command checks its
        // objects again when it runs.
        let rule_gone = |open: &crate::state::Awaiting| {
            open.command.is_none()
                && (rule_of(&world, &open.rule).is_none()
                    || rule_of(&world, &open.rule) != rule_of(&self.world, &open.rule))
        };
        if let Some(open) = &state.awaiting
            && (rule_gone(open) || state.combat.as_ref().is_some_and(|c| !combat_ok(c)))
        {
            state.awaiting = None;
            state.combat = None;
        }
        state
            .fired
            .retain(|id| world.story.rules.iter().any(|r| r.id == *id));
        if state.previous.as_ref().is_some_and(|r| !has_room(r)) {
            state.previous = None;
        }
        if state
            .ending
            .as_ref()
            .is_some_and(|e| world.story.ending(e.as_str()).is_none())
        {
            state.ending = None;
        }
        let has_thing = |id: &str| world.story.thing(id).is_some();
        state
            .relations
            .retain(|key, _| key.split_once('.').is_some_and(|(t, _)| has_thing(t)));
        state.memories.retain(|thing, _| has_thing(thing.as_str()));
        crate::save::keep_known_steps(&world, &mut state.actors);
        crate::save::keep_known_actors(&world, &mut state);
        Self::restore(world, state)
    }

    /// Continue from a saved state.
    pub fn restore(world: Arc<World>, state: State) -> Self {
        Self {
            world,
            state,
            last_shown: None,
            last_fired: Vec::new(),
            last_frame: None,
            stopped: false,
            free: false,
            host_rolls: false,
            host_walks: false,
            forced: ForcedRolls::default(),
            combat_exit: None,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub(crate) fn world_arc(&self) -> Arc<World> {
        self.world.clone()
    }

    /// The quests whose current state has non-empty journal text, in story order:
    /// the open ones first, then the finished ones.
    pub(crate) fn journal(&self) -> Vec<crate::JournalEntry> {
        let ev = self.eval();
        let mut out: Vec<crate::JournalEntry> = (self.world.quests.iter())
            .filter_map(|q| {
                let state = match self.state.facts.get(q.machine.as_str()) {
                    Some(Value::Text(s)) => s.clone(),
                    _ => return None,
                };
                let (line, done) = q.states.get(&state)?;
                // A line whose segments are all gated out shows no entry.
                let text = crate::text::line_text(&ev, line);
                if text.trim().is_empty() {
                    return None;
                }
                Some(crate::JournalEntry {
                    quest: q.machine.to_string(),
                    title: q.title.clone(),
                    state,
                    text,
                    done: *done,
                })
            })
            .collect();
        out.sort_by_key(|e| e.done);
        out
    }

    pub(crate) fn eval(&self) -> Eval<'_> {
        Eval {
            world: &self.world,
            state: &self.state,
            game: self,
        }
    }

    /// The intro, start rules and the first room description.
    pub fn start(&mut self) -> Turn {
        let mut turn = Turn::default();
        let world = self.world.clone();
        let runs = self.assemble(&mut turn, &world.intro);
        turn.push(BlockKind::Narration, runs);
        self.enter_room(&mut turn, 0);
        self.fire(&mut turn, &Event::Start, 0);
        for machine in &world.story.machines {
            // A paused chain goes on after its answer, not here.
            let paused = self.state.awaiting.as_ref().and_then(|a| {
                let rule = world.story.rules.iter().find(|r| r.id == a.rule)?;
                rule.machine.as_ref()
            });
            if paused != Some(&machine.id) {
                self.run_next(&mut turn, &machine.id, 0);
            }
            // A `stop` in a chain ends only that chain.
            self.stopped = false;
        }
        turn.caused_by_world(0);
        self.finish(turn)
    }

    /// The description of the current room, without a turn: the state does
    /// not change. For undo and restore, which show where the player is.
    pub fn describe_here(&mut self) -> Turn {
        let state = self.state.clone();
        let mut turn = Turn::default();
        self.describe_room(&mut turn);
        self.state = state;
        crate::output::split_emphasis(&mut turn.blocks);
        self.link_things(&mut turn.blocks);
        turn
    }

    /// Put another state in place, for undo and restore.
    pub(crate) fn set_state(&mut self, state: State) {
        self.state = state;
        self.stopped = false;
        self.last_fired.clear();
        self.last_frame = None;
    }

    /// What a command that is still being typed refers to, for the verb bar.
    pub(crate) fn preview(&self, text: &str) -> crate::Preview {
        crate::parser::preview(&self.world.verbs, &self.parse_scope(), text)
    }

    /// Set a variable or property without a turn, for tools that build a state.
    pub(crate) fn set_value(&mut self, name: &str, value: Value) {
        let mut scratch = Turn::default();
        self.set_fact(&mut scratch, name, value);
    }

    /// Direct access to the state, for tools that build a state. Use `set_value` for facts.
    pub(crate) fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// The room description as it would show now, without changing the game.
    pub(crate) fn describe_preview(&self) -> Turn {
        let mut copy = self.clone();
        let mut turn = Turn::default();
        copy.describe_room(&mut turn);
        crate::output::split_emphasis(&mut turn.blocks);
        turn
    }

    /// Handle a typed command.
    pub fn input(&mut self, text: &str) -> Turn {
        self.last_frame = None;
        let mut turn = Turn::default();
        // A rule waits for an answer: the input is the answer.
        if self.state.awaiting.is_some() && !self.state.ended {
            turn.say(BlockKind::Command, text.trim());
            self.await_input(&mut turn, text);
            return self.finish(turn);
        }
        // In a talk, the chosen answer shows instead of the typed text.
        if self.state.talk.is_some() && !self.state.ended {
            self.talk_input(&mut turn, text);
            return self.finish(turn);
        }
        turn.say(BlockKind::Command, text.trim());
        if self.state.ended {
            turn.say(BlockKind::Response, "The story has ended.");
            return turn;
        }
        if let Some((actor, steps)) = self.order_in(text) {
            let minutes = self.world.story.clock.minutes_per_turn;
            if self.give_order(&mut turn, &actor, steps) {
                self.advance(&mut turn, minutes);
            }
            return self.finish(turn);
        }
        match parse(&self.world.verbs, &self.parse_scope(), text) {
            Ok(cmd) => self.perform(&mut turn, cmd),
            Err(msg) => turn.say(BlockKind::Response, msg),
        }
        self.finish(turn)
    }

    /// Let one turn of time pass without an action of the player: no command
    /// echo, no action counts, and also during a talk (SE-2).
    pub fn tick(&mut self) -> Turn {
        self.last_frame = None;
        let mut turn = Turn::default();
        if self.state.ended {
            turn.say(BlockKind::Response, "The story has ended.");
            return turn;
        }
        // While a rule waits for the host, time does not pass.
        if let Some(open) = self.state.awaiting.as_ref().map(|a| a.request.describe()) {
            turn.say(BlockKind::Response, format!("The story waits: {open}."));
            return self.finish(turn);
        }
        self.advance(&mut turn, self.world.story.clock.minutes_per_turn);
        self.finish(turn)
    }

    /// Handle a command picked on the verb bar.
    pub fn act(&mut self, cmd: Command) -> Turn {
        self.last_frame = None;
        let mut turn = Turn::default();
        // The echo of `verb("go", ["east"])` is the same as of `Go`.
        let cmd = self.as_move(cmd);
        if !matches!(cmd, Command::Choose { .. }) {
            turn.say(BlockKind::Command, self.describe_command(&cmd));
        }
        if self.state.ended {
            turn.say(BlockKind::Response, "The story has ended.");
            return turn;
        }
        if self.state.awaiting.is_some() {
            match cmd {
                Command::Choose { index } => self.await_input(&mut turn, &(index + 1).to_string()),
                _ => {
                    let open = self.state.awaiting.as_ref().map(|a| a.request.describe());
                    turn.say(
                        BlockKind::Response,
                        format!("The story waits: {}.", open.unwrap_or_default()),
                    );
                }
            }
            return self.finish(turn);
        }
        self.perform(&mut turn, cmd);
        self.finish(turn)
    }

    fn describe_command(&self, cmd: &Command) -> String {
        match cmd {
            Command::Go { direction } => format!("go {direction}"),
            Command::Choose { index } => (index + 1).to_string(),
            Command::Leave => "leave".into(),
            Command::Verb { verb, objects, .. } => {
                let def = self.world.verb(verb.as_str());
                let word = def.map_or(verb.to_string(), |v| v.label.to_lowercase());
                let names: Vec<String> = objects.iter().map(|o| self.name_of(o)).collect();
                match names.as_slice() {
                    [] => word,
                    [a] => format!("{word} {a}"),
                    [a, b, ..] => {
                        let prep = def
                            .and_then(|v| v.preposition.clone())
                            .unwrap_or("with".into());
                        format!("{word} {a} {prep} {b}")
                    }
                }
            }
        }
    }

    /// The current name of a thing: its last name stage that applies, or its name.
    fn name_of(&self, id: &Id) -> String {
        self.eval()
            .thing_name(id.as_str(), loremaster_lang::NameForm::Bare)
    }

    /// "the oil lamp", "The oil lamp", or "Blackbeard" for a proper name.
    fn the(&self, id: &Id, capital: bool) -> String {
        let form = if capital {
            loremaster_lang::NameForm::TheCap
        } else {
            loremaster_lang::NameForm::The
        };
        self.eval().thing_name(id.as_str(), form)
    }

    /// The nouns that name a thing now: those of its current name stage and of
    /// the stages before it, lowercased. A later name is not known yet.
    pub(crate) fn nouns_of(&self, t: &loremaster_model::Thing) -> Vec<String> {
        let ev = self.eval();
        match ev.name_stage(t.id.as_str()) {
            Some(current) => t.names[..=current]
                .iter()
                .flat_map(|stage| std::iter::once(&stage.text).chain(stage.aliases.iter()))
                .map(|n| n.to_lowercase())
                .collect(),
            None => t.nouns().collect(),
        }
    }

    /// The standard verb `go` with one direction, for the player or the
    /// active actor, is the move [`Command::Go`]: `Command::verb("go",
    /// vec!["up".into()])` moves the player. The direction is a compass word
    /// or an exit of the room; `go` with a thing, and a story's own verb
    /// `go`, stay verbs and run their rules.
    fn as_move(&self, cmd: Command) -> Command {
        let Command::Verb {
            verb,
            objects,
            actor,
            preposition: None,
            topic: None,
        } = &cmd
        else {
            return cmd;
        };
        let player = actor
            .as_ref()
            .is_none_or(|a| a.as_str() == crate::PLAYER || Some(a) == self.state.active.as_ref());
        let [object] = objects.as_slice() else {
            return cmd;
        };
        if verb.as_str() != "go"
            || !player
            || self.world.story.verbs.iter().any(|v| v.id == *verb)
            || self.world.thing(object.as_str()).is_some()
        {
            return cmd;
        }
        let word = object.as_str();
        let direction = match loremaster_model::Direction::from_word(word) {
            Some(d) => d.name().to_string(),
            None if self
                .visible_exits()
                .iter()
                .any(|(_, e)| e.direction.name() == word) =>
            {
                word.to_string()
            }
            None => return cmd,
        };
        Command::Go { direction }
    }

    /// Run a command, or first wait for the host to walk there.
    fn perform(&mut self, turn: &mut Turn, cmd: Command) {
        let cmd = self.as_move(cmd);
        if self.host_walks
            && self.state.talk.is_none()
            && let Some(request) = self.walk_for(&cmd)
        {
            self.open_await(turn, &Id::new("walk_to"), 1, request, None);
            if let Some(open) = &mut self.state.awaiting {
                open.command = Some(Box::new(cmd));
            }
            return;
        }
        self.perform_now(turn, cmd);
    }

    /// The walk before a command: to the first object that is in the room
    /// and not carried, or to the exit of a move.
    fn walk_for(&self, cmd: &Command) -> Option<crate::state::AwaitRequest> {
        let room = self.state.room.clone();
        let (thing, exit) = match cmd {
            Command::Go { direction } => {
                let exits = self.visible_exits();
                exits
                    .iter()
                    .find(|(_, e)| e.direction.name() == direction)?;
                (None, Some(direction.clone()))
            }
            Command::Verb { objects, actor, .. } => {
                // An unknown object or another actor: the command says why.
                // The same test as `action`: the active actor is the player.
                if actor.as_ref().is_some_and(|a| {
                    a.as_str() != crate::PLAYER && Some(a) != self.state.active.as_ref()
                }) || objects.iter().any(|o| !self.reachable(o.as_str()))
                {
                    return None;
                }
                let thing = objects
                    .iter()
                    .find(|o| !self.state.is_carried(o.as_str()))?;
                (Some(thing.clone()), None)
            }
            _ => return None,
        };
        Some(crate::state::AwaitRequest::WalkTo { room, thing, exit })
    }

    /// Run a command and move the clock by what it costs.
    fn perform_now(&mut self, turn: &mut Turn, cmd: Command) {
        if let Some(minutes) = self.action(turn, cmd) {
            self.advance(turn, minutes);
        }
    }

    /// Run a command without moving the clock. The minutes it costs, or
    /// `None` when it took no turn. An order runs through here too, so the
    /// clock moves once a turn.
    fn action(&mut self, turn: &mut Turn, cmd: Command) -> Option<u16> {
        let cmd = self.as_move(cmd);
        // Rules of the turn before (`on: turn`) do not make this command free.
        self.free = false;
        // A talk stays open until the player leaves: other commands do not run.
        match (&cmd, self.state.talk.is_some()) {
            (Command::Choose { index }, true) => {
                self.choose(turn, *index);
                return None;
            }
            (Command::Leave, true) => {
                self.leave(turn);
                return None;
            }
            (_, true) => {
                self.refuse(turn);
                return None;
            }
            (Command::Choose { .. } | Command::Leave, false) => {
                turn.say(BlockKind::Response, "You are not talking to anyone.");
                return None;
            }
            _ => {}
        }
        let clock = &self.world.story.clock;
        let minutes = match &cmd {
            Command::Go { .. } => clock.minutes_for("go"),
            Command::Verb { verb, .. } => clock.minutes_for(verb.as_str()),
            // A choice or a leave returned above.
            _ => clock.minutes_per_turn,
        };
        match cmd {
            Command::Choose { .. } | Command::Leave => {}
            Command::Go { direction } => {
                let verb = Id::new("go");
                self.begin_action(&verb, &[], &format!("go {direction}"));
                let frame = self.frame_for(&verb, None);
                self.frame_text(turn, frame.as_ref().map(|f| &f.before));
                let check = Event::Verb {
                    phase: Phase::Check,
                    verb: verb.clone(),
                    objects: vec![Id::new(direction.as_str())],
                    actor: Id::new(crate::PLAYER),
                    preposition: None,
                    topic: None,
                };
                // A check rule that waits holds the move: it moves the player itself.
                if !self.fire(turn, &check, 0).stopped && self.state.awaiting.is_none() {
                    self.go(turn, &direction);
                    self.frame_text(turn, frame.as_ref().map(|f| &f.after));
                }
            }
            Command::Verb {
                verb,
                objects,
                actor,
                preposition,
                topic,
            } => {
                // A command for another actor is an order, if it takes orders.
                if let Some(other) = actor.filter(|a| {
                    a.as_str() != crate::PLAYER && Some(a) != self.state.active.as_ref()
                }) {
                    if self.is_playable(other.as_str()) {
                        let step = Input::Command(Command::Verb {
                            verb,
                            objects,
                            actor: None,
                            preposition,
                            topic,
                        });
                        // Giving an order is a plain turn, as in the text `bilge, ...`.
                        self.give_order(turn, &other, vec![step]);
                        return Some(self.world.story.clock.minutes_per_turn);
                    }
                    turn.say(
                        BlockKind::Response,
                        format!(
                            "Only the player takes commands; {} acts on its own.",
                            self.the(&other, false)
                        ),
                    );
                    return None;
                }
                if verb.as_str() == SWITCH
                    && let Some(to) = objects.first().filter(|o| self.is_playable(o.as_str()))
                {
                    let to = to.clone();
                    self.switch_actor(turn, &to);
                    return None;
                }
                if let Some(missing) = objects.iter().find(|o| !self.reachable(o.as_str())) {
                    turn.say(
                        BlockKind::Response,
                        format!("You can't see {} here.", self.the(missing, false)),
                    );
                    return None;
                }
                let key = std::iter::once(verb.as_str())
                    .chain(objects.iter().map(Id::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.begin_action(&verb, &objects, &key);
                let frame = self.frame_for(&verb, objects.first());
                let event = |phase| Event::Verb {
                    phase,
                    verb: verb.clone(),
                    objects: objects.clone(),
                    actor: Id::new(crate::PLAYER),
                    preposition: preposition.clone(),
                    topic: topic.clone(),
                };
                self.frame_text(turn, frame.as_ref().map(|f| &f.before));
                // `check` rules can refuse the action with `stop`.
                if !self.fire(turn, &event(Phase::Check), 0).stopped
                    && self.state.awaiting.is_none()
                {
                    // `instead` rules run before the default and replace it;
                    // other rules run after it, so their text follows the response.
                    let instead = self.fire(turn, &event(Phase::Instead), 0);
                    let mut stopped = instead.stopped;
                    if !instead.instead && !stopped {
                        // An `instead` frame replaces the default response, but not the rules.
                        let replaced = self.frame_text(turn, frame.as_ref().map(|f| &f.instead));
                        if !replaced {
                            self.default_action(turn, verb.as_str(), &objects);
                        }
                        stopped = self.fire(turn, &event(Phase::After), 0).stopped;
                    }
                    if !stopped {
                        self.frame_text(turn, frame.as_ref().map(|f| &f.after));
                    }
                }
            }
        }
        self.state.action = None;
        // A free rule: no time passes, as for a command the game refuses.
        (!std::mem::take(&mut self.free)).then_some(minutes)
    }

    /// Count an action before it runs, so `action.*` includes this time.
    fn begin_action(&mut self, verb: &Id, objects: &[Id], key: &str) {
        let s = &mut self.state;
        s.repeat = if s.last_command.as_deref() == Some(key) {
            s.repeat + 1
        } else {
            1
        };
        s.last_command = Some(key.to_string());
        *s.action_counts.entry(verb.to_string()).or_default() += 1;
        *s.action_counts
            .entry(format!("{verb}@{}", s.room))
            .or_default() += 1;
        for o in objects {
            *s.interactions.entry(o.clone()).or_default() += 1;
            if verb.as_str() == "talk" {
                *s.talked.entry(o.clone()).or_default() += 1;
            }
        }
        s.action = Some(verb.clone());
    }

    /// The most specific frame for a verb: the thing's, then the room's, then the story's.
    fn frame_for(&mut self, verb: &Id, thing: Option<&Id>) -> Option<crate::world::CompiledFrame> {
        let world = self.world.clone();
        let found = thing
            .and_then(|t| {
                let ti = world.thing_idx(t.as_str())?;
                world.things[ti]
                    .frames
                    .get(verb)
                    .map(|f| (format!("thing:{t}"), f))
            })
            .or_else(|| {
                let ri = world.room_idx(self.state.room.as_str())?;
                world.rooms[ri]
                    .frames
                    .get(verb)
                    .map(|f| (format!("room:{}", self.state.room), f))
            })
            .or_else(|| world.frames.get(verb).map(|f| ("story".to_string(), f)));
        self.last_frame = found
            .as_ref()
            .map(|(scope, f)| self.frame_use(verb, scope, f));
        found.map(|(_, f)| f.clone())
    }

    /// Evaluate a frame for the assembly panel, before its fragments are used.
    fn frame_use(&self, verb: &Id, scope: &str, frame: &crate::world::CompiledFrame) -> FrameUse {
        let ev = self.eval();
        let s = &self.state;
        let part = |name: &str, fragments: &CompiledText| {
            let visible = text::visible(&ev, fragments);
            FramePart {
                part: name.to_string(),
                fragments: fragments
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let shown = visible.contains(&i);
                        let seen = s.progress.get(&f.key).map_or(0, |p| p.shown);
                        let why = if shown {
                            None
                        } else if f.mode == loremaster_model::FragmentMode::Once && seen > 0 {
                            Some("already shown once".to_string())
                        } else if !ev.holds_opt(&f.when) {
                            Some(format!(
                                "{} is false",
                                f.when_src.as_deref().unwrap_or("condition")
                            ))
                        } else {
                            Some("replaced".to_string())
                        };
                        FrameFragment {
                            index: i,
                            mode: f.mode,
                            condition: f.when_src.clone(),
                            text: f.sources.join(" / "),
                            shown,
                            why,
                        }
                    })
                    .collect(),
            }
        };
        let room_key = format!("{verb}@{}", s.room);
        FrameUse {
            verb: verb.to_string(),
            scope: scope.to_string(),
            repeat: s.repeat,
            count: s.action_counts.get(&room_key).copied().unwrap_or(0),
            total: s.action_counts.get(verb.as_str()).copied().unwrap_or(0),
            parts: [
                ("before", &frame.before),
                ("instead", &frame.instead),
                ("after", &frame.after),
            ]
            .into_iter()
            .filter(|(_, f)| !f.is_empty())
            .map(|(name, f)| part(name, f))
            .collect(),
        }
    }

    /// The rules that fired in the last turn.
    pub(crate) fn last_fired(&self) -> &[Id] {
        &self.last_fired
    }

    /// The frame used for the last action, if there was one.
    pub(crate) fn last_frame(&self) -> Option<&FrameUse> {
        self.last_frame.as_ref()
    }

    /// Show frame fragments as one response block. Returns true if any showed.
    fn frame_text(&mut self, turn: &mut Turn, fragments: Option<&CompiledText>) -> bool {
        let Some(fragments) = fragments.filter(|f| !f.is_empty()) else {
            return false;
        };
        let runs = self.assemble(turn, fragments);
        if runs.is_empty() {
            return false;
        }
        turn.log.push(LogEntry {
            kind: "frame".into(),
            text: fragments[0].owner.clone(),
        });
        turn.push(BlockKind::Response, runs);
        true
    }

    // ---- scope and light ------------------------------------------------

    fn room_def(&self) -> Option<&loremaster_model::Room> {
        self.world.room(self.state.room.as_str())
    }

    fn doors_here(&self) -> Vec<Id> {
        self.room_def()
            .map(|r| r.exits.iter().filter_map(|e| e.door.clone()).collect())
            .unwrap_or_default()
    }

    fn is_dark(&self) -> bool {
        let Some(room) = self.room_def() else {
            return false;
        };
        if !room.has_tag("dark") {
            return false;
        }
        !self.world.story.things.iter().any(|t| {
            self.state.flag(&format!("{}.lit", t.id))
                && self.present(t.id.as_str())
                && self.state.closed_around(t.id.as_str(), true).is_none()
        })
    }

    /// Things the player can see and refer to.
    pub(crate) fn in_scope(&self) -> Vec<Id> {
        let dark = self.is_dark();
        let room = self.state.room.as_str();
        let ev = self.eval();
        let mut ids: Vec<Id> = self
            .world
            .story
            .things
            .iter()
            .filter(|t| {
                let id = t.id.as_str();
                // One walk up the containers answers both "carried" and "in the room".
                let visible = match self.state.outer_location(id) {
                    Some(Location::Carried) => true,
                    _ if dark => false,
                    Some(Location::Room(r)) if r.as_str() == room => true,
                    _ => {
                        !t.also_in.is_empty()
                            && ev.is_backdrop_of(t, room, self.state.location_of(id))
                    }
                };
                visible
                    && self.state.closed_around(id, true).is_none()
                    && !self.state.is_concealed(id)
            })
            .map(|t| t.id.clone())
            .collect();
        // The active actor is the body of the player, not a thing around it.
        ids.retain(|id| Some(id) != self.state.active.as_ref());
        for d in self.doors_here() {
            if !ids.contains(&d) && !self.state.is_concealed(d.as_str()) {
                ids.push(d);
            }
        }
        ids
    }

    /// The concealed things directly in or on a holder that a search finds:
    /// not behind a closed container, unless it is transparent (WO-2).
    pub(crate) fn searchable(&self, holder: &str) -> Vec<Id> {
        self.state
            .contents(holder)
            .into_iter()
            .filter(|c| {
                self.state.flag(&format!("{c}.concealed"))
                    && self.state.closed_around(c.as_str(), true).is_none()
            })
            .cloned()
            .collect()
    }

    fn reachable(&self, id: &str) -> bool {
        self.in_scope().iter().any(|t| t.as_str() == id)
    }

    pub(crate) fn visible_exits(&self) -> Vec<(usize, &loremaster_model::Exit)> {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return Vec::new();
        };
        let ev = self.eval();
        self.world.story.rooms[ri]
            .exits
            .iter()
            .enumerate()
            .filter(|(i, _)| ev.holds_opt(&self.world.rooms[ri].exits[*i].visible))
            .collect()
    }

    /// The command record of typed text in the state now, without running
    /// it. `None` when the parser does not understand the text.
    pub(crate) fn parse_text(&self, text: &str) -> Option<Command> {
        parse(&self.world.verbs, &self.parse_scope(), text).ok()
    }

    fn parse_scope(&self) -> ParseScope {
        ParseScope {
            things: self
                .in_scope()
                .iter()
                .filter_map(|id| self.world.thing(id.as_str()))
                .map(|t| (t.id.clone(), self.nouns_of(t)))
                // A playable actor is known from everywhere, for `switch to`.
                .chain(
                    self.remote_actors()
                        .map(|t| (t.id.clone(), self.nouns_of(t))),
                )
                .collect(),
            exits: self
                .visible_exits()
                .iter()
                .map(|(_, e)| {
                    let name =
                        e.to.as_ref()
                            .and_then(|to| self.world.room(to.as_str()))
                            .map(|r| r.name.clone())
                            .unwrap_or_default();
                    (e.direction.name().to_string(), name)
                })
                .collect(),
            doors: self
                .visible_exits()
                .iter()
                .filter_map(|(_, e)| {
                    let door = self.world.thing(e.door.as_ref()?.as_str())?;
                    if self.state.is_concealed(door.id.as_str()) {
                        return None;
                    }
                    Some((e.direction.name().to_string(), self.nouns_of(door)))
                })
                .collect(),
            back: self.state.previous.as_ref().and_then(|prev| {
                self.visible_exits()
                    .iter()
                    .find(|(_, e)| e.to.as_ref() == Some(prev))
                    .map(|(_, e)| e.direction.name().to_string())
            }),
        }
    }

    /// A thing of the story, wherever it is: its current name, its aliases
    /// and its location. `None` for an unknown id.
    pub(crate) fn thing_info(&self, id: &str) -> Option<ThingInfo> {
        let id = &self.world.thing(id)?.id;
        let name = self.name_of(id);
        let lower = name.to_lowercase();
        let mut aliases = Vec::new();
        for noun in (self.world.thing(id.as_str()).into_iter())
            .flat_map(|t| self.nouns_of(t))
            .filter(|n| *n != lower)
        {
            if !aliases.contains(&noun) {
                aliases.push(noun);
            }
        }
        Some(ThingInfo {
            id: id.to_string(),
            name,
            aliases,
            location: (self.state.location_of(id.as_str()).cloned()).unwrap_or(Location::Nowhere),
        })
    }

    pub fn scene(&self) -> Scene {
        let ev = self.eval();
        let info = |id: &Id| self.thing_info(id.as_str());
        let scope = self.in_scope();
        Scene {
            room: self.state.room.to_string(),
            room_name: self.room_def().map(|r| r.name.clone()).unwrap_or_default(),
            time: ev.time_of_day().to_string(),
            phase: ev.phase(),
            day: self.state.minutes / TimeOfDay::MINUTES_PER_DAY + 1,
            minutes: self.state.minutes,
            turns: self.state.turns,
            dark: self.is_dark(),
            things: scope
                .iter()
                .filter(|id| !self.state.is_carried(id.as_str()))
                .filter_map(info)
                .collect(),
            inventory: scope
                .iter()
                .filter(|id| self.state.is_carried(id.as_str()))
                .filter_map(info)
                .collect(),
            // A message exit leads nowhere, so the player does not see it as a way out.
            exits: self
                .visible_exits()
                .iter()
                .filter_map(|(_, e)| {
                    let to = e.to.as_ref()?;
                    Some(ExitInfo {
                        direction: e.direction.name().to_string(),
                        to: to.to_string(),
                        to_name: self
                            .world
                            .room(to.as_str())
                            .map(|r| r.name.clone())
                            .unwrap_or_default(),
                    })
                })
                .collect(),
            ended: self.state.ended,
        }
    }

    /// How the current room's description is assembled right now: every
    /// fragment, whether it shows, and why not. Does not change any state.
    pub(crate) fn assembly(&self) -> Vec<FragmentView> {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return Vec::new();
        };
        let room = &self.world.story.rooms[ri];
        let fragments = &self.world.rooms[ri].description;
        let ev = self.eval();
        let first = self.state.visits_to(room.id.as_str()) <= 1;
        let mut shown = text::visible(&ev, fragments);
        // Text on screen counts as shown, even when it was once-only and is used up now.
        if let Some((last_room, last)) = &self.last_shown
            && *last_room == room.id
        {
            for i in last {
                if !shown.contains(i) {
                    shown.push(*i);
                }
            }
        }
        let dark = self.is_dark();
        room.description
            .iter()
            .zip(fragments.iter())
            .enumerate()
            .map(|(i, (src, f))| {
                let is_shown = !dark && shown.contains(&i);
                let seen = self.state.progress.get(&f.key).map_or(0, |p| p.shown);
                let why = if is_shown {
                    None
                } else if dark {
                    Some("the room is dark".to_string())
                } else if f.mode == loremaster_model::FragmentMode::FirstVisit && !first {
                    Some("first visit only".to_string())
                } else if f.mode == loremaster_model::FragmentMode::Once && seen > 0 {
                    Some("already shown once".to_string())
                } else if !ev.holds_opt(&f.when) {
                    Some(format!(
                        "{} is false",
                        f.when_src.as_deref().unwrap_or("condition")
                    ))
                } else {
                    let by = shown.iter().find(|&&j| {
                        j > i && fragments[j].replaces && fragments[j].layer == f.layer
                    });
                    Some(by.map_or("hidden".to_string(), |j| format!("replaced by {}", j + 1)))
                };
                FragmentView {
                    index: i,
                    layer: f.layer,
                    mode: f.mode,
                    condition: src.when.clone(),
                    replaces: src.replaces,
                    text: src.variants.join(" / "),
                    variants: f
                        .variants
                        .iter()
                        .map(|segs| {
                            let mut pieces = Vec::new();
                            text::outline(&ev, segs, true, &mut pieces);
                            pieces
                        })
                        .collect(),
                    shown: is_shown,
                    why,
                }
            })
            .collect()
    }

    /// The things and exits that make the objects and exits layers of the
    /// current room, with what shows and why. Does not change any state.
    pub(crate) fn room_parts(&self) -> RoomParts {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return RoomParts {
                objects: Vec::new(),
                exits: Vec::new(),
            };
        };
        let story = &self.world.story;
        let room = &story.rooms[ri];
        let ev = self.eval();
        let dark = self.is_dark();

        // The room text as it reads now, to check which things it mentions.
        // A group's text counts while the conditions of its candidates hold.
        let fragments = &self.world.rooms[ri].description;
        let room_text = text::visible(&ev, fragments)
            .iter()
            .flat_map(|&i| {
                let f = &fragments[i];
                f.variants
                    .iter()
                    .zip(&f.gates)
                    .filter(|(_, gate)| {
                        gate.iter()
                            .all(|&w| text::holds_for_player(&ev, &f.inner_when[w]))
                    })
                    .map(|(v, _)| text::render(&ev, v))
            })
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let objects = story
            .things
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                self.state.is_in_room(t.id.as_str(), room.id.as_str())
                    && Some(&t.id) != self.state.active.as_ref()
            })
            .map(|(ti, t)| {
                let presence = &self.world.things[ti].presence;
                let generated = format!("There is {} here.", self.eval().thing_name(t.id.as_str(), loremaster_lang::NameForm::A));
                let text = if t.presence.is_empty() {
                    generated
                } else {
                    t.presence
                        .iter()
                        .flat_map(|f| f.variants.iter().cloned())
                        .collect::<Vec<_>>()
                        .join(" / ")
                };
                let why = if !t.listed {
                    let door_of = room.exits.iter().find(|e| e.door.as_ref() == Some(&t.id));
                    Some(if let Some(exit) = door_of {
                        format!("not listed: it is the door of the {} exit", exit.direction.name())
                    } else if self.nouns_of(t).iter().any(|n| mentions(&room_text, n)) {
                        "not listed: the room text mentions it".to_string()
                    } else {
                        "not listed, and the room text does not mention it: the player cannot find it"
                            .to_string()
                    })
                } else if dark {
                    Some("the room is dark".to_string())
                } else if !t.presence.is_empty() && text::visible(&ev, presence).is_empty() {
                    Some("no presence text applies now".to_string())
                } else {
                    None
                };
                PartView {
                    id: t.id.to_string(),
                    label: t.name.clone(),
                    text,
                    detail: t.portable.then(|| "portable".to_string()),
                    shown: why.is_none(),
                    why,
                }
            })
            .collect();

        let exits = room
            .exits
            .iter()
            .enumerate()
            .map(|(ei, e)| {
                let to = match &e.to {
                    Some(to) => story
                        .room(to.as_str())
                        .map_or(to.to_string(), |r| r.name.clone()),
                    None => "nowhere (message exit)".to_string(),
                };
                let visible = ev.holds_opt(&self.world.rooms[ri].exits[ei].visible);
                let door = e.door.as_ref().map(|d| {
                    let state = if self.state.flag(&format!("{d}.locked")) {
                        "locked"
                    } else if self.state.flag(&format!("{d}.open")) {
                        "open"
                    } else {
                        "closed"
                    };
                    format!("door: {}, {state}", self.name_of(d))
                });
                let passable = ev.holds_opt(&self.world.rooms[ri].exits[ei].passable);
                let detail = match (door, passable) {
                    (Some(d), true) => Some(d),
                    (Some(d), false) => Some(format!("{d}; blocked now")),
                    (None, false) => Some("blocked now".to_string()),
                    (None, true) => None,
                };
                PartView {
                    id: e.direction.name().to_string(),
                    label: format!("{} to {to}", e.direction.name()),
                    text: e.direction.name().to_string(),
                    detail,
                    shown: visible,
                    why: (!visible).then(|| {
                        format!("{} is false", e.visible.as_deref().unwrap_or("condition"))
                    }),
                }
            })
            .collect();

        RoomParts { objects, exits }
    }

    /// Run an author command. Its result shows in the transcript like a turn,
    /// marked as an author action, and the turn log records it.
    pub(crate) fn author(&mut self, cmd: AuthorCommand) -> Turn {
        let mut turn = Turn::default();
        let note = |turn: &mut Turn, text: String| {
            turn.log.push(LogEntry {
                kind: "author".into(),
                text: text.clone(),
            });
            turn.say(BlockKind::Author, text);
        };
        match cmd {
            AuthorCommand::Goto { room } => {
                let Some(r) = self.world.room(&room) else {
                    turn.say(BlockKind::Response, format!("There is no room `{room}`."));
                    return turn;
                };
                note(&mut turn, format!("go to {}", r.name));
                self.state.room = r.id.clone();
                self.place_active();
                *self
                    .state
                    .visits
                    .entry(self.state.room.clone())
                    .or_default() += 1;
                self.describe_room(&mut turn);
            }
            AuthorCommand::Set { name, value } => {
                if let Some(id) = name.strip_prefix("timer.")
                    && self.world.story.timer(id).is_some()
                {
                    let left = value.as_int().unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32;
                    note(&mut turn, format!("set {name} = {left}"));
                    if left == 0 {
                        self.state.timers.remove(id);
                    } else {
                        self.state.timers.insert(Id::new(id), left);
                    }
                    return self.finish(turn);
                }
                if !self.state.facts.contains_key(&name) {
                    turn.say(
                        BlockKind::Response,
                        format!("There is no variable `{name}`."),
                    );
                    return turn;
                }
                note(&mut turn, format!("set {name} = {value}"));
                self.set_fact(&mut turn, &name, value);
            }
            AuthorCommand::Time { time } => match time.parse::<TimeOfDay>() {
                Ok(t) => {
                    note(&mut turn, format!("set the time to {t}"));
                    let day = self.state.minutes - self.state.minutes % TimeOfDay::MINUTES_PER_DAY;
                    self.state.minutes = day + u32::from(t.0);
                }
                Err(e) => turn.say(BlockKind::Response, e),
            },
            AuthorCommand::Give { thing } => {
                let Some(t) = self.world.thing(&thing) else {
                    turn.say(BlockKind::Response, format!("There is no thing `{thing}`."));
                    return turn;
                };
                note(&mut turn, format!("give {}", t.name));
                let id = t.id.clone();
                self.move_thing(&id, Location::Carried);
            }
            AuthorCommand::Situation { room, values } => {
                if !crate::matrix::apply_cell(self, &room, &values) {
                    turn.say(BlockKind::Response, format!("There is no room `{room}`."));
                    return turn;
                }
                let list: Vec<String> = values.iter().map(|(n, v)| format!("{n} = {v}")).collect();
                note(&mut turn, format!("try {room}: {}", list.join(", ")));
                self.describe_room(&mut turn);
            }
        }
        self.finish(turn)
    }

    /// The player holds as many things as the story's carry limit allows.
    pub(crate) fn hands_full(&self) -> bool {
        (self.world.story.carry_limit).is_some_and(|limit| self.state.held_count() >= limit)
    }

    pub(crate) fn verbs(&self) -> Vec<VerbInfo> {
        self.world.verbs.iter().map(verb_info).collect()
    }

    /// The verbs that apply to a thing now (#235): see [`crate::Session::verbs_for`].
    pub(crate) fn verbs_for(&self, id: &str) -> Vec<VerbInfo> {
        use loremaster_model::EventPattern;
        let Some(t) = self.world.thing(id) else {
            return Vec::new();
        };
        if !self.reachable(id) {
            return Vec::new();
        }
        let named = |o: &Option<Id>| o.as_ref().is_some_and(|o| o.as_str() == id);
        let mut ids: std::collections::BTreeSet<&str> = (self.world.story.rules.iter())
            .filter_map(|r| match &r.on {
                EventPattern::Verb(p) | EventPattern::Check(p)
                    if crate::explore::for_player(p)
                        && (named(&p.direct) || named(&p.indirect)) =>
                {
                    Some(p.verb.as_str())
                }
                _ => None,
            })
            .collect();
        ids.extend(t.frames.keys().map(Id::as_str));
        let has = |p: &str| self.state.facts.contains_key(&format!("{id}.{p}"));
        let flag = |p: &str| self.state.flag(&format!("{id}.{p}"));
        let lock_verb = |target: &str| {
            if self.state.flag(&format!("{target}.locked")) {
                "unlock"
            } else {
                "lock"
            }
        };
        let scope = self.in_scope();
        let in_scope = |o: &Id| scope.contains(o);
        ids.insert("examine");
        // As the default take: not behind a closed container, hands not full.
        if t.portable
            && !self.state.is_held(id)
            && self.state.closed_around(id, false).is_none()
            && !self.hands_full()
        {
            ids.insert("take");
        }
        if self.state.is_carried(id) {
            ids.extend(["drop", "put"]);
        }
        if t.container || t.supporter {
            ids.insert("put");
        }
        if has("open") && !self.is_playable(id) {
            if flag("open") {
                ids.insert("close");
            } else if !flag("locked") {
                ids.insert("open");
            }
        }
        // A lock and its key, both in scope (in_scope leaves out concealed things).
        if t.key.as_ref().is_some_and(in_scope) && has("locked") {
            ids.extend([lock_verb(id), "use"]);
        }
        for door in scope.iter().filter_map(|d| self.world.thing(d.as_str())) {
            let locks = self
                .state
                .facts
                .contains_key(&format!("{}.locked", door.id));
            if locks && door.key.as_ref().is_some_and(|k| k.as_str() == id) {
                ids.extend([lock_verb(door.id.as_str()), "use"]);
            }
        }
        if has("lit") {
            ids.extend([if flag("lit") { "turn_off" } else { "turn_on" }, "use"]);
        }
        if self.can_talk(id) {
            ids.insert("talk");
        }
        if !self.searchable(id).is_empty() {
            ids.insert("search");
        }
        // A frame or rule for a verb the story does not have adds nothing.
        ids.into_iter()
            .filter_map(|v| self.world.verb(v).map(verb_info))
            .collect()
    }

    // ---- text -----------------------------------------------------------

    fn assemble(&mut self, turn: &mut Turn, fragments: &CompiledText) -> Vec<Run> {
        self.assemble_with_picks(turn, fragments).0
    }

    /// Show a text in play: the core chooses, the game records the progress
    /// and sets what the player learned. Also the fragments that showed.
    fn assemble_with_picks(
        &mut self,
        turn: &mut Turn,
        fragments: &CompiledText,
    ) -> (Vec<Run>, Vec<usize>) {
        let world = self.world.clone();
        let mut rng = self.state.rng;
        let chosen = text::choose(&self.eval(), &fragments.pool, &world.pools, &mut rng);
        let runs = text::runs(&self.eval(), &chosen, &fragments.pool.id, &Run::plain(""));
        let mut shown: Vec<usize> = Vec::new();
        for &i in &chosen.order {
            let f = chosen.selection.shown[i].leaf.fragment;
            if !shown.contains(&f) {
                shown.push(f);
            }
        }
        self.state.rng = rng;
        let learned = text::commit(&mut self.state, &chosen);
        self.learn(turn, learned);
        (runs, shown)
    }

    /// Show the named text pool `id` in play, its lines like `base`.
    pub(crate) fn say_pool(&mut self, turn: &mut Turn, id: &str, base: &Run) -> Vec<Run> {
        let world = self.world.clone();
        let mut rng = self.state.rng;
        let chosen = text::choose_named(&self.eval(), id, &world.pools, &mut rng);
        let runs = text::runs(&self.eval(), &chosen, &format!("text:{id}"), base);
        self.state.rng = rng;
        let learned = text::commit(&mut self.state, &chosen);
        self.learn(turn, learned);
        runs
    }

    /// The player learned these story variables from a text: they become true.
    fn learn(&mut self, turn: &mut Turn, vars: Vec<String>) {
        for var in vars {
            self.set_fact(turn, &var, Value::Bool(true));
        }
    }

    fn render(&self, segments: &[loremaster_lang::Segment]) -> String {
        text::render(&self.eval(), segments)
    }

    fn describe_room(&mut self, turn: &mut Turn) {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return;
        };
        let world = self.world.clone();
        let room = &world.story.rooms[ri];
        turn.say(BlockKind::Title, room.name.clone());
        if self.is_dark() {
            turn.say(BlockKind::Prose, "It is pitch dark. You can't see a thing.");
        } else {
            let (runs, shown) = self.assemble_with_picks(turn, &world.rooms[ri].description);
            self.last_shown = Some((room.id.clone(), shown));
            turn.push(BlockKind::Prose, runs);
            let mut objects = Vec::new();
            for (ti, thing) in world.story.things.iter().enumerate() {
                let direct = self.state.location_of(thing.id.as_str())
                    == Some(&Location::Room(room.id.clone()))
                    || self
                        .eval()
                        .is_backdrop_here(thing.id.as_str(), room.id.as_str());
                if !direct
                    || self.state.is_concealed(thing.id.as_str())
                    || Some(&thing.id) == self.state.active.as_ref()
                {
                    continue;
                }
                let contents = self.contents_text(thing);
                if !thing.listed {
                    if let Some(text) = contents {
                        if !objects.is_empty() {
                            objects.push(Run::plain(" "));
                        }
                        objects.push(Run::plain(text));
                    }
                    continue;
                }
                let mut runs = self.assemble(turn, &world.things[ti].presence);
                if runs.is_empty() {
                    let a = self
                        .eval()
                        .thing_name(thing.id.as_str(), loremaster_lang::NameForm::A);
                    runs.push(Run::plain(
                        if thing.proper || self.eval().is_proper(thing.id.as_str()) {
                            format!("{a} is here.")
                        } else {
                            format!("There is {a} here.")
                        },
                    ));
                }
                if !objects.is_empty() {
                    objects.push(Run::plain(" "));
                }
                objects.extend(runs);
                if let Some(text) = contents {
                    objects.push(Run::plain(" "));
                    objects.push(Run::plain(text));
                }
            }
            turn.push(BlockKind::Objects, objects);
        }
        let exits: Vec<String> = self
            .visible_exits()
            .iter()
            .map(|(_, e)| e.direction.name().to_string())
            .collect();
        if !exits.is_empty() {
            let mut runs = vec![Run::plain("Exits: ")];
            for (i, dir) in exits.iter().enumerate() {
                if i > 0 {
                    runs.push(Run::plain(", "));
                }
                runs.push(Run {
                    text: dir.clone(),
                    source: None,
                    link: Some(Link {
                        kind: LinkKind::Exit,
                        id: dir.clone(),
                    }),
                    speaker: None,
                    emphasis: false,
                });
            }
            runs.push(Run::plain("."));
            turn.push(BlockKind::Exits, runs);
        }
    }

    // ---- state changes --------------------------------------------------

    pub(crate) fn set_fact(&mut self, turn: &mut Turn, name: &str, value: Value) {
        let value = self.clamped(name, value);
        turn.log.push(LogEntry {
            kind: "set".into(),
            text: format!("{name} = {value}"),
        });
        self.state.facts.insert(name.to_string(), value);
    }

    /// The value within the `min` and `max` of its variable. A limit that
    /// names a variable reads its value now. If the limits cross, `min` wins.
    pub(crate) fn clamped(&self, name: &str, value: Value) -> Value {
        let (&Value::Int(mut n), Some(var)) = (&value, self.world.story.var(name)) else {
            return value;
        };
        let limit = |l: &Option<Limit>| match l.as_ref()? {
            Limit::Value(n) => Some(*n),
            Limit::Var(v) => self.state.facts.get(v.as_str())?.as_int(),
        };
        if let Some(max) = limit(&var.max) {
            n = n.min(max);
        }
        if let Some(min) = limit(&var.min) {
            n = n.max(min);
        }
        Value::Int(n)
    }

    pub(crate) fn move_thing(&mut self, id: &Id, to: Location) {
        self.state.locations.insert(id.clone(), to);
    }

    fn enter_room(&mut self, turn: &mut Turn, depth: u8) {
        *self
            .state
            .visits
            .entry(self.state.room.clone())
            .or_default() += 1;
        self.describe_room(turn);
        let room = self.state.room.clone();
        self.fire(turn, &Event::Enter(room), depth);
    }

    pub(crate) fn move_player(&mut self, turn: &mut Turn, to: Id, depth: u8) {
        let from = self.state.room.clone();
        self.state.previous = Some(from.clone());
        self.fire(turn, &Event::Leave(from), depth);
        self.state.room = to;
        self.place_active();
        self.enter_room(turn, depth);
    }

    /// Run every rule that listens for the event, in rule order.
    pub(crate) fn fire(&mut self, turn: &mut Turn, event: &Event, depth: u8) -> Fired {
        let mut out = Fired::default();
        if depth > MAX_RULE_DEPTH || self.state.ended {
            return out;
        }
        let world = self.world.clone();
        // A `stop` ends this run of rules only: keep the flag of the run around it.
        let outer = std::mem::replace(&mut self.stopped, false);
        // A machine takes one step for one event, even when its new state
        // matches another step.
        let mut stepped: Vec<&Id> = Vec::new();
        let mut ran: Vec<usize> = Vec::new();
        for (at, &i) in world.rule_order.iter().enumerate() {
            if self.stopped || self.state.ended {
                break;
            }
            let rule = &world.story.rules[i];
            if !event.matches(rule) || (reads_fired(rule) && self.state.fired.contains(&rule.id)) {
                continue;
            }
            if rule.machine.as_ref().is_some_and(|m| stepped.contains(&m)) {
                continue;
            }
            if !self.within_holds(&world.rules[i].within, event)
                || !self.eval().holds_opt(&world.rules[i].when)
            {
                continue;
            }
            if any_verb(rule) && self.exact_verb_fires(event, at, &ran, &stepped) {
                continue;
            }
            ran.push(i);
            turn.log.push(LogEntry {
                kind: "rule".into(),
                text: rule.id.to_string(),
            });
            self.state.fired.insert(rule.id.clone());
            out.instead |= rule.instead;
            // Only a rule on the command itself makes it free, not an
            // enter, leave or custom event rule that the command sets off.
            self.free |= rule.free && matches!(event, Event::Verb { .. });
            stepped.extend(rule.machine.as_ref());
            // A paused rule waits; the other rules of the event still run.
            if self.run_effects(turn, i, 0, depth + 1)
                && let Some(m) = &rule.machine
            {
                self.run_next(turn, m, depth + 1);
            }
        }
        // Behaviors run after the rules of the event.
        let kind = event.kind();
        for (b, manifest) in world.behaviors.iter().zip(&world.manifests) {
            if self.stopped || self.state.ended {
                break;
            }
            if !manifest.handles(kind) {
                continue;
            }
            let mut cx = crate::Cx {
                owner: Id::new(format!("behavior:{}", b.id())),
                game: self,
                turn,
                depth,
                manifest,
            };
            if b.handle(event, &mut cx) == crate::Flow::Stop {
                self.stopped = true;
            }
        }
        out.stopped = self.stopped;
        self.stopped = outer;
        out
    }

    /// Does a rule with an exact verb in the scope of the `*` rule at place
    /// `at` of the rule order fire for the event? Then it holds the `*` rule
    /// back (#192). `ran` holds the rules that ran for the event so far.
    /// shortcut: a rule after `at` is judged by its condition now, before the
    /// rules between change the state; upgrade if that bites.
    fn exact_verb_fires(&self, event: &Event, at: usize, ran: &[usize], stepped: &[&Id]) -> bool {
        let world = &self.world;
        let order = &world.rule_order;
        let within = &world.rules[order[at]].within;
        let exact = |j: usize| !any_verb(&world.story.rules[j]) && world.rules[j].within == *within;
        // The rules in `ran` matched the event already.
        ran.iter().any(|&j| exact(j))
            || order[at + 1..].iter().any(|&j| {
                let rule = &world.story.rules[j];
                exact(j)
                    && event.matches(rule)
                    && !(reads_fired(rule) && self.state.fired.contains(&rule.id))
                    && !rule.machine.as_ref().is_some_and(|m| stepped.contains(&m))
                    && self.eval().holds_opt(&world.rules[j].when)
            })
    }

    /// Does the scope of a rule hold for the event? A thing rule needs the
    /// thing as an object of the verb, or present for other events. A room or
    /// region rule needs the player there.
    fn within_holds(&self, within: &Option<Within>, event: &Event) -> bool {
        match within {
            None => true,
            Some(Within::Thing(t)) => match event.objects() {
                Some(objects) => objects.contains(t),
                None => self.present(t.as_str()),
            },
            // A command of another actor happens where that actor is.
            Some(Within::Room(r)) => self.room_of_event(event).as_ref() == Some(r),
            Some(Within::Region(g)) => {
                let room = self.room_of_event(event);
                let def = room.and_then(|r| self.world.room(r.as_str()));
                def.and_then(|r| r.region.as_ref()) == Some(g)
            }
        }
    }

    /// The room where an event happens: the room of the actor of a command,
    /// else the player's room.
    fn room_of_event(&self, event: &Event) -> Option<Id> {
        match event {
            Event::Verb { actor, .. } if actor.as_str() != crate::PLAYER => {
                self.actor_room(actor.as_str())
            }
            _ => Some(self.state.room.clone()),
        }
    }

    /// The thing is carried or in the current room.
    fn present(&self, thing: &str) -> bool {
        self.eval().present(thing)
    }

    /// "The trophy case contains a jewel." for a container or supporter
    /// whose contents the player can see. None when it is empty or closed.
    fn contents_text(&self, holder: &loremaster_model::Thing) -> Option<String> {
        if !(holder.container || holder.supporter) {
            return None;
        }
        let id = holder.id.as_str();
        let closed = self.state.facts.get(&format!("{id}.open")) == Some(&Value::Bool(false))
            && !self.state.flag(&format!("{id}.transparent"));
        if holder.container && closed {
            return None;
        }
        let items: Vec<String> = self
            .state
            .contents(id)
            .iter()
            .filter(|c| !self.state.is_concealed(c.as_str()))
            .map(|c| {
                self.eval()
                    .thing_name(c.as_str(), loremaster_lang::NameForm::A)
            })
            .collect();
        if items.is_empty() {
            return None;
        }
        let list = text::list(&items, "and");
        Some(if holder.container {
            format!("{} contains {list}.", self.the(&holder.id, true))
        } else {
            let verb = if items.len() == 1 { "is" } else { "are" };
            format!("On {} {verb} {list}.", self.the(&holder.id, false))
        })
    }

    /// Put a thing in or on another one: the default action of `put`.
    fn put(&mut self, turn: &mut Turn, objects: &[Id]) {
        let say = |turn: &mut Turn, text: String| turn.say(BlockKind::Response, text);
        let [what, into] = objects else {
            say(turn, "Put it where?".into());
            return;
        };
        let Some(holder) = self.world.thing(into.as_str()).cloned() else {
            say(turn, "You can't see anything like that here.".into());
            return;
        };
        let (the_what, the_into) = (self.the(what, false), self.the(into, false));
        if !(holder.container || holder.supporter) {
            say(turn, format!("You can't put things in {the_into}."));
            return;
        }
        if !self.state.is_carried(what.as_str()) {
            say(turn, format!("You don't have {the_what}."));
            return;
        }
        // The holder must not be the thing, or inside it.
        // A loop of holders is an authoring error; stop after as many steps as `State::outer_location`.
        let mut at = Some(into.clone());
        for _ in 0..16 {
            let Some(h) = at else { break };
            if h == *what {
                say(turn, format!("You can't put {the_what} inside itself."));
                return;
            }
            at = self
                .state
                .locations
                .get(&h)
                .and_then(|l| l.holder().cloned());
        }
        if holder.container
            && self.state.facts.get(&format!("{into}.open")) == Some(&Value::Bool(false))
        {
            say(turn, format!("{} is closed.", self.the(into, true)));
            return;
        }
        if let Some(cap) = holder.capacity
            && self.state.contents(into.as_str()).len() >= cap as usize
        {
            say(
                turn,
                format!(
                    "There is no room {} {the_into}.",
                    if holder.container { "in" } else { "on" }
                ),
            );
            return;
        }
        let to = if holder.container {
            Location::In(into.clone())
        } else {
            Location::On(into.clone())
        };
        turn.log.push(LogEntry {
            kind: "move".into(),
            text: format!("{what} -> {}", to.to_word()),
        });
        self.move_thing(what, to);
        say(turn, "Done.".into());
    }

    /// Run a behavior at this place in a rule (`call`). A direct call does not
    /// depend on the event kinds of the manifest. `Flow::Stop` works as the
    /// `stop` effect.
    fn call_behavior(&mut self, turn: &mut Turn, rule: &Id, id: &Id, depth: u8) {
        let world = self.world.clone();
        let Some(i) = world.behaviors.iter().position(|b| b.id() == id.as_str()) else {
            turn.log.push(LogEntry {
                kind: "author".into(),
                text: if world.is_registered(id.as_str()) {
                    format!("{rule} calls {id}, which the story does not name under `behaviors`")
                } else {
                    format!("{rule} calls {id}, which this build does not have")
                },
            });
            return;
        };
        let (b, manifest) = (&world.behaviors[i], &world.manifests[i]);
        let mut cx = crate::Cx {
            owner: Id::new(format!("behavior:{id}")),
            game: self,
            turn,
            depth,
            manifest,
        };
        if b.handle(&Event::Call(rule.clone()), &mut cx) == crate::Flow::Stop {
            self.stopped = true;
        }
    }

    fn apply(&mut self, turn: &mut Turn, rule: &Id, effect: &CompiledEffect, depth: u8) {
        match effect {
            CompiledEffect::Set(values) => {
                for (k, v) in values {
                    self.set_fact(turn, k, v.clone());
                }
            }
            CompiledEffect::Add(values) => {
                for (k, n) in values {
                    let cur = self.state.facts.get(k).and_then(Value::as_int).unwrap_or(0);
                    self.set_fact(turn, k, Value::Int(cur.saturating_add(*n)));
                }
            }
            CompiledEffect::Calc(sums) => {
                // Each sum reads the values from before the effect.
                let int = |name: &str| self.state.facts.get(name).and_then(Value::as_int);
                let totals: Vec<_> = sums
                    .iter()
                    .map(|(name, terms)| {
                        let total = terms.iter().fold(0i64, |sum, (sign, term)| {
                            let n = match term {
                                Term::Int(n) => *n,
                                Term::Name(v) => int(v).unwrap_or(0),
                            };
                            sum.saturating_add(n.saturating_mul(*sign))
                        });
                        (name.clone(), total)
                    })
                    .collect();
                for (name, total) in totals {
                    self.set_fact(turn, &name, Value::Int(total));
                }
            }
            CompiledEffect::Say(line) => {
                let runs = text::line_runs(&self.eval(), line, &rule_run(String::new(), rule));
                turn.push(BlockKind::Narration, runs);
            }
            CompiledEffect::SayPool(id) => {
                let runs = self.say_pool(turn, id.as_str(), &rule_run(String::new(), rule));
                turn.push(BlockKind::Narration, runs);
            }
            CompiledEffect::Move(moves) => {
                for (thing, to) in moves {
                    let loc = Location::from_word(to.as_str());
                    turn.log.push(LogEntry {
                        kind: "move".into(),
                        text: format!("{thing} -> {to}"),
                    });
                    self.move_thing(thing, loc);
                }
            }
            CompiledEffect::Goto(room) => self.move_player(turn, room.clone(), depth),
            CompiledEffect::End {
                ending,
                title,
                text,
            } => {
                // Two blocks, so that each front end shows the title as its own line.
                let title = self.render(title);
                let text = self.render(text);
                turn.push(BlockKind::End, vec![rule_run(title, rule)]);
                if !text.is_empty() {
                    turn.push(BlockKind::End, vec![rule_run(text, rule)]);
                }
                self.state.ended = true;
                self.state.ending = Some(ending.clone());
                // An ended story answers no await.
                self.state.awaiting = None;
                self.state.combat = None;
            }
            CompiledEffect::Stop => self.stopped = true,
            CompiledEffect::PlaySound(cue) => turn.sounds.push(cue.clone()),
            CompiledEffect::Call(id) => self.call_behavior(turn, rule, id, depth),
            // Only a rule can pause (the fire loop handles it there); the checks
            // reject `await` in timers and dialogues.
            CompiledEffect::EndCombat(room) if self.state.combat.is_some() => {
                self.combat_exit = Some(room.clone());
            }
            CompiledEffect::EndCombat(_) => turn.log.push(LogEntry {
                kind: "combat".into(),
                text: format!("{rule}: `end_combat` without a combat"),
            }),
            CompiledEffect::Await(_) | CompiledEffect::Combat(_) => turn.log.push(LogEntry {
                kind: "await".into(),
                text: format!("{rule}: `await` and `combat` work only in rules"),
            }),
            CompiledEffect::Emit(name) => {
                turn.log.push(LogEntry {
                    kind: "event".into(),
                    text: name.to_string(),
                });
                self.fire(turn, &Event::Custom(name.clone()), depth);
            }
            CompiledEffect::Start(timers) => {
                for (id, turns) in timers {
                    let Some(t) = self.world.story.timer(id.as_str()) else {
                        continue;
                    };
                    let turns = turns.unwrap_or(t.turns);
                    turn.log.push(LogEntry {
                        kind: "timer".into(),
                        text: format!("{id} starts at {turns}"),
                    });
                    self.state.timers.insert(id.clone(), turns);
                }
            }
            CompiledEffect::Cancel(id) => {
                if self.state.timers.remove(id).is_some() {
                    turn.log.push(LogEntry {
                        kind: "timer".into(),
                        text: format!("{id} is cancelled"),
                    });
                }
            }
        }
    }

    /// One turn passes: the clock moves by `minutes`, turn and time rules fire.
    fn advance(&mut self, turn: &mut Turn, minutes: u16) {
        if self.state.ended {
            return;
        }
        let first = turn.blocks.len();
        let before = self.state.minutes;
        self.state.minutes += u32::from(minutes);
        self.state.turns += 1;
        let after = self.state.minutes;
        self.fire(turn, &Event::Turn, 0);
        self.tick_timers(turn);
        self.fire(turn, &Event::Time { before, after }, 0);
        // Actors act after the clock: a step at midnight runs in the turn
        // that reaches midnight.
        self.run_agendas(turn);
        self.run_orders(turn);
        turn.caused_by_world(first);
    }

    /// Each running timer whose `runs_while` holds goes down by one. A timer at
    /// zero runs out: its effects at 0 run, then its rules fire.
    fn tick_timers(&mut self, turn: &mut Turn) {
        let world = self.world.clone();
        for (ti, t) in world.story.timers.iter().enumerate() {
            if self.state.ended {
                return;
            }
            let Some(&left) = self.state.timers.get(&t.id) else {
                continue;
            };
            if !self.eval().holds_opt(&world.timers[ti].runs_while) {
                continue;
            }
            let left = left.saturating_sub(1);
            match (left, t.repeat) {
                (0, true) => self.state.timers.insert(t.id.clone(), t.turns),
                (0, false) => self.state.timers.remove(&t.id),
                _ => self.state.timers.insert(t.id.clone(), left),
            };
            let owner = Id::new(format!("timer:{}", t.id));
            // A `stop` (or `Flow::Stop` of a called behavior) ends this step's
            // effects; the timer's rules still fire when it runs out.
            let outer = std::mem::replace(&mut self.stopped, false);
            for effect in world.timers[ti].at.get(&left).into_iter().flatten() {
                self.apply(turn, &owner, effect, 1);
                if self.state.ended {
                    self.stopped = outer;
                    return;
                }
                if self.stopped {
                    break;
                }
            }
            self.stopped = outer;
            if left == 0 {
                turn.log.push(LogEntry {
                    kind: "timer".into(),
                    text: format!("{} runs out", t.id),
                });
                self.fire(turn, &Event::Timer(t.id.clone()), 0);
            }
        }
    }

    fn finish(&mut self, mut turn: Turn) -> Turn {
        self.last_fired = turn
            .log
            .iter()
            .filter(|l| l.kind == "rule")
            .map(|l| Id::new(l.text.as_str()))
            .collect();
        turn.ended = self.state.ended;
        turn.talk = self.talk_info();
        turn.choices = self.choices();
        crate::output::split_emphasis(&mut turn.blocks);
        self.link_things(&mut turn.blocks);
        turn
    }

    /// Mark mentions of things in scope as clickable.
    fn link_things(&self, blocks: &mut [Block]) {
        let mut needles: Vec<(String, String)> = self
            .in_scope()
            .iter()
            .filter_map(|id| self.world.thing(id.as_str()))
            .flat_map(|t| {
                self.nouns_of(t)
                    .into_iter()
                    .map(move |n| (n, t.id.to_string()))
            })
            .collect();
        needles.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));
        for block in blocks.iter_mut() {
            if matches!(
                block.kind,
                BlockKind::Command | BlockKind::Title | BlockKind::Exits | BlockKind::Author
            ) {
                continue;
            }
            let runs = std::mem::take(&mut block.runs);
            for run in runs {
                if run.link.is_some() {
                    block.runs.push(run);
                } else {
                    split_links(run, &needles, &mut block.runs);
                }
            }
        }
    }

    // ---- verbs ----------------------------------------------------------

    fn go(&mut self, turn: &mut Turn, direction: &str) {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return;
        };
        let world = self.world.clone();
        let found = self
            .visible_exits()
            .iter()
            .find(|(_, e)| e.direction.name() == direction)
            .map(|(i, e)| (*i, (*e).clone()));
        let Some((ei, exit)) = found else {
            turn.say(BlockKind::Response, "You can't go that way.");
            return;
        };
        let compiled = &world.rooms[ri].exits[ei];
        let Some(to) = exit.to.clone() else {
            // A message exit: the move only prints its text.
            let text = compiled
                .blocked
                .as_ref()
                .map(|b| self.render(b))
                .unwrap_or_else(|| "You can't go that way.".into());
            turn.say(BlockKind::Response, text);
            return;
        };
        if let Some(door) = &exit.door {
            let the = self.the(door, false);
            let the_cap = self.the(door, true);
            // A concealed door is not named: it opens without a note, and a
            // locked one names only the way, which the exits still list.
            let concealed = self.state.is_concealed(door.as_str());
            if !self.state.flag(&format!("{door}.open")) {
                if self.state.flag(&format!("{door}.locked")) {
                    let text = if concealed {
                        format!("The way {direction} is locked.")
                    } else {
                        format!("{the_cap} is locked.")
                    };
                    turn.say(BlockKind::Response, text);
                    return;
                }
                self.set_fact(turn, &format!("{door}.open"), Value::Bool(true));
                if !concealed {
                    turn.say(BlockKind::Note, format!("(first opening {the})"));
                }
            }
        }
        if !self.eval().holds_opt(&compiled.passable) {
            let text = compiled
                .blocked
                .as_ref()
                .map(|b| self.render(b))
                .unwrap_or_else(|| "You can't go that way.".into());
            turn.say(BlockKind::Response, text);
            return;
        }
        if let Some(t) = &compiled.traversal {
            let text = self.render(t);
            turn.say(BlockKind::Response, text);
        }
        self.move_player(turn, to, 0);
    }

    fn default_action(&mut self, turn: &mut Turn, verb: &str, objects: &[Id]) {
        let say = |turn: &mut Turn, text: String| turn.say(BlockKind::Response, text);
        let first = objects.first();
        let the = first.map(|o| self.the(o, false)).unwrap_or_default();
        let the_cap = first.map(|o| self.the(o, true)).unwrap_or_default();
        let thing = first.and_then(|o| self.world.thing(o.as_str())).cloned();
        let prop = |p: &str| first.map(|o| format!("{o}.{p}")).unwrap_or_default();
        let has = |g: &Game, p: &str| g.state.facts.contains_key(&prop(p));
        match verb {
            "look" => self.describe_room(turn),
            "inventory" => {
                let items: Vec<String> = self
                    .in_scope()
                    .iter()
                    .filter(|id| self.state.is_held(id.as_str()))
                    .map(|id| {
                        let n = self.name_of(id);
                        let inside: Vec<String> = self
                            .state
                            .contents(id.as_str())
                            .iter()
                            .filter(|c| !self.state.is_concealed(c.as_str()))
                            .map(|c| {
                                self.eval()
                                    .thing_name(c.as_str(), loremaster_lang::NameForm::A)
                            })
                            .collect();
                        let open = self.state.facts.get(&format!("{id}.open"))
                            != Some(&Value::Bool(false));
                        if inside.is_empty() || !open {
                            format!("{} {n}", article(&n))
                        } else {
                            format!(
                                "{} {n} (with {} in it)",
                                article(&n),
                                text::list(&inside, "and")
                            )
                        }
                    })
                    .collect();
                if items.is_empty() {
                    say(turn, "You are empty-handed.".into());
                } else {
                    say(
                        turn,
                        format!("You are carrying {}.", text::list(&items, "and")),
                    );
                }
            }
            "wait" => say(turn, "Time passes.".into()),
            "search" => {
                let Some(o) = first else { return };
                let found = self.searchable(o.as_str());
                if found.is_empty() {
                    let closed = self.state.facts.get(&prop("open")) == Some(&Value::Bool(false));
                    if closed && thing.as_ref().is_some_and(|t| t.container) {
                        say(turn, format!("{the_cap} is closed."));
                    } else {
                        say(turn, "You find nothing.".into());
                    }
                    return;
                }
                for c in &found {
                    self.set_fact(turn, &format!("{c}.concealed"), Value::Bool(false));
                }
                let names: Vec<String> = found
                    .iter()
                    .map(|c| {
                        self.eval()
                            .thing_name(c.as_str(), loremaster_lang::NameForm::A)
                    })
                    .collect();
                say(turn, format!("You find {}.", text::list(&names, "and")));
            }
            "score" => {
                let Some(score) = &self.world.story.score else {
                    say(turn, "This story has no score.".into());
                    return;
                };
                let now = self
                    .state
                    .facts
                    .get(score.var.as_str())
                    .and_then(Value::as_int)
                    .unwrap_or(0);
                let moves = self.state.turns;
                let unit = if moves == 1 { "move" } else { "moves" };
                let mut text = format!(
                    "Your score is {now} (total of {} points), in {moves} {unit}.",
                    score.max
                );
                if let Some(rank) = score.rank(now) {
                    text.push_str(&format!(" This gives you the rank of {rank}."));
                }
                say(turn, text);
            }
            "examine" => {
                let Some(o) = first else { return };
                let Some(ti) = self.world.thing_idx(o.as_str()) else {
                    return;
                };
                let world = self.world.clone();
                let runs = self.assemble(turn, &world.things[ti].description);
                let contents = self.contents_text(&world.story.things[ti]);
                if runs.is_empty() && contents.is_none() {
                    say(turn, format!("You see nothing special about {the}."));
                } else if !runs.is_empty() {
                    turn.push(BlockKind::Prose, runs);
                }
                if let Some(text) = contents {
                    say(turn, text);
                }
            }
            "take" => {
                let Some(t) = thing else { return };
                if self.state.is_held(t.id.as_str()) {
                    say(turn, format!("You already have {the}."));
                } else if let Some(c) = self.state.closed_around(t.id.as_str(), false) {
                    let c = c.clone();
                    say(turn, format!("{} is closed.", self.the(&c, true)));
                } else if !t.portable {
                    say(turn, format!("{the_cap} won't budge."));
                } else if self.hands_full() {
                    say(turn, "Your hands are full.".into());
                } else {
                    self.move_thing(&t.id, Location::Carried);
                    turn.log.push(LogEntry {
                        kind: "move".into(),
                        text: format!("{} -> carried", t.id),
                    });
                    say(turn, "Taken.".into());
                }
            }
            "drop" => {
                let Some(t) = thing else { return };
                if !self.state.is_carried(t.id.as_str()) {
                    say(turn, format!("You don't have {the}."));
                } else if let Some(c) = self.state.closed_around(t.id.as_str(), false) {
                    let c = c.clone();
                    say(turn, format!("{} is closed.", self.the(&c, true)));
                } else {
                    let here = Location::Room(self.state.room.clone());
                    turn.log.push(LogEntry {
                        kind: "move".into(),
                        text: format!("{} -> {}", t.id, self.state.room),
                    });
                    self.move_thing(&t.id, here);
                    say(turn, "Dropped.".into());
                }
            }
            "open" | "close" => {
                let opening = verb == "open";
                // A playable actor's `open` fact only hides what it carries.
                if !has(self, "open") || first.is_some_and(|o| self.is_playable(o.as_str())) {
                    say(turn, format!("{the_cap} isn't something you can {verb}."));
                } else if opening && self.state.flag(&prop("locked")) {
                    say(turn, format!("{the_cap} is locked."));
                } else if self.state.flag(&prop("open")) == opening {
                    let word = if opening { "open" } else { "closed" };
                    say(turn, format!("{the_cap} is already {word}."));
                } else {
                    self.set_fact(turn, &prop("open"), Value::Bool(opening));
                    let reveal = thing
                        .as_ref()
                        .filter(|_| opening)
                        .and_then(|t| self.contents_text(t));
                    match reveal {
                        Some(contents) => say(turn, format!("You open {the}. {contents}")),
                        None => {
                            let word = if opening { "open" } else { "close" };
                            say(turn, format!("You {word} {the}."))
                        }
                    }
                }
            }
            "unlock" | "lock" => {
                let locking = verb == "lock";
                let Some(t) = thing else { return };
                let Some(key) = t.key.clone().filter(|_| has(self, "locked")) else {
                    say(turn, format!("{the_cap} has no lock."));
                    return;
                };
                if self.state.flag(&prop("locked")) == locking {
                    let word = if locking { "locked" } else { "unlocked" };
                    say(turn, format!("{the_cap} is already {word}."));
                    return;
                }
                let with = objects.get(1).cloned().or_else(|| {
                    self.state.is_carried(key.as_str()).then(|| {
                        turn.say(
                            BlockKind::Note,
                            format!("(with the {})", self.name_of(&key)),
                        );
                        key.clone()
                    })
                });
                match with {
                    None => say(turn, format!("You have nothing to {verb} {the} with.")),
                    Some(k) if k != key => {
                        say(turn, format!("The {} doesn't fit.", self.name_of(&k)))
                    }
                    Some(_) => {
                        if locking && self.state.flag(&prop("open")) {
                            self.set_fact(turn, &prop("open"), Value::Bool(false));
                        }
                        self.set_fact(turn, &prop("locked"), Value::Bool(locking));
                        say(turn, format!("You {verb} {the}."));
                    }
                }
            }
            "turn_on" | "turn_off" => {
                let on = verb == "turn_on";
                if !has(self, "lit") {
                    say(
                        turn,
                        format!("You can't {} {the}.", if on { "light" } else { "put out" }),
                    );
                } else if self.state.flag(&prop("lit")) == on {
                    say(
                        turn,
                        format!("{the_cap} is already {}.", if on { "lit" } else { "out" }),
                    );
                } else {
                    let was_dark = self.is_dark();
                    self.set_fact(turn, &prop("lit"), Value::Bool(on));
                    say(
                        turn,
                        format!("{the_cap} is now {}.", if on { "lit" } else { "out" }),
                    );
                    if was_dark != self.is_dark() {
                        self.describe_room(turn);
                    }
                }
            }
            "use" => match objects {
                [o] if self.state.facts.contains_key(&format!("{o}.lit")) => {
                    let lit = self.state.flag(&format!("{o}.lit"));
                    self.default_action(turn, if lit { "turn_off" } else { "turn_on" }, objects);
                }
                [k, target]
                    if self
                        .world
                        .thing(target.as_str())
                        .and_then(|t| t.key.as_ref())
                        == Some(k) =>
                {
                    let lock = if self.state.flag(&format!("{target}.locked")) {
                        "unlock"
                    } else {
                        "lock"
                    };
                    self.default_action(turn, lock, &[target.clone(), k.clone()]);
                }
                _ => say(turn, "Nothing happens.".into()),
            },
            "talk" => {
                let started = first.is_some_and(|o| self.start_talk(turn, o));
                if !started {
                    say(turn, format!("{the_cap} has nothing to say."));
                }
            }
            "give" => say(turn, "Nobody seems interested.".into()),
            "put" => self.put(turn, objects),
            "push" | "pull" => say(turn, format!("You {verb} {the}. Nothing happens.")),
            "go" => {}
            custom => {
                let text = self
                    .world
                    .verb(custom)
                    .and_then(|v| v.default.clone())
                    .unwrap_or_else(|| "Nothing happens.".into());
                say(turn, text);
            }
        }
    }
}

fn verb_info(v: &loremaster_model::VerbDef) -> VerbInfo {
    VerbInfo {
        id: v.id.to_string(),
        label: v.label.clone(),
        words: v.words.clone(),
        slots: v.slots,
        preposition: v.preposition.clone(),
        bar: v.bar,
    }
}

/// True when the game reads `State.fired` for this rule: a `once` rule that
/// fired does not fire again. This is the only read of `fired` that changes
/// what happens (the others only drop ids of rules that are gone), and the
/// explorer relies on that to leave the other fired rules out of its state
/// key.
pub(crate) fn reads_fired(rule: &loremaster_model::Rule) -> bool {
    rule.once
}

pub(crate) fn rule_run(text: String, rule: &Id) -> Run {
    Run {
        text,
        source: Some(crate::Source {
            owner: format!("rule:{rule}"),
            fragment: 0,
            layer: loremaster_model::Layer::Event,
            condition: None,
            variant: 0,
        }),
        link: None,
        speaker: None,
        emphasis: false,
    }
}

/// Does `text` contain `noun` as whole words? Both are lowercase.
fn mentions(text: &str, noun: &str) -> bool {
    text.match_indices(noun).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + noun.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Did the clock pass time of day `t` in `(before, after]`?
pub(crate) fn crossed(before: u32, after: u32, t: TimeOfDay) -> bool {
    let day = TimeOfDay::MINUTES_PER_DAY;
    let t = u32::from(t.0);
    let mut start = before - before % day;
    while start <= after {
        let at = start + t;
        if at > before && at <= after {
            return true;
        }
        start += day;
    }
    false
}

/// Split a run at mentions of things, linking each mention.
fn split_links(run: Run, needles: &[(String, String)], out: &mut Vec<Run>) {
    let lower = run.text.to_ascii_lowercase();
    let boundary = |i: usize| i == 0 || !lower.as_bytes()[i - 1].is_ascii_alphanumeric();
    let end_ok = |i: usize| i >= lower.len() || !lower.as_bytes()[i].is_ascii_alphanumeric();
    let hit = needles
        .iter()
        .filter_map(|(n, id)| {
            lower
                .match_indices(n.as_str())
                .find(|(i, _)| boundary(*i) && end_ok(i + n.len()))
                .map(|(i, _)| (i, n.len(), id))
        })
        .min_by_key(|(i, len, _)| (*i, std::cmp::Reverse(*len)));
    let Some((i, len, id)) = hit else {
        out.push(run);
        return;
    };
    let piece = |text: &str, link: Option<Link>| Run {
        text: text.to_string(),
        link,
        ..run.clone()
    };
    if i > 0 {
        out.push(piece(&run.text[..i], None));
    }
    out.push(piece(
        &run.text[i..i + len],
        Some(Link {
            kind: LinkKind::Thing,
            id: id.clone(),
        }),
    ));
    let rest = piece(&run.text[i + len..], None);
    if !rest.text.is_empty() {
        split_links(rest, needles, out);
    }
}

/// The value of a host event's argument before the first report.
fn default_arg(ty: &loremaster_model::ArgType) -> Value {
    use loremaster_model::ArgType;
    match ty {
        ArgType::Bool => Value::Bool(false),
        ArgType::Int => Value::Int(0),
        ArgType::Text => Value::Text(String::new()),
        // The least value: the interface hash does not see the order.
        ArgType::Enum(values) => Value::Text(values.iter().min().cloned().unwrap_or_default()),
    }
}

/// A rule on a verb event whose pattern matches any verb (`*`).
fn any_verb(rule: &loremaster_model::Rule) -> bool {
    matches!(&rule.on, loremaster_model::EventPattern::Verb(p) | loremaster_model::EventPattern::Check(p) if p.any_verb())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn clock_crossing() {
        let t = TimeOfDay::hm(21, 0);
        check!(crossed(20 * 60 + 55, 21 * 60, t));
        check!(!crossed(21 * 60, 21 * 60 + 5, t));
        check!(crossed(23 * 60, 24 * 60 + 21 * 60, t));
    }
}
