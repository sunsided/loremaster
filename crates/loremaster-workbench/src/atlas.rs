//! The Atlas and the map: all rooms, what is in them now, and the exits.

use crate::ThingInfo;
use loremaster_lang::NameForm;
use loremaster_runtime::authoring::Inspector;
use serde::{Deserialize, Serialize};

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

/// The whole map, for the map panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapView {
    pub rooms: Vec<MapRoom>,
    pub links: Vec<MapLink>,
    /// The message exits.
    #[serde(default)]
    pub ends: Vec<MapEnd>,
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

/// An exit that leads nowhere and only prints a message, for its own mark on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapEnd {
    pub room: String,
    pub direction: String,
    /// False while the exit is hidden from the player.
    pub visible: bool,
}

/// All rooms with what is in them now, for the Atlas.
pub(crate) fn atlas(ins: Inspector<'_>) -> Vec<AtlasRoom> {
    let story = &ins.world().story;
    story
        .rooms
        .iter()
        .map(|r| AtlasRoom {
            id: r.id.to_string(),
            name: r.name.clone(),
            level: ins.world().level_of(r.id.as_str()),
            visited: ins.state().visits_to(r.id.as_str()) > 0,
            current: r.id == ins.state().room,
            things: story
                .things
                .iter()
                .filter(|t| ins.state().is_in_room(t.id.as_str(), r.id.as_str()))
                .map(|t| ThingInfo {
                    id: t.id.to_string(),
                    name: ins.thing_name(t.id.as_str(), NameForm::Bare),
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

/// All rooms and exits with the player's position and visits. Does not change any state.
pub(crate) fn map(ins: Inspector<'_>) -> MapView {
    let story = &ins.world().story;
    let mut next_free = 0.0_f32;
    // Rooms without a position go on a row under the placed rooms; with no placed room, on row 0.
    let free_row = story
        .rooms
        .iter()
        .filter_map(|r| r.position.map(|[_, y]| y))
        .reduce(f32::max)
        .map_or(0.0, |y| y + 1.0);
    let rooms = story
        .rooms
        .iter()
        .map(|r| {
            let [x, y] = r.position.unwrap_or_else(|| {
                next_free += 1.0;
                [next_free - 1.0, free_row]
            });
            MapRoom {
                id: r.id.to_string(),
                name: r.name.clone(),
                x,
                y,
                level: ins.world().level_of(r.id.as_str()),
                visited: ins.state().visits_to(r.id.as_str()) > 0,
                current: r.id == ins.state().room,
            }
        })
        .collect();
    let mut links: Vec<MapLink> = Vec::new();
    let mut ends: Vec<MapEnd> = Vec::new();
    for room in &story.rooms {
        for (ei, exit) in room.exits.iter().enumerate() {
            let visible = ins.exit_visible(room.id.as_str(), ei);
            // A message exit is no edge of the map: it gets its own mark.
            let Some(to) = &exit.to else {
                ends.push(MapEnd {
                    room: room.id.to_string(),
                    direction: exit.direction.name().to_string(),
                    visible,
                });
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
                visible,
                one_way: !back,
            });
        }
    }
    MapView { rooms, links, ends }
}
