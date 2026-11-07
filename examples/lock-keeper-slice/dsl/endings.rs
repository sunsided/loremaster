// Part "Endings": the three endings as entities with a kind, a rank and a
// title. The YAML form loads since P1-03 (#44). This form compiles since the
// world! core (DSL step L3, P3-03 #66). `kind death` sends the death prompt
// (SE-9).
//
// Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    // draft: the vars of the today pack, used in the ending text.
    extern {
        var pages_burned;
    }

    // draft: an ending block. `ending <id> "<title>" { kind; rank; text }`.
    // A death has no rank, as in endings/drowned.yaml.
    ending committed "Committed for trial" {
        kind win;
        rank 1;
        "At dawn the county takes the lock-keeper to town. The pages go with him, in your coat.";
    }

    // Rank 2 is kept for the weak case of the full demo ("Charged, case weak").
    ending ashes "Released at dawn" {
        kind loss;
        rank 3;
        "Without the pages, the case does not hold. At dawn the lock-keeper opens the gates for the first barge, as if nothing happened.{pages_burned ? The ash in the stove was fresh. | }";
    }

    ending drowned "Drowned" {
        kind death;
        "The lock keeps what falls into it.";
    }
}
