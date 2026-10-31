//! Behaviors: game logic in Rust, for what rules, timers and machines cannot say.
//!
//! A behavior listens to the same events as rules and runs after the rules
//! of each event. It changes the world only through [`Cx`], so every change is
//! in [`State`](crate::State): save games, the state explorer and walkthroughs
//! keep working. A behavior has no fields that change during play.

use crate::{BlockKind, Event, Game, State, Turn, World};
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

    /// React to an event.
    fn handle(&self, event: &Event, cx: &mut Cx<'_>) -> Flow;
}

/// What a behavior can read and change.
pub struct Cx<'a> {
    pub(crate) game: &'a mut Game,
    pub(crate) turn: &'a mut Turn,
    pub(crate) owner: Id,
    pub(crate) depth: u8,
}

impl Cx<'_> {
    pub fn state(&self) -> &State {
        self.game.state()
    }

    pub fn world(&self) -> &World {
        self.game.world()
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
        self.game.set_fact(self.turn, name, value);
    }

    /// Print a line, like the `say` effect.
    pub fn say(&mut self, text: impl Into<String>) {
        let run = crate::game::rule_run(text.into(), &self.owner);
        self.turn.push(BlockKind::Narration, vec![run]);
    }

    /// Move a thing. Rules do not fire.
    pub fn move_thing(&mut self, thing: &Id, to: Location) {
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
        self.world()
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
