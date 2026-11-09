// Part "Dialogue": the keeper's dialogue. Its first node says the text pool
// keeper_remark, so the file waits for the text pool (phase 4, R9). The rest
// is in world! since DSL step L4 (P4-08 #85): crates/loremaster-cli/tests/
// slice_dsl.rs compiles this dialogue with an inline line in place of the
// pool and compares it with dialogues/keeper.yaml.
//
// Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    // draft: `extern` with the kind texts (R9).
    extern {
        thing keeper;
        var ash_is_fresh;
        texts keeper_remark;
    }

    // A dialogue is dramaturge's token form with the lines of world!. Without
    // an id, it has the id of its thing (`keeper`, as dialogues/keeper.yaml).
    // A node is `<key>: <line> { <options> }`; an option is
    // `<line> [when <cond>] => <target>;`. `says "<text>"` is a speech
    // segment (#111): the renderer adds the quotes. A node's speech is the
    // thing's, an option's speech is the player's.
    dialogue with keeper start [stove] {
        // draft: a node with an id in place of a line says a text pool, as
        // `line keeper_remark;` does in a rule (D2 side note, R9).
        stove: keeper_remark {
            says "Who fed it?" when ash_is_fresh => who_fed_it;
            says "What do you burn at night?" => what_burns;
            says "Good night." => end;
        }
        what_burns: says "Coal, when the company sends it. Driftwood, when it does not." {
            says "And paper?" => who_fed_it;
            says "Good night." => end;
        }
        // Screenplay form (spike, #112): `(…)` is a stage direction, and the
        // bare string of such a line is speech. It is
        // `"He turns the poker in his hands." says "Nobody. …"`.
        who_fed_it: (He turns the poker in his hands.) "Nobody. The stove was cold all night." {
            says "Good night." => end;
        }
    }
}
