use crate::eval::{Eval, to_fact_value};
use crate::parser::{ParseScope, parse};
use crate::text::{self, SourcePiece, article};
use crate::world::{CEffect, CFragment};
use crate::{Block, BlockKind, Command, Link, LinkKind, LogEntry, Run, State, Turn, World};
use dramaturge_model::{Fact, FactId, FactScope};
use dramaturge_runtime::DialogWorldState;
use lantern_model::{EventPattern, Id, Location, TimeOfDay, Value, VerbSlots};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A game in progress.
#[derive(Debug, Clone)]
pub struct Game {
    world: Arc<World>,
    state: State,
    facts: DialogWorldState,
    /// The room fragments in the last description the player saw, so the
    /// assembly panel matches the screen even after once-only text is used up.
    last_shown: Option<(Id, Vec<usize>)>,
    /// Rules that fired in the last turn, for the rules view.
    last_fired: Vec<Id>,
    /// The frame used for the last action, for the assembly panel.
    last_frame: Option<FrameUse>,
}

/// A verb for the verb bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbInfo {
    pub id: String,
    pub label: String,
    pub slots: VerbSlots,
    pub preposition: Option<String>,
    pub bar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThingInfo {
    pub id: String,
    pub name: String,
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
    pub time: String,
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
    pub layer: lantern_model::Layer,
    pub mode: lantern_model::FragmentMode,
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

/// A room on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapRoom {
    pub id: String,
    pub name: String,
    /// Grid position from the story, or a generated one when the story has none.
    pub x: f32,
    pub y: f32,
    /// The level, for towers and dungeons. Up exits go one level higher.
    pub level: i32,
    pub visited: bool,
    pub current: bool,
}

/// A connection between two rooms on the map. Paired exits show as one link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapLink {
    pub from: String,
    pub to: String,
    /// Direction from `from`, for example `north` or `up`.
    pub direction: String,
    /// False while the exit is hidden from the player.
    pub visible: bool,
    /// True when there is no exit back.
    pub one_way: bool,
}

/// The whole map, for the map panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapView {
    pub rooms: Vec<MapRoom>,
    pub links: Vec<MapLink>,
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

/// A rule as When / If / Then, with its state now, for the rules view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleView {
    pub id: String,
    /// The event, in words, for example "the player enters lantern_room".
    pub when: String,
    /// The condition as written.
    pub condition: Option<String>,
    /// True when the condition holds now (or there is none).
    pub condition_holds: bool,
    /// Each effect, in words.
    pub then: Vec<String>,
    pub once: bool,
    pub instead: bool,
    /// True when the rule fired at least one time in this game.
    pub fired: bool,
    /// The kind of event: start, turn, enter, leave, time or verb.
    pub trigger: String,
    /// Why the rule matters here and now, or None. For example "fires when the
    /// player enters this room" or "fired in the last turn".
    pub relevant: Option<String>,
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
    /// The names and values are the ones of [`crate::RoomMatrix`].
    Situation {
        room: String,
        values: Vec<(String, String)>,
    },
}

/// One room for the Atlas: where it is, what is in it now, and its exits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasRoom {
    pub id: String,
    pub name: String,
    pub level: i32,
    pub visited: bool,
    pub current: bool,
    /// The things in the room now. Things that the room text names are here too.
    pub things: Vec<ThingInfo>,
    /// Each exit as its direction and the room it leads to (empty for an exit that only prints a message).
    pub exits: Vec<(String, String)>,
    /// The number of room fragments.
    pub fragments: usize,
}

/// A name that conditions and markup can use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    /// For example `oil_lamp.lit`, `time.phase`, `oil_lamp.the`.
    pub name: String,
    /// `variable`, `property`, `name`, `derived`, `phase` or `markup`.
    pub kind: String,
    /// What it is, in a few words.
    pub detail: String,
    /// The value now, as text, when it has one.
    pub value: Option<String>,
    /// The values to compare with, for example the values of an enum.
    pub values: Vec<String>,
}

/// The result of a check of text that the author is typing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupCheck {
    /// The first error, with its byte range in the text.
    pub error: Option<TextError>,
    /// The rendered text in the current state. Empty for a condition.
    pub text: String,
    /// The source in pieces, with branches that do not hold now marked as not active.
    pub pieces: Vec<SourcePiece>,
    /// For a condition: whether it holds now.
    pub holds: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextError {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

impl MarkupCheck {
    fn failed(e: lantern_lang::LangError) -> Self {
        Self {
            error: Some(TextError {
                start: e.span.start,
                end: e.span.end,
                message: e.message,
            }),
            ..Self::default()
        }
    }
}

/// A variable or thing property with its value now, for the command palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarView {
    pub name: String,
    pub value: Value,
    /// For an enum variable: all values. Empty for other types.
    pub options: Vec<String>,
}

/// The phases of the day with their start times, for the command palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseView {
    pub name: String,
    pub from: String,
}

