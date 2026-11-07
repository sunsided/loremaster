// Part "Keeper rule": the keeper comments when the player examines the
// stove. Waits for the Command record in the event (phase 4, D6), the text
// pool in `say` (phase 4, R9) and the world! core (DSL step L3).
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        thing keeper;
        thing stove;
        texts keeper_remark;
    }

    // draft: `direct: stove` names a slot of the Command record, as YAML
    // `on: { verb: examine, direct: stove }`. The D2 sketch writes the old
    // form `on verb(examine, stove)`.
    rule keeper_comments on verb(examine, direct: stove) when present(keeper) {
        say keeper_remark;
    }
}
