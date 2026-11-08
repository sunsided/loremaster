//! The active media slots (ME-1, ME-2). They come from the world state each
//! time, so they are never saved.

use super::Game;
use crate::world::CompiledMedia;
use crate::{MediaCue, MediaView};
use loremaster_model::Value;

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
        let pick = |slot: usize| -> Option<MediaCue> {
            layers.iter().find_map(|m| {
                let (_, cue) = m.slots[slot].iter().find(|(w, _)| eval.holds_opt(w))?;
                Some(cue.clone())
            })
        };
        MediaView {
            illustration: pick(0),
            portrait: pick(1),
            music: pick(2),
            ambience: pick(3),
        }
    }
}
