// Named slots of the command record wait for D6.
use loremaster::prelude::*;

world! {
    rule keeper_comments on verb(examine, direct: stove) {
        line "Hm.";
    }
}

fn main() {}
