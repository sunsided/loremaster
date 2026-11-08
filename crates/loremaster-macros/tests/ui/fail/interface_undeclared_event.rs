use loremaster::prelude::*;

world! {
    story "Host" { start: hall }
    room hall "Hall" { "A hall."; }
    rule hear on event(kiled) { line "Dead."; }

    interface {
        event killed(boss: text);
    }
}

fn main() {}
