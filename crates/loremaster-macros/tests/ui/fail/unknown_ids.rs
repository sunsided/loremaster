// Unknown rooms, things, timers and endings: each error points at its tokens.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }

    room hall "Hall" {
        exit north -> garden;
    }

    thing lamp "lamp" in cellar {}

    rule light on verb(light, lantern) {
        start burning;
        end bright;
    }
}

fn main() {}
