use loremaster::prelude::*;

world! {
    story "Host" { start: hall }
    room hall "Hall" { "A hall."; }
    rule hear on event(killed) { line "Dead."; }

    interface {
        event killed(boss: text, boss: int);
    }
}

fn main() {}
