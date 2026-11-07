// Part "Texts": the named text pool keeper_remark. Rules and dialogue call it
// by id. Waits for the text core (phase 4, R9) and the world! core (DSL step
// L3). The syntax is the one of the D2 side note.
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        room kitchen;
        var ash_is_fresh;
    }

    texts keeper_remark best {
        "Mind your step.";
        when here == kitchen => "\"Do not touch the stove,\" the lock-keeper says.";
        sticky when here == kitchen and ash_is_fresh => [
            "\"I let it go out at midnight,\" he says. He does not look at you.",
            "\"Cold stove, cold supper,\" he says.",
        ];
    }
}
