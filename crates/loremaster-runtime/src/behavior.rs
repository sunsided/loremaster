//! Behaviors: game logic in Rust, for what rules, timers and machines cannot say.
//!
//! A behavior listens to the same events as rules and runs after the rules
//! of each event. It reads and changes the world only through [`Cx`], so every
//! change is in [`State`](crate::State): save games, the state explorer and
//! walkthroughs keep working. A behavior has no fields that change during play.
//!
//! [`Cx`] is a narrow, stable view (XP-1): facts, locations, the player's
//! room, neighbours, carried things, tags, the turn count, and the effect
//! calls. Random numbers come only from [`Cx::roll`] (XP-7), never from a
//! clock or the host. [`Cx::unstable`] is the escape hatch to the whole state
//! and world for Rust code; it can change between versions.

use crate::{BlockKind, Event, EventKind, Game, State, Turn, World};
use loremaster_model::{Id, Location, Value};

/// What a behavior wants after it ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flow {
    /// Go on: later rules, behaviors and phases run.
    #[default]
    Continue,
    /// Like the `stop` effect: in a `check` phase, this refuses the action.
    Stop,
}

/// Game logic in Rust. Register it with [`World::register`] and name its id
/// under `behaviors:` in story.yaml.
pub trait Behavior: Send + Sync + std::fmt::Debug {
    /// The id that story.yaml names.
    fn id(&self) -> &str;

    /// One line for the rules view: what the behavior does.
    fn description(&self) -> &str {
        ""
    }

    /// What the behavior reads, writes and handles (XP-4). The default is
    /// opaque: it may read and write anything and handles every event.
    fn manifest(&self) -> Manifest {
        Manifest::opaque()
    }

    /// React to an event. The session calls it only for the event kinds that
    /// the manifest names.
    fn handle(&self, event: &Event, cx: &mut Cx<'_>) -> Flow;
}

/// What a behavior reads, what it writes and which events it handles (XP-4).
///
/// Names are facts (`beacon_lit`, `lamp.lit`) or the place of a thing
/// (`thief.location`). `None` means unknown: the behavior is opaque there.
/// The session sends a behavior only the events of its kinds, and the
/// explorer and the checks use the lists for code they cannot look into.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Manifest {
    pub reads: Option<Vec<String>>,
    pub writes: Option<Vec<String>>,
    pub events: Option<Vec<EventKind>>,
}

impl Manifest {
    /// Unknown reads, writes and events.
    pub fn opaque() -> Self {
        Self::default()
    }

    /// A behavior that reads and writes nothing and handles no event, to
    /// build on with [`Manifest::reads`], [`Manifest::writes`] and
    /// [`Manifest::events`].
    pub fn new() -> Self {
        Self {
            reads: Some(Vec::new()),
            writes: Some(Vec::new()),
            events: Some(Vec::new()),
        }
    }

    pub fn reads<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.reads
            .get_or_insert_with(Vec::new)
            .extend(names.into_iter().map(Into::into));
        self
    }

    pub fn writes<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.writes
            .get_or_insert_with(Vec::new)
            .extend(names.into_iter().map(Into::into));
        self
    }

    pub fn events(mut self, kinds: impl IntoIterator<Item = EventKind>) -> Self {
        self.events.get_or_insert_with(Vec::new).extend(kinds);
        self
    }

    /// Does the behavior handle events of this kind?
    pub fn handles(&self, kind: EventKind) -> bool {
        self.events.as_ref().is_none_or(|k| k.contains(&kind))
    }

    /// May the behavior write this name?
    pub fn may_write(&self, name: &str) -> bool {
        self.writes
            .as_ref()
            .is_none_or(|w| w.iter().any(|n| n == name))
    }

    /// One line for the rules view.
    pub fn describe(&self) -> String {
        let list = |l: &Option<Vec<String>>| match l {
            None => "anything".to_string(),
            Some(l) if l.is_empty() => "nothing".to_string(),
            Some(l) => l.join(", "),
        };
        let events = match &self.events {
            None => "every event".to_string(),
            Some(k) if k.is_empty() => "no event".to_string(),
            Some(k) => k
                .iter()
                .map(|k| format!("{k:?}").to_lowercase())
                .collect::<Vec<_>>()
                .join(", "),
        };
        format!(
            "on {events}; reads {}; writes {}",
            list(&self.reads),
            list(&self.writes)
        )
    }
}

