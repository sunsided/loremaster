// Part "Texts": the named text pool keeper_remark. Rules and dialogue call it
// by id. Waits for the text core (phase 4, R9) and the world! core (DSL step
// L3). The syntax is the one of the D2 side note.
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        room kitchen;
        thing keeper;
        var ash_is_fresh;
    }

    // draft: `says "<text>"` is a speech segment (#111); the renderer adds
    // the quotes. The speaker of a pool line is the keeper: the pool has no
    // dialogue, so the speech segment names it with `keeper says`.
    // `narrate "<text>"` is narration. A bare string is narration too, but
    // right after `says` it reads like more speech, so the slice uses
    // `narrate` there (and a lint would suggest it).
    texts keeper_remark best {
        "Mind your step.";
        when in(kitchen) => keeper says "Do not touch the stove," narrate "the lock-keeper says.";
        sticky when in(kitchen) and ash_is_fresh => [
            keeper says "I let it go out at midnight," narrate "he says. He does not look at you.",
            keeper says "Cold stove, cold supper," narrate "he says.",
        ];
    }
}
