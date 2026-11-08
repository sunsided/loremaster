// The walkthroughs of the slice, as test blocks. They are the same as
// ../yaml/tests/, with the same names and the same pending marks. Test blocks
// compile since the world! core (DSL step L3, P3-03 #66). Each test passes
// after the phase in its pending mark.
//
// draft: in this slice the test blocks are data in their own pack. The DSL
// doc makes each test block a #[test] that assembles all packs. But each
// slice test loads only some parts, so slice.rs picks the parts and runs the
// walkthrough, as it does for YAML.
//
// Read ../PENDING.md.

use loremaster::prelude::*;

// A test sees rendered text, with the quotes that the renderer adds. Raw
// strings (`r#"…"#`, #111) keep those expectations free of escapes.
world! {
    extern {
        room kitchen;
        room lock_chamber;
        machine lock [full, emptying, empty];
        var ash_is_fresh;
        var pages_burned;
        thing stove;
        ending committed;
        ending ashes;
    }

    // Too late: the keeper burns the pages at midnight. The game stays
    // playable, but only the ashes ending is left (a point of no return that
    // the agenda step marks with `closes`).
    // draft: `test <id> "<name>"`. The id is the file name in yaml/tests/.
    // The DSL doc gives a test only a name.
    // draft: `pending phase <n>, needs "<ids>";` is YAML `pending:`.
    // draft: `sees [..]` and `not_sees` take a list; `expect` after a step
    // checks facts, `ended` and `room` after that step.
    // draft: `wait * 16` is 16 `wait` steps. It expands to 16 steps, so the
    // model is the same as in YAML.
    test slice_ashes "Too late for the pages" {
        pending phase 4, needs "P4-07 #84";
        "examine stove" sees "The stove is cold.";
        wait * 16;
        wait sees "He opens the stove and feeds it in." expect pages_burned == true;
        "examine stove" sees "The ash on the grate is fresh." expect ash_is_fresh == true;
        "accuse keeper" sees ["Released at dawn", "The ash in the stove was fresh."];
        expect ending ashes, ended;
    }

    // Passes today: rooms, things, the lock machine and its timer in today's
    // syntax.
    test slice_baseline "Empty the lock and take the pages" {
        up;
        "take windlass";
        down;
        e;
        "open gear housing" sees "under black water";
        "open lower paddles" sees "Water roars out" expect lock == emptying;
        wait;
        wait sees "The chamber is empty.";
        "open gear housing" sees "oilcloth packet";
        "take pages";
        expect lock == empty, ash_is_fresh == false, room == lock_chamber, not ended;
    }

    // Jumping into the full lock ends the game; only the rule's text shows.
    test slice_drowning "Jump into the full lock" {
        jump sees "Nothing happens." expect not ended;
        e;
        jump sees ["The water closes over you", "Drowned"] not_sees "Nothing happens."
            expect ended, ending drowned;
    }

    // The best path: take the pages before midnight and accuse the keeper.
    test slice_best "The pages before midnight" {
        pending phase 4, needs "P4-07 #84";
        "examine stove" sees "The stove is cold.";
        up;
        "take windlass";
        down;
        e;
        "open lower paddles";
        wait;
        wait sees "The chamber is empty.";
        "open gear housing";
        "take pages";
        w;
        "examine stove" sees r#""Do not touch the stove," the lock-keeper says."#;
        "accuse keeper" sees "Committed for trial";
        // draft: `not_sees` in `expect` is YAML `expect: { not_see: .. }`.
        expect ending committed, pages_burned == false,
            not_sees "You smell smoke", ended;
    }

    // Endings are entities: the game reports the ending by its id (P1-03).
    test slice_endings "Accuse without proof" {
        "accuse keeper" sees "Released at dawn" not_sees "ashes";
        expect ending ashes, ended;
    }

    // The keeper's agenda: at midnight he feeds the stove with the pages.
    test slice_keeper_agenda "The keeper at midnight" {
        wait * 17;
        wait sees "He opens the stove and feeds it in.";
        expect pages_burned == true, stove.hot == true, room == kitchen;
    }

    // One dialogue format: a dialogue node says a named text pool, and an
    // option shows only after the player learned a fact.
    test slice_keeper_dialogue "Ask the keeper about the stove" {
        pending phase 4, needs "R9 (text pools), P4-01 #78";
        "talk to keeper" sees r#""Do not touch the stove," the lock-keeper says."# not_sees "Who fed it?";
        "1" sees "Coal, when the company sends it.";
        "2" expect not ended;
        expect room == kitchen;
    }

    // The stove text: one_of with pick best, seen_by and learn (the D2
    // sketch).
    test slice_stove_text "The stove after midnight" {
        pending phase 4, needs "TX, D2, P4-01 #78";
        "examine stove" sees "The stove is cold." not_sees "too hot";
        wait * 18;
        "examine stove" sees "The ash on the grate is fresh."
            not_sees ["The stove is cold.", "too hot to touch"]
            expect ash_is_fresh == true;
        expect ash_is_fresh == true;
    }

    // The keeper is in a talk at midnight. His agenda step waits for the talk
    // to end, then runs late.
    // Midnight is after command 18 (22:30 + 18 × 5 minutes): the talk starts
    // then.
    test slice_talk_pauses_agenda "Talk through midnight" {
        pending phase 4, needs "R9 (text pools), P4-01 #78";
        wait * 17;
        "talk to keeper" sees r#""Do not touch the stove," the lock-keeper says."#;
        "1" sees "Coal, when the company sends it." not_sees "feeds it in";
        "1" sees "The stove was cold all night." not_sees "feeds it in";
        "1" sees "He opens the stove and feeds it in.";
        expect pages_burned == true, stove.hot == true;
    }

    // A death ending sends the death prompt, and undo goes back one input.
    test slice_undo_after_death "Undo after drowning" {
        e;
        jump sees ["The water closes over you", "Drowned"] expect ended;
        undo sees "Lock Chamber" expect not ended, room == lock_chamber;
        expect lock == full, not ended;
    }
}
