//! Actors and their agendas (D6, WO-5, WO-6).
//!
//! An actor is a thing that acts on its own: the actor of an agenda. Its
//! place is its location, as for any thing. Once per turn, after the clock
//! moved, each actor looks at the current step of its agenda (the first one
//! that is not over):
//!
//! - Not due yet: nothing happens.
//! - The player is in a talk with the actor: the step waits and runs late.
//! - Due, with a time, and its `when` does not hold: it is skipped, and the
//!   actor looks at the next step in the same turn.
//! - Due, without a time, and its `when` does not hold: it waits.
//! - The actor is not at the step's `place`: it walks one room towards it.
//! - Else the step runs: its `do` command, then its `then` effects.
//!
//! An actor runs at most one step or walks one room per turn. A step skipped
//! for its `when` takes no time; a step whose `do` a `check` rule refuses
//! takes the turn, because the actor tried.

use super::Game;
use crate::state::StepOutcome;
use crate::world::CompiledAgenda;
use crate::{BlockKind, CommandRecord, Event, LogEntry, Phase, Turn};
use loremaster_model::{Id, Location};

/// What the current step of an actor does in this turn.
enum Move {
    /// Nothing more this turn.
    Rest,
    /// The step is over without taking the turn: look at the next one.
    Next,
}

impl Game {
    /// Let each actor do its part of the turn.
    pub(super) fn run_agendas(&mut self, turn: &mut Turn) {
        let world = self.world.clone();
        for agenda in &world.agendas {
            // A skipped step leaves the turn to the next one; the bound keeps
            // the loop finite.
            for _ in 0..=agenda.steps.len() {
                if self.state.ended {
                    return;
                }
                match self.agenda_turn(turn, agenda) {
                    Move::Rest => break,
                    Move::Next => continue,
                }
            }
        }
    }

    fn agenda_turn(&mut self, turn: &mut Turn, agenda: &CompiledAgenda) -> Move {
        let actor = agenda.actor.as_str();
        let done = self.state.actors.get(actor);
        let Some(step) = agenda
            .steps
            .iter()
            .find(|s| done.is_none_or(|d| !d.steps.contains_key(&s.key)))
        else {
            return Move::Rest;
        };
        if self.state.minutes < step.due {
            return Move::Rest;
        }
        let name = format!("{}#{}", agenda.id, step.key);
        let log = |turn: &mut Turn, text: String| {
            turn.log.push(LogEntry {
                kind: "agenda".into(),
                text,
            });
        };
        if self.talks_with(actor) {
            log(turn, format!("{name} waits for the talk"));
            return Move::Rest;
        }
        if !self.eval().holds_opt(&step.when) {
            if !step.timed {
                return Move::Rest;
            }
            log(turn, format!("{name} is skipped"));
            self.end_step(actor, &step.key, StepOutcome::Skipped);
            return Move::Next;
        }
        if let Some(place) = &step.place
            && self.actor_room(actor).as_ref() != Some(place)
        {
            self.walk(turn, &agenda.actor, place);
            log(turn, format!("{name} walks towards {place}"));
            return Move::Rest;
        }
        let owner = Id::new(format!("agenda:{name}"));
        if let Some(cmd) = &step.action {
            let record = CommandRecord {
                actor: agenda.actor.clone(),
                verb: cmd.verb.clone(),
                direct: cmd.direct.clone(),
                indirect: cmd.indirect.clone(),
                preposition: cmd.preposition.clone(),
                topic: cmd.topic.clone(),
            };
            if !self.actor_command(turn, &record) {
                log(turn, format!("{name} is refused"));
                self.end_step(actor, &step.key, StepOutcome::Skipped);
                // The actor tried: the refusal takes its turn.
                return Move::Rest;
            }
        }
        log(turn, format!("{name} runs"));
        // A `stop` ends the step's effects, as in a timer.
        let outer = std::mem::replace(&mut self.stopped, false);
        for effect in &step.then {
            // Also after `do`: a rule of the command can end the story.
            if self.stopped || self.state.ended {
                break;
            }
            self.apply(turn, &owner, effect, 1);
        }
        self.stopped = outer;
        self.end_step(actor, &step.key, StepOutcome::Ran);
        Move::Rest
    }

