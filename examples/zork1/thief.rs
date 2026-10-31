//! The thief of Zork I as a loremaster behavior: hand-ported in part from
//! I-THIEF and ROBBER-FUNCTION (1actions.zil).
//!
//! The thief walks through the dark rooms of the underground. In the room of
//! the player, he can take a treasure that the player carries and put it in
//! his bag. All of his state is the location of the thief and of the things
//! in his bag, so a save game and the state explorer see it.

use loremaster_model::{Id, Location};
use loremaster_runtime::{Behavior, Cx, Event, Flow};

#[derive(Debug)]
pub struct Thief {
    /// The thing that is the thief.
    pub thief: Id,
    /// Where the thief starts.
    pub home: Id,
    /// The things that the thief steals.
    pub treasures: Vec<Id>,
}

impl Thief {
    /// The thief of the imported Zork I.
    pub fn zork() -> Self {
        let treasures = [
            "diamond",
            "emerald",
            "bauble",
            "bracelet",
            "bag_of_coins",
            "bar",
            "pot_of_gold",
            "scarab",
            "trunk",
            "chalice",
            "painting",
            "egg",
            "torch",
            "sceptre",
            "skull",
            "jade",
            "coffin",
            "trident",
            "canary",
        ];
        Thief {
            thief: Id::new("thief"),
            home: Id::new("treasure_room"),
            treasures: treasures.iter().map(|t| Id::new(*t)).collect(),
        }
    }

    /// A dark room next to `from`, picked at random. The thief stays underground.
    fn next_room(&self, cx: &mut Cx<'_>, from: &str) -> Option<Id> {
        let dark: Vec<Id> = cx
            .neighbours(from)
            .into_iter()
            .filter(|r| {
                cx.world()
                    .story
                    .room(r.as_str())
                    .is_some_and(|room| room.has_tag("dark"))
            })
            .collect();
        if dark.is_empty() {
            return None;
        }
        let i = cx.roll(dark.len() as u32) as usize;
        Some(dark[i].clone())
    }
}

impl Behavior for Thief {
    fn id(&self) -> &str {
        "thief"
    }

    fn description(&self) -> &str {
        "the thief walks the dark rooms and steals treasures that the player carries"
    }

    fn handle(&self, event: &Event, cx: &mut Cx<'_>) -> Flow {
        match event {
            Event::Start => {
                cx.move_thing(&self.thief, Location::Room(self.home.clone()));
            }
            Event::Turn => {
                let Some(Location::Room(at)) = cx.location(self.thief.as_str()).cloned() else {
                    // Dead, or not in play.
                    return Flow::Continue;
                };
                let here = cx.here().clone();
                if at == here {
                    // The thief steals only when he can get away; a room without a dark exit keeps him.
                    let Some(to) = self.next_room(cx, at.as_str()) else {
                        return Flow::Continue;
                    };
                    let loot: Vec<Id> = cx
                        .carried()
                        .into_iter()
                        .filter(|t| self.treasures.contains(t))
                        .collect();
                    if !loot.is_empty() && cx.roll(100) < 40 {
                        let i = cx.roll(loot.len() as u32) as usize;
                        cx.move_thing(&loot[i], Location::In(self.thief.clone()));
                        cx.say(
                            "The thief just left, still carrying his large bag. You may not have \
                             noticed that he robbed you blind first.",
                        );
                    } else if cx.roll(100) < 30 {
                        cx.say("The holder of the large bag just left, looking disgusted.");
                    } else {
                        return Flow::Continue;
                    }
                    cx.move_thing(&self.thief, Location::Room(to));
                } else if cx.roll(100) < 50
                    && let Some(to) = self.next_room(cx, at.as_str())
                {
                    cx.move_thing(&self.thief, Location::Room(to.clone()));
                    if to == here {
                        cx.say(
                            "Someone carrying a large bag is casually leaning against one of the \
                             walls here. He does not speak, but it is clear from his aspect that \
                             the bag will be taken only over his dead body.",
                        );
                    }
                }
            }
            _ => {}
        }
        Flow::Continue
    }
}
