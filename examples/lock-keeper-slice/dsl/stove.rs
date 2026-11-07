// Part "Stove": the stove with the text of the D2 side note (one_of,
// pick best, seen_by and learn). Waits for the text core (phase 4, R9) and
// the world! core (DSL step L3). The D2 sketch puts this group into the
// kitchen description; here it is the description of the stove, so that the
// kitchen stays in the today pack.
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        room kitchen;
        var ash_is_fresh;
    }

    thing stove "stove" in kitchen {
        aliases ["iron stove", grate, ash];
        unlisted;
        prop hot = false;
        // draft: `examine one_of <id> best { ... }`. The D2 sketch has
        // `objects one_of ...` in a room, with a layer word in front.
        examine one_of stove_state best {
            "The stove is cold.";
            when stove.hot => "The stove is still too hot to touch.";
            when stove.hot and seen_by(player)
                => "Someone fed the stove after midnight. The ash on the grate is fresh."
                learn ash_is_fresh;
        }
    }
}
