// The reference example of DSL step L6 (DS-11 to DS-15): a story in world!,
// and a writer's YAML override of one room in override/. The Rust part is
// read-only in the editor; override/rooms/shore.yaml is what the editor's
// "Override" writes.

use loremaster::prelude::*;

world! {
    story "The Cove" { start: cottage, seed: 1 }

    room shore "The Shore" {
        "Shingle crunches underfoot.";
        exit north blocked "The cliff is sheer.";
    }

    room cottage "The Cottage" {
        "A cold hearth.";
        exit south -> shore;
    }
}
