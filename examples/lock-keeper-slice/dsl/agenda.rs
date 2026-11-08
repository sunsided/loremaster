// Part "Agenda": the keeper's agenda. One step: at midnight the keeper feeds
// the stove with the pages, if the player has not taken them. Waits for
// actors and agendas (phase 4, D6), the winnable check (phase 4, CK) and the
// world! core (DSL step L3).
//
// The YAML form (../yaml/agendas/keeper.yaml) loads since P4-02 (#79);
// ADR 0004 and docs/writing-stories.md give the model. world! has no agenda
// syntax yet: this block is still a draft, and its keywords should give the
// same `Agenda` values (`actor`, `steps`, `at`, `place`, `when`, `do`,
// `then`, `closes`, `reason`).
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    extern {
        thing keeper;
        thing pages;
        thing gear_housing;
        ending committed;
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
            line "{in(kitchen) ? The lock-keeper comes in from the lock with something wrapped in oilcloth. He opens the stove and feeds it in. | You smell smoke from the kitchen.}";
        }
        // draft: `closes [<endings>] because "<reason>"` after the step body.
        // A point of no return, not a dead end: after this step the best
        // ending (committed) cannot be reached, but the ashes ending still
        // can. The ending check (phase 4, CK) reports such a rank drop as a
        // warning; the mark names the endings that the step closes on purpose.
        closes [committed] because "The keeper burns the pages at midnight. A late player gets the ashes ending.";
    }
}
