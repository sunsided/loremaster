// The smallest pack that checks: a story header and its start room.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }
    room hall "Hall" { "An empty hall."; }
}

fn main() {
    let world = Loremaster::new().pack(pack()).build().unwrap();
    assert_eq!(world.story.title, "Tiny");
}
