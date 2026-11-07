// An error in a machine step points at the step's tokens, not at the
// machine.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }
    room hall "Hall" {}
    thing lever "lever" in hall { prop lit = false; }

    machine gate starts closed {
        state closed;
        state open;
        closed -> open on verb(pull, lever) when lever.lt {
            line "Click.";
            start missing_timer;
        }
        open -> closed on verb(push, crank);
    }
}

fn main() {}
