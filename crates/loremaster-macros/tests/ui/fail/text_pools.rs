// one_of groups and text pools wait for the text core (R9).
use loremaster::prelude::*;

world! {
    thing stove "stove" in kitchen {
        examine one_of stove_state best {
            "The stove is cold.";
        }
    }
}

fn main() {}