/// What a behavior can read and change.
pub struct Cx<'a> {
    pub(crate) game: &'a mut Game,
    pub(crate) turn: &'a mut Turn,
    pub(crate) owner: Id,
    pub(crate) depth: u8,
    pub(crate) manifest: &'a Manifest,
}

/// The whole state and world, read-only. The type follows semver like the
/// rest of the crate, but `State` and `World` change with the model, so code
/// that reads them changes more often than code that uses only `Cx`. A
/// script host must not hand it to scripts.
pub struct Unstable<'a> {
    game: &'a Game,
}

impl<'a> Unstable<'a> {
    pub fn state(&self) -> &'a State {
        self.game.state()
    }

    pub fn world(&self) -> &'a World {
        self.game.world()
    }
}

impl Cx<'_> {
    /// The escape hatch for Rust code: the whole state and world, read-only.
    /// Prefer the other methods of `Cx`, which stay when the model changes.
    pub fn unstable(&self) -> Unstable<'_> {
        Unstable { game: self.game }
    }

    /// The turns played so far.
    pub fn turns(&self) -> u32 {
        self.game.state().turns
    }

    /// The minutes on the story clock since midnight of the first day.
    pub fn minutes(&self) -> u32 {
        self.game.state().minutes
    }

    /// The player has been in the room.
    pub fn visited(&self, room: &str) -> bool {
        self.game.state().visits_to(room) > 0
    }

    /// The room has the tag, for example `dark`. An unknown room has no tags,
    /// so a misspelled id gives false; `loremaster check` does not see ids in
    /// Rust code.
    pub fn room_has_tag(&self, room: &str, tag: &str) -> bool {
        self.game
            .world()
            .story
            .room(room)
            .is_some_and(|r| r.has_tag(tag))
    }

    /// The room of the player.
    pub fn here(&self) -> &Id {
        &self.game.state().room
    }

    pub fn location(&self, thing: &str) -> Option<&Location> {
        self.game.state().location_of(thing)
    }

    /// The thing is carried, in the player's room, or scenery of it.
    pub fn present(&self, thing: &str) -> bool {
        self.game.eval().present(thing)
    }

    pub fn fact(&self, name: &str) -> Option<&Value> {
        self.game.state().facts.get(name)
    }

    /// Set a variable or property.
    pub fn set(&mut self, name: &str, value: Value) {
        self.check_write(name);
        self.game.set_fact(self.turn, name, value);
    }

    /// A write that the manifest does not name is a note for the author.
    fn check_write(&mut self, name: &str) {
        if !self.manifest.may_write(name) {
            let text = format!(
                "{} writes `{name}`, which its manifest does not name",
                self.owner
            );
            self.turn.log.push(crate::LogEntry {
                kind: "author".into(),
                text,
            });
        }
    }

    /// Print a line, like the `say` effect.
    pub fn say(&mut self, text: impl Into<String>) {
        let run = crate::game::rule_run(text.into(), &self.owner);
        self.turn.push(BlockKind::Narration, vec![run]);
    }

    /// Move a thing. Rules do not fire.
    pub fn move_thing(&mut self, thing: &Id, to: Location) {
        self.check_write(&format!("{thing}.location"));
        self.turn.log.push(crate::LogEntry {
            kind: "move".into(),
            text: format!("{thing} -> {}", to.to_word()),
        });
        self.game.move_thing(thing, to);
    }

    /// Move the player. Leave and enter rules fire.
    pub fn goto(&mut self, room: Id) {
        self.game.move_player(self.turn, room, self.depth + 1);
    }

    /// Send a custom event, like the `emit` effect.
    pub fn emit(&mut self, name: &str) {
        self.game
            .fire(self.turn, &Event::Custom(Id::new(name)), self.depth + 1);
    }

    /// A number from 0 to `n - 1`, from the random numbers in the save state,
    /// so play replays.
    pub fn roll(&mut self, n: u32) -> u32 {
        crate::rng::below(&mut self.game.state_mut().rng, n as usize) as u32
    }

    /// The rooms that the exits of a room lead to, without their conditions.
    pub fn neighbours(&self, room: &str) -> Vec<Id> {
        self.game
            .world()
            .story
            .room(room)
            .map(|r| r.exits.iter().filter_map(|e| e.to.clone()).collect())
            .unwrap_or_default()
    }

    /// The things that the player holds, also in carried containers.
    pub fn carried(&self) -> Vec<Id> {
        let s = self.game.state();
        s.locations
            .keys()
            .filter(|id| s.is_carried(id.as_str()))
            .cloned()
            .collect()
    }
}
