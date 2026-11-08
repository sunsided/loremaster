// The grammar of a dialogue in world!: dramaturge's token form with the
// lines of world!. Each error points at its tokens.

mod text_pool {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            // A node that says a text pool waits for R9.
            stove: keeper_remark {
                says "Good night." => end;
            }
        }
    }
}

mod no_target {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            stove: "He nods." {
                says "Good night.";
            }
        }
    }
}

mod bare_string_after_speech {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            stove: says "Nobody." "He turns the poker." {
                says "Good night." => end;
            }
        }
    }
}

mod unknown_effect {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            stove: "He nods." {
                then jump;
                says "Good night." => end;
            }
        }
    }
}

fn main() {}
