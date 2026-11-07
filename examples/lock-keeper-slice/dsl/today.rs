// Part "Today": the story header, the vars, the three rooms, the things (not
// the stove), the lock machine and its timer, the lock and accuse rules, and
// the verbs. The YAML form of this part loads now. This form waits for the
// world! core (DSL step L3) and for machines, timers and verbs (step L4).
// `end committed` names an ending entity (phase 1, P1-03).
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    // draft: the ending ids live in the endings pack. Today's YAML gives
    // `end:` a text, so it loads without endings/. Here the ids must be
    // declared, and the check fails at load time without the endings pack.
    extern {
        ending committed;
        ending ashes;
        ending drowned;
    }

    story "The Lock-Keeper's Night (slice)" {
        author: "loremaster golden slice",
        start: kitchen,
        seed: 11,
        intro: "The county sent you to look at the toll books. The lock-keeper did not expect you before morning.",
        clock { start: "22:30", minutes_per_turn: 5,
                phases { night "00:00", dawn "03:30", day "07:00", night "21:00" } }
    }

    var ash_is_fresh: bool = false;
    var pages_burned: bool = false;

    // Rooms

    room kitchen "Kitchen" {
        at [0, 0];
        "A low room under the lock house, with a black iron stove against the wall.";
        ambient cycle when night => [
            "Water knocks against the lock gate.",
            "Somewhere a rope creaks.",
        ];
        exit up -> keepers_room;
        exit east -> lock_chamber;
    }

    room keepers_room "Keeper's Room" {
        at [0, -1];
        "A narrow room under the roof. A cot, a chair, and an iron hook by the door.";
        exit down -> kitchen;
    }

    room lock_chamber "Lock Chamber" {
        at [1, 0];
        "Stone walls drop into the lock. {lock == full ? Black water fills the chamber to the brim. | }{lock == emptying ? The water falls fast and the walls shine wet. | }{lock == empty ? The chamber is empty. Wet ladder rungs go down to the gear housing. | }";
        exit west -> kitchen;
    }

    // Things

    thing keeper "lock-keeper" in kitchen {
        aliases [keeper, man];
        presence "The lock-keeper sits by the stove and does not look up.";
        examine "A broad man with tar on his hands. He watches the stove more than he watches you.";
    }

    thing windlass "windlass" in keepers_room {
        aliases [handle, crank];
        portable;
        examine "An iron handle that turns the lock paddles. The square socket is worn bright.";
        presence "A windlass hangs on the hook by the door.";
    }

    thing lower_paddles "lower paddles" in lock_chamber {
        aliases [paddles, paddle, sluice];
        // draft: `unlisted` is YAML `listed: false`.
        unlisted;
        examine "Two iron paddles in the lower gate. A windlass turns them.";
    }

    thing gear_housing "gear housing" in lock_chamber {
        aliases [housing, gears, box];
        unlisted;
        // draft: `container` is YAML `container: true`.
        container;
        prop open = false;
        examine "An iron box at the bottom of the chamber, bolted to the stone. {lock == empty ? You can reach it now. | It is under water.}";
    }

    // Singular name: loremaster has no plural articles yet ("a ledger pages").
    // draft: `inside` is YAML `location: { in: gear_housing }`. `in` takes a
    // room only, so that the macro does not have to guess the kind of an id.
    thing pages "oilcloth packet" inside gear_housing {
        // draft: an alias with a space is a string literal.
        aliases [packet, pages, "ledger pages", oilcloth];
        portable;
        examine "Four pages torn from a toll ledger, dry inside the oilcloth. The sums on them do not match the sums in the book.";
    }

    // The lock machine. The chamber empties when the player opens the lower
    // paddles with the windlass. The pages are in the gear housing at the
    // bottom.

    machine lock starts full {
        state full "The chamber is full. The gear housing is under water.";
        state emptying "The lower paddles are open. The chamber is empty 3 turns after they open, the opening turn included (timer lock_drain).";
        state empty "The chamber is empty. The player can open the gear housing.";

        full -> emptying on verb(open, lower_paddles) instead when windlass.carried {
            say "You fit the windlass and turn. Water roars out under the lower gate.";
            start lock_drain;
        }
        emptying -> empty on timer(lock_drain) {
            say "{here == lock_chamber ? The chamber is empty. Wet ladder rungs go down into the dark. | }";
        }
    }

    // draft: a timer line. YAML `timers/lock_drain.yaml` with `turns: 3`.
    timer lock_drain turns 3;

    // Lock rules

    // draft: `on check(verb, thing)` is YAML `on: { check: [verb, thing] }`,
    // and `stop` is the stop effect.
    rule paddles_need_windlass on check(open, lower_paddles) when not windlass.carried {
        say "The paddles turn only with a windlass.";
        stop;
    }

    rule housing_under_water on check(open, gear_housing) when lock != empty {
        say "The gear housing is under black water.";
        stop;
    }

    // draft: `end <ident>` names an ending entity (P1-03). `end "<text>"` in
    // the DSL doc is today's form, with a text.
    rule jump_into_the_lock on verb(jump) when here == lock_chamber and lock == full {
        say "The water closes over you, colder than anything.";
        end drowned;
    }

    // Accuse rules

    rule accuse_with_pages on verb(accuse, keeper) when pages.carried {
        say "You lay the pages on the table. The lock-keeper looks at them for a long time.";
        end committed;
    }

    rule accuse_without_pages on verb(accuse, keeper) when not pages.carried {
        // draft: a speech segment with its speaker (#111).
        say "The lock-keeper shrugs." keeper says "Prove it," "he says.";
        end ashes;
    }

    // Verbs

    // draft: a verb block. The DSL doc shows only `verb examine costs 1;`.
    verb accuse "Accuse" {
        words [accuse, charge, arrest];
        default "You have no reason to accuse that.";
    }

    verb jump "Jump" {
        words [jump, "jump in", "jump into"];
        slots none;
        // draft: `no_bar` is YAML `bar: false`.
        no_bar;
        default "You jump on the spot. Nothing happens.";
    }
}
