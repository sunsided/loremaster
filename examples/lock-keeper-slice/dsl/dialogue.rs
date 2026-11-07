// Part "Dialogue": the keeper's dialogue. Its first node says the text pool
// keeper_remark. Waits for one dialogue format (phase 3, D3), the text pool
// (phase 4, R9) and dialogue in world! (DSL step L4).
//
// Draft syntax. It does not compile. Read ../PENDING.md.

use loremaster::prelude::*;

world! {
    // draft: `extern` with the kinds var and texts. The DSL doc shows only
    // thing and room.
    extern {
        thing keeper;
        var ash_is_fresh;
        texts keeper_remark;
    }

    // draft: `says "<text>"` is a speech segment (#111). The renderer adds
    // the quotes, so the line has no escapes. A plain string is narration.
    // Segments in a row join with a space, in any order:
    // `"He turns the poker." says "Nobody."` or
    // `says "Nobody." narrate "He turns the poker."`. An option's speaker is
    // the player; a node's speaker is the thing of the dialogue.
    dialogue with keeper start [stove] {
        // draft: a node with an id in place of a string says a text pool,
        // as `line keeper_remark;` does in a rule (D2 side note).
        stove: keeper_remark {
            // draft: `when` after the option text, before `=>`.
            says "Who fed it?" when ash_is_fresh => who_fed_it;
            says "What do you burn at night?" => what_burns;
            says "Good night." => end;
        }
        what_burns: says "Coal, when the company sends it. Driftwood, when it does not." {
            says "And paper?" => who_fed_it;
            says "Good night." => end;
        }
        who_fed_it: "He turns the poker in his hands." says "Nobody. The stove was cold all night." {
            says "Good night." => end;
        }
    }
}
