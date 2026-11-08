// The checks of a dialogue in world!, at the tokens they name.

mod structure {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove, nowhere] {
            stove: "He nods." {
                says "Who fed it?" => who_fed_it;
                says "Good night." => end;
            }
        }
    }
}

// More lines and `next` play; their speakers and targets are checked.
mod lines_and_next {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            stove {
                keeper: "He nods.";
                stranger: "Hm.";
                -> end;
            }
        }
    }
}

mod schema {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; var ash_is_fresh; }
        dialogue keeper_talk with keeper start [stove] {
            stove: "He nods." {
                when ash_is_cold;
                set ash_is_fresh = 3;
                // The first error of a condition or a relation hides the
                // next ones: unknown_relation and extern_property test them.
                says "Who fed it?" => end {
                    then goto cellar;
                }
                says "{bad" => end;
                stranger says "Hm." => end;
            }
        }
    }
}

mod unknown_relation {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; }
        dialogue with keeper start [stove] {
            stove: "He nods." {
                relation trust += 1;
                says "Good night." => end;
            }
        }
    }
}

mod extern_property {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper [calm: bool]; }
        dialogue with keeper start [stove] {
            stove: "He nods." {
                says "Who fed it?" when keeper.angry => end;
            }
        }
    }
}

// A node whose key starts with `then` keeps its own span.
mod then_node {
    use loremaster::prelude::*;
    world! {
        extern { thing keeper; var ash_is_fresh; }
        dialogue with keeper start [then_what] {
            then_what: "He nods." {
                set ash_is_fresh = 3;
                says "Good night." => end;
            }
        }
    }
}

mod unknown_thing {
    use loremaster::prelude::*;
    world! {
        dialogue with keeper start [stove] {
            stove: "He nods." {
                says "Good night." => end;
            }
        }
    }
}

fn main() {}
