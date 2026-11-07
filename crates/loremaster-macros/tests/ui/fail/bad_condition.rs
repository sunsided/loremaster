// A condition that does not type-check, and markup that does not parse.
use loremaster::prelude::*;

world! {
    story "Tiny" { start: hall }

    room hall "Hall" {
        "A hall. {lamp.lit ? It is bright. | It is dark.";
    }

    thing lamp "lamp" in hall { prop lit = false; }

    rule glow on turn when lamp.lt and not missing {
        line "It glows.";
    }
}

fn main() {}
