//! Things: objects, doors, scenery and characters.

use crate::convert::{IntoValue, id, strings};
use crate::pack::{register, replaceable, with_notes};
use crate::{HasFrames, IntoFragment};
use loremaster_model::{Frames, Id, Location, NameStage, Thing};
use loremaster_project::Pack;
use std::collections::BTreeMap;

/// A thing of a pack, from [`BuildPack::thing`](crate::BuildPack::thing).
pub struct ThingBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> ThingBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, name: String) -> Self {
        register(pack, format!("thing:{id}"));
        pack.things.push(Thing {
            media: Default::default(),
            id,
            name,
            aliases: Vec::new(),
            head: None,
            location: Location::Nowhere,
            portable: false,
            key: None,
            container: false,
            supporter: false,
            capacity: None,
            also_in: Vec::new(),
            props: BTreeMap::new(),
            description: Vec::new(),
            presence: Vec::new(),
            listed: true,
            proper: false,
            red_herring: false,
            names: Vec::new(),
            frames: Frames::default(),
            rules: Vec::new(),
            notes: Vec::new(),
        });
        let i = pack.things.len() - 1;
        ThingBuilder { pack, i }
    }

    fn thing(&mut self) -> &mut Thing {
        &mut self.pack.things[self.i]
    }

    /// More nouns for the parser.
    pub fn aliases(mut self, aliases: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.thing().aliases.extend(strings(aliases));
        self
    }

    /// `head`: the main word of the name for hosts, such as "boots".
    pub fn head(mut self, head: impl Into<String>) -> Self {
        self.thing().head = Some(head.into());
        self
    }

    /// `location: { room }`.
    pub fn in_room(mut self, room: impl AsRef<str>) -> Self {
        self.thing().location = Location::Room(id(room));
        self
    }

    /// `location: { in }`: inside a container.
    pub fn in_thing(mut self, holder: impl AsRef<str>) -> Self {
        self.thing().location = Location::In(id(holder));
        self
    }

    /// `location: { on }`: on a supporter.
    pub fn on_thing(mut self, holder: impl AsRef<str>) -> Self {
        self.thing().location = Location::On(id(holder));
        self
    }

    /// `location: carried`: the player has it at the start.
    pub fn carried(mut self) -> Self {
        self.thing().location = Location::Carried;
        self
    }

    /// `location: nowhere`, the default: a rule moves it into the world.
    pub fn nowhere(mut self) -> Self {
        self.thing().location = Location::Nowhere;
        self
    }

    /// `portable: true`.
    pub fn portable(mut self) -> Self {
        self.thing().portable = true;
        self
    }

    /// `key:`, the thing that unlocks it.
    pub fn key(mut self, key: impl AsRef<str>) -> Self {
        self.thing().key = Some(id(key));
        self
    }

    /// `container: true`.
    pub fn container(mut self) -> Self {
        self.thing().container = true;
        self
    }

    /// `supporter: true`.
    pub fn supporter(mut self) -> Self {
        self.thing().supporter = true;
        self
    }

    /// `capacity:`.
    pub fn capacity(mut self, capacity: u32) -> Self {
        self.thing().capacity = Some(capacity);
        self
    }

    /// One more room or region of `also_in:`.
    pub fn also_in(mut self, place: impl AsRef<str>) -> Self {
        self.thing().also_in.push(id(place));
        self
    }

    /// A property with its initial value, for example `open` or `lit`.
    pub fn prop(mut self, name: impl Into<String>, value: impl IntoValue) -> Self {
        self.thing().props.insert(name.into(), value.into_value());
        self
    }

    /// One fragment of the description, for `examine`.
    pub fn text(mut self, f: impl IntoFragment) -> Self {
        self.thing().description.push(f.into_fragment());
        self
    }

    /// One fragment of `presence:`, the line in the room's object layer.
    pub fn presence(mut self, f: impl IntoFragment) -> Self {
        self.thing().presence.push(f.into_fragment());
        self
    }

    /// `listed: false`: the room text describes the thing itself.
    pub fn unlisted(mut self) -> Self {
        self.thing().listed = false;
        self
    }

    /// `proper: true`: a proper name, without an article.
    pub fn proper(mut self) -> Self {
        self.thing().proper = true;
        self
    }

    /// `red_herring: true`: no puzzle needs the thing, on purpose.
    pub fn red_herring(mut self) -> Self {
        self.thing().red_herring = true;
        self
    }

    /// One stage of `names:`.
    pub fn name_stage(mut self, stage: NameStage) -> Self {
        self.thing().names.push(stage);
        self
    }
}

impl HasFrames for ThingBuilder<'_> {
    fn frames_mut(&mut self) -> &mut Frames {
        &mut self.thing().frames
    }
}

replaceable!(ThingBuilder, things, "thing");
with_notes!(ThingBuilder, things);
