// Speech segments wait for #111.
use loremaster::prelude::*;

world! {
    rule accuse on verb(accuse, keeper) {
        line "The lock-keeper shrugs." keeper says "Prove it," narrate "he says.";
    }
}

fn main() {}
