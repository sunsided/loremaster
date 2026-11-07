// Part "Agenda": the keeper's agenda. One step: at midnight the keeper feeds
// the stove with the pages, if the player has not taken them. Waits for
// actors and agendas (phase 4, D6), the winnable check (phase 4, CK) and the
// world! core (DSL step L3).
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        thing keeper;
        thing pages;
        thing stove;
        room kitchen;
        var pages_burned;
    }

    // draft: an agenda block. `agenda for <actor>`; the id is the actor id,
    // as the file name gives it in agendas/keeper.yaml.
    agenda for keeper {
        // draft: `step at "<time>" place <room> when <cond>`. The time is a
        // string, because 00:00 is not a Rust token.
        step at "00:00" place kitchen when in(pages, gear_housing) {
            move pages -> nowhere;
            set stove.hot = true, pages_burned = true;
            say "{here == kitchen ? The lock-keeper comes in from the lock with something wrapped in oilcloth. He opens the stove and feeds it in. | You smell smoke from the kitchen.}";
        }
        // draft: `allow_unwinnable` after the step body. The winnable check
        // (phase 4, CK) reports this step: after it, the best ending cannot
        // be reached. The mark accepts the edge.
        allow_unwinnable "The keeper burns the pages at midnight. A late player gets the ashes ending.";
    }
}
