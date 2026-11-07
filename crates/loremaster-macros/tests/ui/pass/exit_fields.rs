// The words of exit fields are also legal ids: `door.locked` in a condition
// is a property, not the field `door`.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }
    room hall "Hall" {
        exit east -> garden visible not door.locked passable door.open door door blocked "Shut.";
    }
    room garden "Garden" {}
    thing door "door" in hall { prop locked = false; prop open = true; }
}

fn main() {
    let p = pack();
    let exit = &p.rooms[0].exits[0];
    assert_eq!(exit.visible.as_deref(), Some("not door.locked"));
    assert_eq!(exit.passable.as_deref(), Some("door.open"));
    assert_eq!(exit.door.as_ref().unwrap().as_str(), "door");
    assert_eq!(exit.blocked.as_deref(), Some("Shut."));
}
