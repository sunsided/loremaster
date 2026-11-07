// Ids that conditions keep for themselves.
use loremaster::prelude::*;

world! {
    story "Tiny" {
        start: hall,
        clock { phases { night "00:00", day "07:00" } },
    }

    var turns: int = 0;
    var night: bool = false;

    room hall "Hall" {}
    room here "Here" {}
}

fn main() {}
