//! Rooms and their exits.

use crate::convert::{IntoDirection, id};
use crate::pack::{register, replaceable, with_notes};
use crate::{HasFrames, IntoFragment};
use loremaster_model::{Exit, Frames, Id, Room};
use loremaster_project::Pack;

/// A room of a pack, from [`BuildPack::room`](crate::BuildPack::room).
///
/// [`exit`](Self::exit) adds an exit; [`door`](Self::door),
/// [`traversal`](Self::traversal), [`passable`](Self::passable),
/// [`blocked`](Self::blocked) and [`visible`](Self::visible) then set the
/// fields of the exit added last:
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("tower");
/// p.room("stairwell", "Lighthouse Stairwell")
///     .exit("west", "shore")
///     .door("lighthouse_door")
///     .exit(Direction::Up, "lantern_room")
///     .traversal("You climb, and climb, and climb.");
/// ```
pub struct RoomBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> RoomBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, name: String) -> Self {
        register(pack, format!("room:{id}"));
        pack.rooms.push(Room {
            id,
            name,
            region: None,
            tags: Vec::new(),
            description: Vec::new(),
            exits: Vec::new(),
            position: None,
            level: None,
            frames: Frames::default(),
            rules: Vec::new(),
            notes: Vec::new(),
        });
        let i = pack.rooms.len() - 1;
        RoomBuilder { pack, i }
    }

    fn room(&mut self) -> &mut Room {
        &mut self.pack.rooms[self.i]
    }

    /// One fragment of the description.
    pub fn text(mut self, f: impl IntoFragment) -> Self {
        self.room().description.push(f.into_fragment());
        self
    }

    /// A free tag, such as `dark` or `outdoors`.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.room().tags.push(tag.into());
        self
    }

    /// `region:`.
    pub fn region(mut self, region: impl AsRef<str>) -> Self {
        self.room().region = Some(id(region));
        self
    }

    /// `position:` on the map.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.room().position = Some([x, y]);
        self
    }

    /// `level:`, for towers and dungeons.
    pub fn level(mut self, level: i32) -> Self {
        self.room().level = Some(level);
        self
    }

    /// An exit to another room.
    pub fn exit(mut self, direction: impl IntoDirection, to: impl AsRef<str>) -> Self {
        self.room().exits.push(Exit {
            direction: direction.into_direction(),
            to: Some(id(to)),
            visible: None,
            door: None,
            passable: None,
            blocked: None,
            traversal: None,
        });
        self
    }

    /// A message exit: it leads nowhere and prints `text`.
    pub fn exit_blocked(mut self, direction: impl IntoDirection, text: impl Into<String>) -> Self {
        self.room().exits.push(Exit {
            direction: direction.into_direction(),
            to: None,
            visible: None,
            door: None,
            passable: None,
            blocked: Some(text.into()),
            traversal: None,
        });
        self
    }

    #[track_caller]
    fn last_exit(&mut self) -> &mut Exit {
        match self.room().exits.last_mut() {
            Some(exit) => exit,
            None => panic!("the room has no exit yet: call `exit` first"),
        }
    }

    /// `door:` of the last exit: it is passable only while the door is open.
    #[track_caller]
    pub fn door(mut self, door: impl AsRef<str>) -> Self {
        self.last_exit().door = Some(id(door));
        self
    }

    /// `traversal:` of the last exit: printed when the player goes through.
    #[track_caller]
    pub fn traversal(mut self, text: impl Into<String>) -> Self {
        self.last_exit().traversal = Some(text.into());
        self
    }

    /// `passable:` of the last exit: an extra condition for passing.
    #[track_caller]
    pub fn passable(mut self, condition: impl Into<String>) -> Self {
        self.last_exit().passable = Some(condition.into());
        self
    }

    /// `blocked:` of the last exit: the text when it is not passable.
    #[track_caller]
    pub fn blocked(mut self, text: impl Into<String>) -> Self {
        self.last_exit().blocked = Some(text.into());
        self
    }

    /// `visible:` of the last exit: it is listed and usable only while this holds.
    #[track_caller]
    pub fn visible(mut self, condition: impl Into<String>) -> Self {
        self.last_exit().visible = Some(condition.into());
        self
    }
}

impl HasFrames for RoomBuilder<'_> {
    fn frames_mut(&mut self) -> &mut Frames {
        &mut self.room().frames
    }
}

replaceable!(RoomBuilder, rooms, "room");
with_notes!(RoomBuilder, rooms);
