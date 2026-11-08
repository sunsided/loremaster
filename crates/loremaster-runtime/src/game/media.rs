//! The active media slots (ME-1, ME-2). They come from the world state each
//! time, so they are never saved.

use super::Game;
use crate::world::CompiledMedia;
use crate::{MediaView, SceneRegion};
use loremaster_model::{Region, Value};

impl Game {
    pub(crate) fn media(&self) -> MediaView {
        let world = &self.world;
        let state = &self.state;
        // The strongest first: a machine state, the partner in a talk or a
        // fight, then the room.
        let mut layers: Vec<&CompiledMedia> = (world.machine_media.iter())
            .filter_map(
                |(machine, states)| match state.facts.get(machine.as_str()) {
                    Some(Value::Text(s)) => states.get(s),
                    _ => None,
                },
            )
            .collect();
        let talking = (state.talk.as_ref())
            .and_then(|t| world.dialogue(t.dialogue.as_str()))
            .map(|d| d.with.clone());
        // The enemy the player fights now: the first one still standing.
        let fighting =
            || (state.combat.as_ref()).and_then(|c| self.living(&c.spec).into_iter().next());
        if let Some(i) = talking
            .or_else(fighting)
            .and_then(|t| world.thing_idx(t.as_str()))
        {
            layers.push(&world.things[i].media);
        }
        if let Some(i) = world.room_idx(state.room.as_str()) {
            layers.push(&world.rooms[i].media);
        }
        let eval = self.eval();
        let pick = |slot: usize| {
            layers.iter().find_map(|m| {
                let (_, cue, regions) = m.slots[slot].iter().find(|(w, ..)| eval.holds_opt(w))?;
                Some((cue.clone(), regions))
            })
        };
        let illustration = pick(0);
        let regions = illustration.as_ref().map_or_else(Vec::new, |(_, r)| {
            r.iter().filter_map(|r| self.region(r)).collect()
        });
        let cue = |slot| pick(slot).map(|(c, _)| c);
        MediaView {
            illustration: illustration.map(|(c, _)| c),
            portrait: cue(1),
            music: cue(2),
            ambience: cue(3),
            regions,
        }
    }

    /// The hotspot as the host sees it, while the player sees its thing on
    /// the ground or its exit.
    fn region(&self, region: &Region) -> Option<SceneRegion> {
        let name = if let Some(thing) = &region.thing {
            let id = thing.as_str();
            // The scope also has the doors of the room, in the dark and
            // behind hidden exits: the player does not see those.
            let in_room = matches!(
                self.state.outer_location(id),
                Some(loremaster_model::Location::Room(r)) if *r == self.state.room
            );
            let visible_door =
                (self.visible_exits().iter()).any(|(_, e)| e.door.as_ref() == Some(thing));
            if !self.reachable(id)
                || self.state.is_carried(id)
                || self.is_dark()
                || (!in_room && !visible_door && self.doors_here().contains(thing))
            {
                return None;
            }
            self.name_of(thing)
        } else {
            let dir = region.exit.as_ref()?.name();
            let exits = self.visible_exits();
            let (_, exit) = exits.iter().find(|(_, e)| e.direction.name() == dir)?;
            (exit.to.as_ref())
                .and_then(|to| self.world.room(to.as_str()))
                .map_or_else(|| dir.to_string(), |r| r.name.clone())
        };
        Some(SceneRegion {
            thing: region.thing.as_ref().map(ToString::to_string),
            exit: region.exit.as_ref().map(|d| d.name().to_string()),
            name,
            rect: region.rect,
        })
    }
}
