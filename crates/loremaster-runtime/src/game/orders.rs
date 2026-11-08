//! Several player actors (WO-5, WO-14): the active actor, switching, and
//! orders that run one step per turn.
//!
//! The story lists its `playable` actors. `State.room` and the carried
//! things always belong to the active one. Switching saves them into the
//! old actor (its things go `in` it, as a closed container) and takes them
//! from the new one. So every default action, and the perceiver of a text
//! (TX-9), works on the active actor without knowing about actors.
//!
//! An order runs in the same way: each turn, the actor with an order becomes
//! active for one step, and the player's actor comes back after it. The
//! player sees what the actor perceives while it acts.

use super::{Game, parse};
use crate::{BlockKind, Input, Turn};
use loremaster_model::{Id, Location, Thing, Value};

/// The verb that switches the active actor. Only a story with `playable`
/// has it.
pub(crate) const SWITCH: &str = "switch";

impl Game {
    pub(super) fn is_playable(&self, actor: &str) -> bool {
        self.world
            .story
            .playable
            .iter()
            .any(|a| a.as_str() == actor)
    }

    /// The playable actors that are not in scope here.
    pub(super) fn remote_actors(&self) -> impl Iterator<Item = &Thing> {
        let scope = self.in_scope();
        (self.world.story.playable.iter())
            .filter(move |a| !scope.contains(a))
            .filter_map(|a| self.world.thing(a.as_str()))
    }

    /// The active actor's thing follows the player's room.
    pub(super) fn place_active(&mut self) {
        if let Some(a) = self.state.active.clone() {
            self.move_thing(&a, Location::Room(self.state.room.clone()));
        }
    }

    /// Make `to` the active actor. False when it is nowhere.
    fn steer(&mut self, to: &Id) -> bool {
        let Some(room) = self.actor_room(to.as_str()) else {
            return false;
        };
        if let Some(old) = self.state.active.take() {
            for thing in self.held() {
                self.move_thing(&thing, Location::In(old.clone()));
            }
            self.state
                .facts
                .insert(format!("{old}.active"), Value::Bool(false));
        }
        for thing in self
            .state
            .contents(to.as_str())
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            self.move_thing(&thing, Location::Carried);
        }
        self.state.room = room;
        self.state.previous = None;
        self.state.active = Some(to.clone());
        self.state
            .facts
            .insert(format!("{to}.active"), Value::Bool(true));
        true
    }

    fn held(&self) -> Vec<Id> {
        (self.state.locations.iter())
            .filter(|(_, at)| matches!(at, Location::Carried))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The command `switch to <actor>`. It takes no turn.
    pub(super) fn switch_actor(&mut self, turn: &mut Turn, to: &Id) {
        let name = self.the(to, false);
        if self.state.active.as_ref() == Some(to) {
            return turn.say(BlockKind::Response, format!("You are {name} already."));
        }
        if !self.steer(to) {
            return turn.say(BlockKind::Response, format!("You can't reach {name} now."));
        }
        // An order ends when the player takes over.
        self.state.orders.remove(to);
        turn.say(BlockKind::Response, format!("You are {name} now."));
        // A switch is no entry: the actor stood here already, so no `enter`
        // rule runs and no visit counts.
        self.describe_room(turn);
    }

    /// `<actor>, <command> then <command>`, for a playable actor.
    pub(super) fn order_in(&self, text: &str) -> Option<(Id, Vec<Input>)> {
        let (name, rest) = text.split_once(',')?;
        let name = name.trim().to_lowercase();
        let mut named = (self.world.story.playable.iter()).filter(|a| {
            self.world
                .thing(a.as_str())
                .is_some_and(|t| self.nouns_of(t).contains(&name))
        });
        // A noun that names two actors (`robot`) names none of them.
        let (Some(actor), None) = (named.next(), named.next()) else {
            return None;
        };
        let steps: Vec<_> = rest.split(" then ").map(str::trim).collect();
        // `bilge,` alone, or an empty step, is no order: the parser answers it.
        if steps.iter().any(|s| s.is_empty()) {
            return None;
        }
        let steps = steps.into_iter().map(|s| Input::Text(s.into())).collect();
        Some((actor.clone(), steps))
    }

    /// Give `actor` new orders; earlier ones it had are gone. False when it
    /// takes none, because the player steers it.
    pub(super) fn give_order(&mut self, turn: &mut Turn, actor: &Id, steps: Vec<Input>) -> bool {
        let name = self.the(actor, true);
        if self.state.active.as_ref() == Some(actor) {
            turn.say(BlockKind::Response, format!("{name} is you now."));
            return false;
        }
        turn.say(BlockKind::Response, format!("{name} takes the order."));
        self.state.orders.insert(actor.clone(), steps.into());
        true
    }

    /// Each actor with an order runs its next step.
    pub(super) fn run_orders(&mut self, turn: &mut Turn) {
        let actors: Vec<Id> = self.state.orders.keys().cloned().collect();
        for actor in actors {
            // A talk or a rule that waits holds the orders until it is over,
            // also when the step of an actor before opened it.
            if self.state.ended || self.state.talk.is_some() || self.state.awaiting.is_some() {
                return;
            }
            let Some(queue) = self.state.orders.get_mut(&actor) else {
                continue;
            };
            let Some(step) = queue.pop_front() else {
                continue;
            };
            let last = queue.is_empty();
            if last {
                self.state.orders.remove(&actor);
            }
            self.order_step(turn, &actor, step, last);
        }
    }

    /// One step of an order, as `actor`. The last step reports.
    fn order_step(&mut self, turn: &mut Turn, actor: &Id, step: Input, last: bool) {
        let name = self.the(actor, true);
        let home = self.state.active.clone();
        // An actor's commands do not count as repeats of the player's, and
        // do not change the player's way back.
        let repeats = (self.state.last_command.clone(), self.state.repeat);
        let previous = self.state.previous.clone();
        if !self.steer(actor) {
            self.state.orders.remove(actor);
            return turn.say(BlockKind::Note, format!("{name} does not answer."));
        }
        turn.say(BlockKind::Note, format!("{name}:"));
        let command = match step {
            Input::Text(text) => parse(&self.world.verbs, &self.parse_scope(), &text),
            Input::Command(c) => Ok(c),
            // shortcut: only `give_order` fills the queue, with these two.
            _ => Err("That is no order.".to_string()),
        };
        // shortcut: a `talk` order opens the talk for the player. Upgrade when
        // an order can be a conversation.
        let failed = match command {
            Ok(c) => {
                self.action(turn, c);
                false
            }
            Err(message) => {
                turn.say(BlockKind::Response, message);
                true
            }
        };
        if let Some(home) = &home {
            self.steer(home);
        }
        (self.state.last_command, self.state.repeat) = repeats;
        self.state.previous = previous;
        if failed {
            self.state.orders.remove(actor);
            turn.say(
                BlockKind::Note,
                format!("{name} reports: the order stops here."),
            );
        } else if last {
            turn.say(BlockKind::Note, format!("{name} reports: done."));
        }
    }
}
