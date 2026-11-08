// `extend` waits: an extension in Rust needs the fields as data (later work).
use loremaster::prelude::*;

world! {
    extend thing stove {
        prop hot = true;
    }
}

fn main() {}
