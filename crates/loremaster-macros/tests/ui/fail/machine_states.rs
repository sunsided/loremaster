// A machine step to a state that the machine does not have.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }
    room hall "Hall" {}
    thing lever "lever" in hall {}

    machine gate starts closed {
        state closed;
        state open;
        closed -> ajar on verb(pull, lever);
    }
}

fn main() {}
