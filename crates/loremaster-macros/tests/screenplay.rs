//! Spike (#112): the screenplay form of a dialogue line is the same
//! dialogue as `says` and narration.

mod screenplay {
    use loremaster::prelude::*;

    world! {
        pack screenplay;

        extern { thing keeper; var late; }

        dialogue with keeper start [ask] {
            ask {
                keeper: "Nobody." (turns the poker in his hands.);
                keeper: ("He doesn't look up.") "Go to bed." "Now.";
                keeper: "Late." when late then "It is late." (yawns);
                (nods) "Good night." => end;
                "Not yet." => ask;
            }
        }
    }
}

mod spelled_out {
    use loremaster::prelude::*;

    world! {
        pack spelled_out;

        extern { thing keeper; var late; }

        dialogue with keeper start [ask] {
            ask {
                keeper: says "Nobody." narrate "turns the poker in his hands.";
                keeper: "He doesn't look up." says "Go to bed." says "Now.";
                keeper: says "Late." when late then "It is late." narrate "yawns";
                "nods" says "Good night." => end;
                "Not yet." => ask;
            }
        }
    }
}

#[test]
fn the_screenplay_form_is_says_and_narration() {
    assert_eq!(
        screenplay::screenplay().dialogues,
        spelled_out::spelled_out().dialogues
    );
}