    /// For each agenda, the minutes until its current step is due (0 when it
    /// is due or the agenda is over). The explorer keeps them in its state
    /// fingerprint, so that waiting reaches a step later in the same phase.
    pub(crate) fn agenda_waits(&self) -> Vec<u32> {
        self.world
            .agendas
            .iter()
            .map(|a| {
                let done = self.state.actors.get(a.actor.as_str());
                a.steps
                    .iter()
                    .find(|s| done.is_none_or(|d| !d.steps.contains_key(&s.key)))
                    .map_or(0, |s| s.due.saturating_sub(self.state.minutes))
            })
            .collect()
    }

    fn end_step(&mut self, actor: &str, key: &str, outcome: StepOutcome) {
        self.state
            .actors
            .entry(Id::new(actor))
            .or_default()
            .steps
            .insert(key.to_string(), outcome);
    }

    /// The player is in a talk with the thing.
    fn talks_with(&self, thing: &str) -> bool {
        self.state.talk.as_ref().is_some_and(|t| {
            self.world
                .dialogue(t.dialogue.as_str())
                .is_some_and(|d| d.with.as_str() == thing)
        })
    }

    /// The room where an actor is: the player's room, or the room around a
    /// thing. None for a thing that is nowhere or carried.
    pub(super) fn actor_room(&self, actor: &str) -> Option<Id> {
        if actor == crate::PLAYER {
            return Some(self.state.room.clone());
        }
        match self.state.outer_location(actor)? {
            Location::Room(r) => Some(r.clone()),
            _ => None,
        }
    }

    /// One room towards `place`, along the exits. An actor without a way
    /// there (or without a room) goes there at once. The player sees an actor
    /// leave and come in.
    fn walk(&mut self, turn: &mut Turn, actor: &Id, place: &Id) {
        let from = self.actor_room(actor.as_str());
        let (direction, to) = from
            .as_ref()
            .and_then(|f| self.world.route(f.as_str(), place.as_str()))
            .map_or((None, place.clone()), |(d, r)| (Some(d), r));
        let here = self.state.room.clone();
        let sees = !self.is_dark();
        turn.log.push(LogEntry {
            kind: "move".into(),
            text: format!("{actor} -> {to}"),
        });
        turn.moves.push(crate::MoveDetail {
            actor: actor.clone(),
            name: self.name_of(actor),
            from: from.clone(),
            to: to.clone(),
        });
        if sees && from.as_ref() == Some(&here) {
            let the = self.the(actor, true);
            let text = match direction {
                Some(d) => format!("{the} goes {d}."),
                None => format!("{the} leaves."),
            };
            turn.say(BlockKind::Narration, text);
        }
        self.move_thing(actor, Location::Room(to.clone()));
        if to == here && !self.is_dark() {
            let the = self.the(actor, true);
            turn.say(BlockKind::Narration, format!("{the} comes in."));
        }
    }

    /// Run a command of an actor that is not the player: its `check` rules,
    /// then its `instead` or `verb` rules. The default actions of the
    /// standard verbs are the player's and do not run. False when a `check`
    /// rule refused it.
    pub(super) fn actor_command(&mut self, turn: &mut Turn, c: &CommandRecord) -> bool {
        turn.log.push(LogEntry {
            kind: "command".into(),
            text: format!(
                "{} {}",
                c.actor,
                std::iter::once(c.verb.as_str())
                    .chain(c.direct.as_ref().map(Id::as_str))
                    .chain(c.indirect.as_ref().map(Id::as_str))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        });
        let waiting = self.state.awaiting.is_some();
        if self
            .fire(turn, &Event::of_command(Phase::Check, c), 1)
            .stopped
        {
            return false;
        }
        // A check rule that waits holds the action, as for the player. An
        // await that was open before says nothing about this command.
        if !waiting && self.state.awaiting.is_some() {
            return true;
        }
        let instead = self.fire(turn, &Event::of_command(Phase::Instead, c), 1);
        if !instead.instead && !instead.stopped {
            self.fire(turn, &Event::of_command(Phase::After, c), 1);
        }
        true
    }
}
