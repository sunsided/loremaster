// A speaker must be a thing of the story or `player`, and the condition of
// a segment must check.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }
    room hall "Hall" {}
    thing keeper "keeper" in hall {}

    rule greet on turn {
        line "He nods." ghost says "Boo." when keeper.awake then narrate "He yawns.";
    }
}

fn main() {}
