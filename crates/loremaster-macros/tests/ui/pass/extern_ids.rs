// A pack without a story header, with the ids of another pack in `extern`.
use loremaster::prelude::*;

world! {
    extern {
        room hall;
        thing chest;
        var opened;
        var visits: int;
        machine door [closed, open];
        timer ticking;
        ending done;
        verb kick;
    }

    thing coin "coin" inside chest { portable; }

    rule kick_chest on verb(kick, chest) when not opened and visits > 1 and door == open {
        set opened = true;
        move coin -> hall;
        start ticking;
        end done;
    }
}

fn main() {
    let p = pack();
    assert!(p.header.is_none());
    assert_eq!(p.rules.len(), 1);
}