/// What the author sees about a thing or an exit in a tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inspect {
    /// `thing` or `exit`.
    pub kind: String,
    /// The thing id, or the exit direction.
    pub id: String,
    pub title: String,
    /// Label and value pairs, in display order.
    pub facts: Vec<(String, String)>,
    /// Ids of rules that name this thing or the room of this exit.
    pub rules: Vec<String>,
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
    pub mode: lantern_model::FragmentMode,
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
            action: None,
            last_command: None,
            repeat: 0,
            action_counts: Default::default(),
            interactions: Default::default(),
            talked: Default::default(),
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
        Self::restore(world, state)
    }

    /// Continue the game of `self` in a changed world, for example after an edit of the story.
    ///
    /// The state carries over. A value that is still at its start value in the old world takes
    /// the start value of the new world, so an edit of a start value shows at once. Values and
    /// things that the new world does not have are dropped; new ones start at their start value.
    /// A current room that is gone becomes the start room.
    pub fn carry_over(&self, world: Arc<World>) -> Self {
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
        let has_room = |id: &Id| world.story.room(id.as_str()).is_some();
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
        state.visits.retain(|r, _| has_room(r));
        Self::restore(world, state)
    }

    /// Continue from a saved state.
    pub fn restore(world: Arc<World>, state: State) -> Self {
        let mut facts = DialogWorldState::new();
        for (id, value) in &state.facts {
            facts.set_fact(
                FactScope::World,
                Fact {
                    id: FactId::new(id.as_str()),
                    value: to_fact_value(value),
                },
            );
        }
        Self {
            world,
            state,
            facts,
            last_shown: None,
            last_fired: Vec::new(),
            last_frame: None,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub(crate) fn eval(&self) -> Eval<'_> {
        Eval {
            world: &self.world,
            state: &self.state,
            facts: &self.facts,
        }
    }

    /// The intro, start rules and the first room description.
    pub fn start(&mut self) -> Turn {
        let mut turn = Turn::default();
        let intro = self.world.intro.clone();
        let runs = self.assemble(&intro, true);
        turn.push(BlockKind::Narration, runs);
        self.enter_room(&mut turn, 0);
        self.fire(&mut turn, |r| matches!(r.on, EventPattern::Start), 0);
        self.finish(turn)
    }

    /// What a command that is still being typed refers to, for the verb bar.
    pub fn preview(&self, text: &str) -> crate::Preview {
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

    /// Check fragment text that the author is typing, and render it in the current state.
    pub fn try_markup(&self, src: &str) -> MarkupCheck {
        match lantern_lang::compile_markup(src, &self.world.scope) {
            Ok(segments) => {
                let ev = self.eval();
                let mut pieces = Vec::new();
                text::outline(&ev, &segments, true, &mut pieces);
                MarkupCheck {
                    error: None,
                    text: text::render(&ev, &segments),
                    pieces,
                    holds: None,
                }
            }
            Err(e) => MarkupCheck::failed(e),
        }
    }

    /// All rooms with what is in them now, for the Atlas.
    pub fn atlas(&self) -> Vec<AtlasRoom> {
        let story = &self.world.story;
        let ev = self.eval();
        story
            .rooms
            .iter()
            .map(|r| AtlasRoom {
                id: r.id.to_string(),
                name: r.name.clone(),
                level: self.world.level_of(r.id.as_str()),
                visited: self.state.visits_to(r.id.as_str()) > 0,
                current: r.id == self.state.room,
                things: story
                    .things
                    .iter()
                    .filter(|t| self.state.is_in_room(t.id.as_str(), r.id.as_str()))
                    .map(|t| ThingInfo {
                        id: t.id.to_string(),
                        name: ev.thing_name(t.id.as_str(), lantern_lang::NameForm::Bare),
                    })
                    .collect(),
                exits: r
                    .exits
                    .iter()
                    .map(|e| {
                        let to = e.to.as_ref().map(|t| t.to_string()).unwrap_or_default();
                        (e.direction.name().to_string(), to)
                    })
                    .collect(),
                fragments: r.description.len(),
            })
            .collect()
    }

    /// Every name that conditions and markup can use, for completion and hover in the editor.
    pub fn symbols(&self) -> Vec<Symbol> {
        let scope = &self.world.scope;
        let story = &self.world.story;
        let mut out = Vec::new();
        let mut add = |name: String, kind: &str, detail: String, values: Vec<String>| {
            out.push(Symbol {
                name,
                kind: kind.to_string(),
                detail,
                value: None,
                values,
            });
        };
        let rooms: Vec<String> = scope.rooms.iter().cloned().collect();
        for (name, ty) in &scope.vars {
            let values = match ty {
                lantern_lang::Ty::Enum(v) => v.clone(),
                lantern_lang::Ty::Bool => vec!["true".into(), "false".into()],
                _ => Vec::new(),
            };
            add(
                name.clone(),
                "variable",
                format!("variable, {}", ty.describe()),
                values,
            );
        }
        for (thing, props) in &scope.things {
            for (prop, ty) in props {
                add(
                    format!("{thing}.{prop}"),
                    "property",
                    format!("property of {thing}, {}", ty.describe()),
                    Vec::new(),
                );
            }
            for (form, what) in [
                ("name", "the current name"),
                ("the", "the name with \"the\""),
                ("The", "the name with \"The\""),
                ("a", "the name with \"a\" or \"an\""),
                ("A", "the name with \"A\" or \"An\""),
            ] {
                add(
                    format!("{thing}.{form}"),
                    "name",
                    format!("{what} of {thing}, for text"),
                    Vec::new(),
                );
            }
            for (derived, what) in [
                ("carried", "true when the player carries it"),
                ("present", "true when it is carried or in the current room"),
                ("interactions", "how many commands named it"),
                ("talked", "how many times the player talked to it"),
            ] {
                add(
                    format!("{thing}.{derived}"),
                    "derived",
                    what.to_string(),
                    Vec::new(),
                );
            }
            let mut places = rooms.clone();
            places.extend(["carried".to_string(), "nowhere".to_string()]);
            add(
                format!("{thing}.location"),
                "derived",
                "a room id, carried or nowhere".into(),
                places,
            );
        }
        for room in &rooms {
            add(
                format!("{room}.visits"),
                "derived",
                "how many times the player entered it".into(),
                Vec::new(),
            );
            add(
                format!("{room}.visited"),
                "derived",
                "true after the first visit".into(),
                Vec::new(),
            );
        }
        add(
            "time.phase".into(),
            "derived",
            "the phase of the day".into(),
            scope.phases.clone(),
        );
        add(
            "time.hour".into(),
            "derived",
            "the hour, 0 to 23".into(),
            Vec::new(),
        );
        add(
            "time.day".into(),
            "derived",
            "the day, from 1".into(),
            Vec::new(),
        );
        add(
            "turns".into(),
            "derived",
            "the number of turns".into(),
            Vec::new(),
        );
        add(
            "here".into(),
            "derived",
            "the id of the current room".into(),
            rooms.clone(),
        );
        add(
            "first_visit".into(),
            "derived",
            "true on the first visit to the current room".into(),
            Vec::new(),
        );
        for (name, what) in [
            (
                "action.repeat",
                "times in a row the player gave this command",
            ),
            (
                "action.count",
                "times the player used this verb in this room",
            ),
            (
                "action.total",
                "times the player used this verb in the game",
            ),
            ("action.verb", "the id of the current verb"),
        ] {
            add(name.into(), "derived", what.into(), Vec::new());
        }
        for phase in &scope.phases {
            add(
                phase.clone(),
                "phase",
                format!("true when time.phase is {phase}"),
                Vec::new(),
            );
        }
        add(
            "first".into(),
            "markup",
            "in markup: {first: …} shows only on the first visit".into(),
            Vec::new(),
        );
        let _ = story;
        // The value now: render the name as an insert, or test it as a condition.
        for sym in &mut out {
            let insert = self.try_markup(&format!("{{{}}}", sym.name));
            sym.value = if insert.error.is_none() {
                Some(insert.text)
            } else {
                self.try_condition(&sym.name).holds.map(|b| b.to_string())
            };
        }
        out
    }

    /// Check a condition that the author is typing, and tell whether it holds now.
    pub fn try_condition(&self, src: &str) -> MarkupCheck {
        if src.trim().is_empty() {
            return MarkupCheck::default();
        }
        match lantern_lang::compile_condition(src, &self.world.scope) {
            Ok(c) => MarkupCheck {
                holds: Some(self.eval().holds(&c)),
                ..MarkupCheck::default()
            },
            Err(e) => MarkupCheck::failed(e),
        }
    }

    /// The room description in another combination of values, without changing the game.
    /// The names and values are the ones of [`crate::RoomMatrix`]. None for an unknown room.
    pub fn preview_situation(&self, room: &str, values: &[(String, String)]) -> Option<Turn> {
        let mut copy = self.clone();
        crate::matrix::apply_cell(&mut copy, room, values).then(|| copy.describe_preview())
    }

    /// Check fragment text and render it as it would read in other values, without changing the game.
    pub fn try_markup_situation(
        &self,
        room: &str,
        values: &[(String, String)],
        src: &str,
    ) -> MarkupCheck {
        let mut copy = self.clone();
        crate::matrix::apply_cell(&mut copy, room, values);
        copy.try_markup(src)
    }

    /// The room description as it would show now, without changing the game.
    pub fn describe_preview(&self) -> Turn {
        let mut copy = self.clone();
        let mut turn = Turn::default();
        copy.describe_room(&mut turn);
        turn
    }

    /// Handle a typed command.
    pub fn input(&mut self, text: &str) -> Turn {
        self.last_frame = None;
        let mut turn = Turn::default();
        turn.say(BlockKind::Command, text.trim());
        if self.state.ended {
            turn.say(BlockKind::Response, "The story has ended.");
            return turn;
        }
        match parse(&self.world.verbs, &self.parse_scope(), text) {
            Ok(cmd) => self.perform(&mut turn, cmd),
            Err(msg) => turn.say(BlockKind::Response, msg),
        }
        self.finish(turn)
    }

    /// Handle a command picked on the verb bar.
    pub fn act(&mut self, cmd: Command) -> Turn {
        self.last_frame = None;
        let mut turn = Turn::default();
        turn.say(BlockKind::Command, self.describe_command(&cmd));
        if self.state.ended {
            turn.say(BlockKind::Response, "The story has ended.");
            return turn;
        }
        self.perform(&mut turn, cmd);
        self.finish(turn)
    }

    fn describe_command(&self, cmd: &Command) -> String {
        match cmd {
            Command::Go { direction } => format!("go {direction}"),
            Command::Verb { verb, objects } => {
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
            .thing_name(id.as_str(), lantern_lang::NameForm::Bare)
    }

    /// "the oil lamp", "The oil lamp", or "Blackbeard" for a proper name.
    fn the(&self, id: &Id, capital: bool) -> String {
        let form = if capital {
            lantern_lang::NameForm::TheCap
        } else {
            lantern_lang::NameForm::The
        };
        self.eval().thing_name(id.as_str(), form)
    }

    /// The nouns that name a thing now: those of its current name stage and of
    /// the stages before it, lowercased. A later name is not known yet.
    fn nouns_of(&self, t: &lantern_model::Thing) -> Vec<String> {
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

    fn perform(&mut self, turn: &mut Turn, cmd: Command) {
        match cmd {
            Command::Go { direction } => {
                let verb = Id::new("go");
                self.begin_action(&verb, &[], &format!("go {direction}"));
                let frame = self.frame_for(&verb, None);
                self.frame_text(turn, frame.as_ref().map(|f| &f.before));
                self.go(turn, &direction);
                self.frame_text(turn, frame.as_ref().map(|f| &f.after));
            }
            Command::Verb { verb, objects } => {
                if let Some(missing) = objects.iter().find(|o| !self.reachable(o.as_str())) {
                    turn.say(
                        BlockKind::Response,
                        format!("You can't see {} here.", self.the(missing, false)),
                    );
                    return;
                }
                let key = std::iter::once(verb.as_str())
                    .chain(objects.iter().map(Id::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.begin_action(&verb, &objects, &key);
                let frame = self.frame_for(&verb, objects.first());
                let on_verb = |rule: &lantern_model::Rule| match &rule.on {
                    EventPattern::Verb(parts) => {
                        parts.first() == Some(&verb)
                            && parts.len() - 1 <= objects.len()
                            && parts[1..].iter().zip(objects.iter()).all(|(a, b)| a == b)
                    }
                    _ => false,
                };
                self.frame_text(turn, frame.as_ref().map(|f| &f.before));
                // `instead` rules run before the default and replace it;
                // other rules run after it, so their text follows the response.
                let instead = self.fire(turn, |r| r.instead && on_verb(r), 0);
                if !instead {
                    // An `instead` frame replaces the default response, but not the rules.
                    let replaced = self.frame_text(turn, frame.as_ref().map(|f| &f.instead));
                    if !replaced {
                        self.default_action(turn, verb.as_str(), &objects);
                    }
                    self.fire(turn, |r| !r.instead && on_verb(r), 0);
                }
                self.frame_text(turn, frame.as_ref().map(|f| &f.after));
            }
        }
        self.state.action = None;
        self.advance(turn);
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
    fn frame_for(&mut self, verb: &Id, thing: Option<&Id>) -> Option<crate::world::CFrame> {
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
    fn frame_use(&self, verb: &Id, scope: &str, frame: &crate::world::CFrame) -> FrameUse {
        let ev = self.eval();
        let s = &self.state;
        let part = |name: &str, fragments: &[CFragment]| {
            let visible = text::visible(&ev, fragments, false);
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
                        } else if f.mode == lantern_model::FragmentMode::Once && seen > 0 {
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

    /// The frame used for the last action, if there was one.
    pub fn last_frame(&self) -> Option<&FrameUse> {
        self.last_frame.as_ref()
    }

    /// Show frame fragments as one response block. Returns true if any showed.
    fn frame_text(&mut self, turn: &mut Turn, fragments: Option<&Vec<CFragment>>) -> bool {
        let Some(fragments) = fragments.filter(|f| !f.is_empty()) else {
            return false;
        };
        let runs = self.assemble(fragments, false);
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

    fn room_def(&self) -> Option<&lantern_model::Room> {
        self.world.story.room(self.state.room.as_str())
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
        let room_id = self.state.room.as_str();
        !self.world.story.things.iter().any(|t| {
            self.state.flag(&format!("{}.lit", t.id))
                && (self.state.is_carried(t.id.as_str())
                    || self.state.is_in_room(t.id.as_str(), room_id))
        })
    }

    /// Things the player can see and refer to.
    pub(crate) fn in_scope(&self) -> Vec<Id> {
        let dark = self.is_dark();
        let room = self.state.room.as_str();
        let mut ids: Vec<Id> = self
            .world
            .story
            .things
            .iter()
            .filter(|t| {
                self.state.is_carried(t.id.as_str())
                    || (!dark && self.state.is_in_room(t.id.as_str(), room))
            })
            .map(|t| t.id.clone())
            .collect();
        for d in self.doors_here() {
            if !ids.contains(&d) {
                ids.push(d);
            }
        }
        ids
    }

    fn reachable(&self, id: &str) -> bool {
        self.in_scope().iter().any(|t| t.as_str() == id)
    }

    pub(crate) fn visible_exits(&self) -> Vec<(usize, &lantern_model::Exit)> {
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

    fn parse_scope(&self) -> ParseScope {
        ParseScope {
            things: self
                .in_scope()
                .iter()
                .filter_map(|id| self.world.story.thing(id.as_str()))
                .map(|t| (t.id.clone(), self.nouns_of(t)))
                .collect(),
            exits: self
                .visible_exits()
                .iter()
                .map(|(_, e)| {
                    let name =
                        e.to.as_ref()
                            .and_then(|to| self.world.story.room(to.as_str()))
                            .map(|r| r.name.clone())
                            .unwrap_or_default();
                    (e.direction.name().to_string(), name)
                })
                .collect(),
        }
    }

    pub fn scene(&self) -> Scene {
        let ev = self.eval();
        let info = |id: &Id| ThingInfo {
            id: id.to_string(),
            name: self.name_of(id),
        };
        let scope = self.in_scope();
        Scene {
            room: self.state.room.to_string(),
            room_name: self.room_def().map(|r| r.name.clone()).unwrap_or_default(),
            time: ev.time_of_day().to_string(),
            phase: ev.phase(),
            day: self.state.minutes / TimeOfDay::MINUTES_PER_DAY + 1,
            turns: self.state.turns,
            dark: self.is_dark(),
            things: scope
                .iter()
                .filter(|id| !self.state.is_carried(id.as_str()))
                .map(info)
                .collect(),
            inventory: scope
                .iter()
                .filter(|id| self.state.is_carried(id.as_str()))
                .map(info)
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
                            .story
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
    pub fn assembly(&self) -> Vec<FragmentView> {
        let Some(ri) = self.world.room_idx(self.state.room.as_str()) else {
            return Vec::new();
        };
        let room = &self.world.story.rooms[ri];
        let fragments = &self.world.rooms[ri].description;
        let ev = self.eval();
        let first = self.state.visits_to(room.id.as_str()) <= 1;
        let mut shown = text::visible(&ev, fragments, first);
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
                } else if f.mode == lantern_model::FragmentMode::FirstVisit && !first {
                    Some("first visit only".to_string())
                } else if f.mode == lantern_model::FragmentMode::Once && seen > 0 {
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

    /// All rooms and exits with the player's position and visits. Does not change any state.
    pub fn map(&self) -> MapView {
        let story = &self.world.story;
        let ev = self.eval();
        let mut next_free = 0.0_f32;
        let rooms = story
            .rooms
            .iter()
            .map(|r| {
                let [x, y] = r.position.unwrap_or_else(|| {
                    // Rooms without a position go on a row under the map.
                    next_free += 1.0;
                    [next_free - 1.0, 100.0]
                });
                MapRoom {
                    id: r.id.to_string(),
                    name: r.name.clone(),
                    x,
                    y,
                    level: self.world.level_of(r.id.as_str()),
                    visited: self.state.visits_to(r.id.as_str()) > 0,
                    current: r.id == self.state.room,
                }
            })
            .collect();
        let mut links: Vec<MapLink> = Vec::new();
        for (ri, room) in story.rooms.iter().enumerate() {
            for (ei, exit) in room.exits.iter().enumerate() {
                // A message exit is no edge of the map.
                let Some(to) = &exit.to else {
                    continue;
                };
                let back = story
                    .room(to.as_str())
                    .is_some_and(|t| t.exits.iter().any(|e| e.to.as_ref() == Some(&room.id)));
                if back
                    && links
                        .iter()
                        .any(|l| l.from == to.as_str() && l.to == room.id.as_str())
                {
                    continue;
                }
                links.push(MapLink {
                    from: room.id.to_string(),
                    to: to.to_string(),
                    direction: exit.direction.name().to_string(),
                    visible: ev.holds_opt(&self.world.rooms[ri].exits[ei].visible),
                    one_way: !back,
                });
            }
        }
        MapView { rooms, links }
    }

    /// The things and exits that make the objects and exits layers of the
    /// current room, with what shows and why. Does not change any state.
    pub fn room_parts(&self) -> RoomParts {
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
        let first = self.state.visits_to(room.id.as_str()) <= 1;

        // The room text as it reads now, to check which things it mentions.
        let fragments = &self.world.rooms[ri].description;
        let room_text = text::visible(&ev, fragments, first)
            .iter()
            .flat_map(|&i| fragments[i].variants.iter().map(|v| text::render(&ev, v)))
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let objects = story
            .things
            .iter()
            .enumerate()
            .filter(|(_, t)| self.state.is_in_room(t.id.as_str(), room.id.as_str()))
            .map(|(ti, t)| {
                let presence = &self.world.things[ti].presence;
                let generated = format!("There is {} here.", self.eval().thing_name(t.id.as_str(), lantern_lang::NameForm::A));
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
                } else if !t.presence.is_empty() && text::visible(&ev, presence, first).is_empty() {
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
    pub fn author(&mut self, cmd: AuthorCommand) -> Turn {
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
                let Some(r) = self.world.story.room(&room) else {
                    turn.say(BlockKind::Response, format!("There is no room `{room}`."));
                    return turn;
                };
                note(&mut turn, format!("go to {}", r.name));
                self.state.room = r.id.clone();
                *self
                    .state
                    .visits
                    .entry(self.state.room.clone())
                    .or_default() += 1;
                self.describe_room(&mut turn);
            }
            AuthorCommand::Set { name, value } => {
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
                let Some(t) = self.world.story.thing(&thing) else {
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

    /// Facts about a thing, for an author tooltip. Does not change any state.
    pub fn inspect_thing(&self, id: &str) -> Option<Inspect> {
        let t = self.world.story.thing(id)?;
        let mut facts: Vec<(String, String)> = vec![("id".into(), t.id.to_string())];
        let ev = self.eval();
        match ev.name_stage(id) {
            Some(current) => {
                let stages: Vec<String> = t
                    .names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| {
                        let when = n
                            .when
                            .as_deref()
                            .map(|w| format!(" when {w}"))
                            .unwrap_or_default();
                        let now = if i == current { " (now)" } else { "" };
                        format!("{}{when}{now}", n.text)
                    })
                    .collect();
                facts.push(("names".into(), stages.join(" → ")));
                facts.push(("player calls it".into(), self.nouns_of(t).join(", ")));
            }
            None if !t.aliases.is_empty() => {
                facts.push(("also called".into(), t.aliases.join(", ")))
            }
            None => {}
        }
        let location = match self.state.location_of(id) {
            Some(Location::Room(r)) => self
                .world
                .story
                .room(r.as_str())
                .map_or(r.to_string(), |room| format!("{} ({r})", room.name)),
            Some(Location::Carried) => "carried by the player".into(),
            _ => "nowhere".into(),
        };
        facts.push(("location".into(), location));
        let mut kind = vec![if t.portable { "portable" } else { "fixed" }];
        if !t.listed {
            kind.push("not listed");
        }
        facts.push(("kind".into(), kind.join(", ")));
        if let Some(key) = &t.key {
            facts.push(("key".into(), key.to_string()));
        }
        for prop in t.props.keys() {
            let name = format!("{id}.{prop}");
            if let Some(v) = self.state.facts.get(&name) {
                facts.push((prop.clone(), v.to_string()));
            }
        }
        let doors: Vec<String> = self
            .world
            .story
            .rooms
            .iter()
            .flat_map(|r| {
                r.exits
                    .iter()
                    .filter(|e| e.door.as_ref().is_some_and(|d| d.as_str() == id))
                    .map(move |e| format!("{} {}", r.id, e.direction.name()))
            })
            .collect();
        if !doors.is_empty() {
            facts.push(("door of".into(), doors.join(", ")));
        }
        let mentions = |text: &str| text.contains(id);
        let rules = self
            .world
            .story
            .rules
            .iter()
            .filter(|r| {
                matches!(&r.on, EventPattern::Verb(p) if p.iter().any(|x| x.as_str() == id))
                    || r.when.as_deref().is_some_and(mentions)
                    || r.then.iter().any(|e| mentions(&format!("{e:?}")))
            })
            .map(|r| r.id.to_string())
            .collect();
        Some(Inspect {
            kind: "thing".into(),
            id: id.to_string(),
            title: self.name_of(&t.id),
            facts,
            rules,
        })
    }

    /// Facts about an exit of the current room, for an author tooltip.
    pub fn inspect_exit(&self, direction: &str) -> Option<Inspect> {
        let ri = self.world.room_idx(self.state.room.as_str())?;
        let room = &self.world.story.rooms[ri];
        let (ei, exit) = room
            .exits
            .iter()
            .enumerate()
            .find(|(_, e)| e.direction.name() == direction)?;
        let ev = self.eval();
        let compiled = &self.world.rooms[ri].exits[ei];
        let to = match &exit.to {
            Some(to) => self
                .world
                .story
                .room(to.as_str())
                .map_or(to.to_string(), |r| format!("{} ({})", r.name, r.id)),
            None => "nowhere: a message exit".to_string(),
        };
        let mut facts = vec![("leads to".to_string(), to)];
        if let Some(door) = &exit.door {
            let state = if self.state.flag(&format!("{door}.locked")) {
                "locked"
            } else if self.state.flag(&format!("{door}.open")) {
                "open"
            } else {
                "closed"
            };
            facts.push(("door".into(), format!("{door}, {state}")));
        }
        if let Some(src) = &exit.visible {
            let now = if ev.holds_opt(&compiled.visible) {
                "true now"
            } else {
                "false now"
            };
            facts.push(("visible when".into(), format!("{src} ({now})")));
        }
        if let Some(src) = &exit.passable {
            let now = if ev.holds_opt(&compiled.passable) {
                "true now"
            } else {
                "false now"
            };
            facts.push(("passable when".into(), format!("{src} ({now})")));
        }
        if let Some(to) = &exit.to {
            let back = self
                .world
                .story
                .room(to.as_str())
                .is_some_and(|t| t.exits.iter().any(|e| e.to.as_ref() == Some(&room.id)));
            facts.push((
                "way back".into(),
                if back { "yes" } else { "no, one way" }.into(),
            ));
        }
        let target = exit.to.clone();
        let rules = self
            .world
            .story
            .rules
            .iter()
            .filter(|r| matches!(&r.on, EventPattern::Enter(x) | EventPattern::Leave(x) if Some(x) == target.as_ref()))
            .map(|r| r.id.to_string())
            .collect();
        Some(Inspect {
            kind: "exit".into(),
            id: direction.to_string(),
            title: format!("Exit {direction}"),
            facts,
            rules,
        })
    }

    /// All story variables and thing properties with their values now.
    pub fn vars(&self) -> Vec<VarView> {
        self.state
            .facts
            .iter()
            .map(|(name, value)| VarView {
                name: name.clone(),
                value: value.clone(),
                options: match self.world.story.var(name).map(|v| &v.ty) {
                    Some(lantern_model::VarType::Enum(values)) => values.clone(),
                    _ => Vec::new(),
                },
            })
            .collect()
    }

    /// The phases of the day, in order of their start time.
    pub fn phases(&self) -> Vec<PhaseView> {
        let mut phases: Vec<&lantern_model::Phase> = self.world.story.clock.phases.iter().collect();
        phases.sort_by_key(|p| p.from);
        phases
            .into_iter()
            .map(|p| PhaseView {
                name: p.name.clone(),
                from: p.from.to_string(),
            })
            .collect()
    }

    /// All rules with their state now. Does not change any state.
    pub fn rules(&self) -> Vec<RuleView> {
        let ev = self.eval();
        let story = &self.world.story;
        let thing_or_room = |id: &Id| {
            story
                .thing(id.as_str())
                .map(|t| t.name.clone())
                .or_else(|| story.room(id.as_str()).map(|r| r.name.clone()))
                .unwrap_or_else(|| id.to_string())
        };
        story
            .rules
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let when = match &r.on {
                    EventPattern::Start => "the game starts".to_string(),
                    EventPattern::Turn => "a turn passes".to_string(),
                    EventPattern::Enter(room) => {
                        format!("the player enters {}", thing_or_room(room))
                    }
                    EventPattern::Leave(room) => {
                        format!("the player leaves {}", thing_or_room(room))
                    }
                    EventPattern::Time(t) => format!("the clock reaches {t}"),
                    EventPattern::Verb(parts) => {
                        let verb = parts
                            .first()
                            .and_then(|v| self.world.verb(v.as_str()))
                            .map_or_else(
                                || parts.first().map(Id::to_string).unwrap_or_default(),
                                |v| v.label.to_lowercase(),
                            );
                        let objects: Vec<String> =
                            parts.iter().skip(1).map(thing_or_room).collect();
                        if objects.is_empty() {
                            format!("the player uses \"{verb}\"")
                        } else {
                            format!("the player uses \"{verb} {}\"", objects.join(" with "))
                        }
                    }
                };
                let then = r
                    .then
                    .iter()
                    .map(|e| match e {
                        lantern_model::Effect::Set(m) => format!(
                            "set {}",
                            m.iter()
                                .map(|(k, v)| format!("{k} = {v}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        lantern_model::Effect::Add(m) => format!(
                            "add {}",
                            m.iter()
                                .map(|(k, v)| format!("{v} to {k}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        lantern_model::Effect::Say(t) => format!("say \"{t}\""),
                        lantern_model::Effect::Move(m) => format!(
                            "move {}",
                            m.iter()
                                .map(|(t, to)| format!("{} to {to}", thing_or_room(t)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        lantern_model::Effect::Goto(room) => {
                            format!("move the player to {}", thing_or_room(room))
                        }
                        lantern_model::Effect::End(t) => format!("end the game: \"{t}\""),
                    })
                    .collect();
                let trigger = match &r.on {
                    EventPattern::Start => "start",
                    EventPattern::Turn => "turn",
                    EventPattern::Enter(_) => "enter",
                    EventPattern::Leave(_) => "leave",
                    EventPattern::Time(_) => "time",
                    EventPattern::Verb(_) => "verb",
                };
                let relevant = self.relevance(r);
                RuleView {
                    id: r.id.to_string(),
                    trigger: trigger.to_string(),
                    relevant,
                    when,
                    condition: r.when.clone(),
                    condition_holds: ev.holds_opt(&self.world.rules[i].when),
                    then,
                    once: r.once,
                    instead: r.instead,
                    fired: self.state.fired.contains(&r.id),
                }
            })
            .collect()
    }

    /// Why a rule matters at the player's position and time now, if it does.
    fn relevance(&self, rule: &lantern_model::Rule) -> Option<String> {
        if self.last_fired.contains(&rule.id) {
            return Some("fired in the last turn".into());
        }
        if rule.once && self.state.fired.contains(&rule.id) {
            return None;
        }
        let here = &self.state.room;
        match &rule.on {
            EventPattern::Enter(r) | EventPattern::Leave(r) if r == here => {
                Some("about this room".into())
            }
            EventPattern::Enter(r) => {
                let next_door = self
                    .room_def()
                    .is_some_and(|room| room.exits.iter().any(|e| e.to.as_ref() == Some(r)));
                next_door.then(|| "fires in a room next to this one".into())
            }
            EventPattern::Verb(parts) => {
                let scope = self.in_scope();
                let things = &parts[1..];
                if things.is_empty() {
                    Some("works with any thing".into())
                } else if things.iter().all(|t| scope.contains(t)) {
                    Some("uses things in reach".into())
                } else {
                    None
                }
            }
            EventPattern::Time(t) => {
                let now = self.state.minutes % TimeOfDay::MINUTES_PER_DAY;
                let at = u32::from(t.0);
                let ahead = (at + TimeOfDay::MINUTES_PER_DAY - now) % TimeOfDay::MINUTES_PER_DAY;
                (ahead <= 60).then(|| format!("fires in {ahead} minutes"))
            }
            EventPattern::Turn => Some("checks every turn".into()),
            EventPattern::Leave(_) | EventPattern::Start => None,
        }
    }

    pub fn verbs(&self) -> Vec<VerbInfo> {
        self.world
            .verbs
            .iter()
            .map(|v| VerbInfo {
                id: v.id.to_string(),
                label: v.label.clone(),
                slots: v.slots,
                preposition: v.preposition.clone(),
                bar: v.bar,
            })
            .collect()
    }

    // ---- text -----------------------------------------------------------

    fn assemble(&mut self, fragments: &[CFragment], first_visit: bool) -> Vec<Run> {
        self.assemble_with_picks(fragments, first_visit).0
    }

    fn assemble_with_picks(
        &mut self,
        fragments: &[CFragment],
        first_visit: bool,
    ) -> (Vec<Run>, Vec<usize>) {
        let visible = text::visible(&self.eval(), fragments, first_visit);
        let picks = text::pick(&mut self.state, fragments, &visible);
        (text::runs(&self.eval(), fragments, &picks), visible)
    }

    fn render(&self, segments: &[lantern_lang::Segment]) -> String {
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
            let first = self.state.visits_to(room.id.as_str()) <= 1;
            let (runs, shown) = self.assemble_with_picks(&world.rooms[ri].description, first);
            self.last_shown = Some((room.id.clone(), shown));
            turn.push(BlockKind::Prose, runs);
            let mut objects = Vec::new();
            for (ti, thing) in world.story.things.iter().enumerate() {
                if !thing.listed || !self.state.is_in_room(thing.id.as_str(), room.id.as_str()) {
                    continue;
                }
                let mut runs = self.assemble(&world.things[ti].presence, first);
                if runs.is_empty() {
                    let a = self
                        .eval()
                        .thing_name(thing.id.as_str(), lantern_lang::NameForm::A);
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
                });
            }
            runs.push(Run::plain("."));
            turn.push(BlockKind::Exits, runs);
        }
    }

    // ---- state changes --------------------------------------------------

    fn set_fact(&mut self, turn: &mut Turn, name: &str, value: Value) {
        turn.log.push(LogEntry {
            kind: "set".into(),
            text: format!("{name} = {value}"),
        });
        self.facts.set_fact(
            FactScope::World,
            Fact {
                id: FactId::new(name),
                value: to_fact_value(&value),
            },
        );
        self.state.facts.insert(name.to_string(), value);
    }

    fn move_thing(&mut self, id: &Id, to: Location) {
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
        self.fire(
            turn,
            |r| matches!(&r.on, EventPattern::Enter(id) if *id == room),
            depth,
        );
    }

    fn move_player(&mut self, turn: &mut Turn, to: Id, depth: u8) {
        let from = self.state.room.clone();
        self.fire(
            turn,
            |r| matches!(&r.on, EventPattern::Leave(id) if *id == from),
            depth,
        );
        self.state.room = to;
        self.enter_room(turn, depth);
    }

    /// Run every matching rule. Returns true if one of them was `instead`.
    fn fire(
        &mut self,
        turn: &mut Turn,
        matches: impl Fn(&lantern_model::Rule) -> bool,
        depth: u8,
    ) -> bool {
        if depth > MAX_RULE_DEPTH || self.state.ended {
            return false;
        }
        let world = self.world.clone();
        let mut instead = false;
        for (i, rule) in world.story.rules.iter().enumerate() {
            if !matches(rule) || (rule.once && self.state.fired.contains(&rule.id)) {
                continue;
            }
            if !self.eval().holds_opt(&world.rules[i].when) {
                continue;
            }
            turn.log.push(LogEntry {
                kind: "rule".into(),
                text: rule.id.to_string(),
            });
            self.state.fired.insert(rule.id.clone());
            instead |= rule.instead;
            for effect in &world.rules[i].then {
                self.apply(turn, &rule.id, effect, depth + 1);
            }
        }
        instead
    }

    fn apply(&mut self, turn: &mut Turn, rule: &Id, effect: &CEffect, depth: u8) {
        match effect {
            CEffect::Set(values) => {
                for (k, v) in values {
                    self.set_fact(turn, k, v.clone());
                }
            }
            CEffect::Add(values) => {
                for (k, n) in values {
                    let cur = self.state.facts.get(k).and_then(Value::as_int).unwrap_or(0);
                    self.set_fact(turn, k, Value::Int(cur + n));
                }
            }
            CEffect::Say(segments) => {
                let text = self.render(segments);
                turn.push(BlockKind::Narration, vec![rule_run(text, rule)]);
            }
            CEffect::Move(moves) => {
                for (thing, to) in moves {
                    let loc = match to.as_str() {
                        "carried" => Location::Carried,
                        "nowhere" => Location::Nowhere,
                        _ => Location::Room(to.clone()),
                    };
                    turn.log.push(LogEntry {
                        kind: "move".into(),
                        text: format!("{thing} -> {to}"),
                    });
                    self.move_thing(thing, loc);
                }
            }
            CEffect::Goto(room) => self.move_player(turn, room.clone(), depth),
            CEffect::End(segments) => {
                let text = self.render(segments);
                turn.push(BlockKind::End, vec![rule_run(text, rule)]);
                self.state.ended = true;
            }
        }
    }

    /// One turn passes: the clock moves, turn and time rules fire.
    fn advance(&mut self, turn: &mut Turn) {
        if self.state.ended {
            return;
        }
        let before = self.state.minutes;
        self.state.minutes += u32::from(self.world.story.clock.minutes_per_turn);
        self.state.turns += 1;
        let after = self.state.minutes;
        self.fire(turn, |r| matches!(r.on, EventPattern::Turn), 0);
        self.fire(
            turn,
            |r| match r.on {
                EventPattern::Time(t) => crossed(before, after, t),
                _ => false,
            },
            0,
        );
    }

    fn finish(&mut self, mut turn: Turn) -> Turn {
        self.last_fired = turn
            .log
            .iter()
            .filter(|l| l.kind == "rule")
            .map(|l| Id::new(l.text.as_str()))
            .collect();
        turn.ended = self.state.ended;
        self.link_things(&mut turn.blocks);
        turn
    }

    /// Mark mentions of things in scope as clickable.
    fn link_things(&self, blocks: &mut [Block]) {
        let mut needles: Vec<(String, String)> = self
            .in_scope()
            .iter()
            .filter_map(|id| self.world.story.thing(id.as_str()))
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
            if !self.state.flag(&format!("{door}.open")) {
                if self.state.flag(&format!("{door}.locked")) {
                    turn.say(BlockKind::Response, format!("{the_cap} is locked."));
                    return;
                }
                self.set_fact(turn, &format!("{door}.open"), Value::Bool(true));
                turn.say(BlockKind::Note, format!("(first opening {the})"));
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
        let thing = first
            .and_then(|o| self.world.story.thing(o.as_str()))
            .cloned();
        let prop = |p: &str| first.map(|o| format!("{o}.{p}")).unwrap_or_default();
        let has = |g: &Game, p: &str| g.state.facts.contains_key(&prop(p));
        match verb {
            "look" => self.describe_room(turn),
            "inventory" => {
                let items: Vec<String> = self
                    .in_scope()
                    .iter()
                    .filter(|id| self.state.is_carried(id.as_str()))
                    .map(|id| {
                        let n = self.name_of(id);
                        format!("{} {n}", article(&n))
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
            "examine" => {
                let Some(o) = first else { return };
                let Some(ti) = self.world.thing_idx(o.as_str()) else {
                    return;
                };
                let world = self.world.clone();
                let runs = self.assemble(&world.things[ti].description, true);
                if runs.is_empty() {
                    say(turn, format!("You see nothing special about {the}."));
                } else {
                    turn.push(BlockKind::Prose, runs);
                }
            }
            "take" => {
                let Some(t) = thing else { return };
                if self.state.is_carried(t.id.as_str()) {
                    say(turn, format!("You already have {the}."));
                } else if !t.portable {
                    say(turn, format!("{the_cap} won't budge."));
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
                if !has(self, "open") {
                    say(turn, format!("{the_cap} isn't something you can {verb}."));
                } else if opening && self.state.flag(&prop("locked")) {
                    say(turn, format!("{the_cap} is locked."));
                } else if self.state.flag(&prop("open")) == opening {
                    let word = if opening { "open" } else { "closed" };
                    say(turn, format!("{the_cap} is already {word}."));
                } else {
                    self.set_fact(turn, &prop("open"), Value::Bool(opening));
                    let word = if opening { "open" } else { "close" };
                    say(turn, format!("You {word} {the}."));
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
                        .story
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
            "talk" => say(turn, format!("{the_cap} has nothing to say.")),
            "give" => say(turn, "Nobody seems interested.".into()),
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

fn rule_run(text: String, rule: &Id) -> Run {
    Run {
        text,
        source: Some(crate::Source {
            owner: format!("rule:{rule}"),
            fragment: 0,
            layer: lantern_model::Layer::Event,
            condition: None,
            variant: 0,
        }),
        link: None,
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
fn crossed(before: u32, after: u32, t: TimeOfDay) -> bool {
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
        source: run.source.clone(),
        link,
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
