// Speech segments (#111): the lint for a bare string after speech, and
// the mistakes of the grammar.

mod bare_after_says {
    use loremaster::prelude::*;
    world! {
        rule accuse on turn {
            line "The lock-keeper shrugs." says "Prove it," "he says.";
        }
    }
}

mod when_without_then {
    use loremaster::prelude::*;
    world! {
        rule r on turn { line when night "Dark."; }
    }
}

mod speech_in_a_description {
    use loremaster::prelude::*;
    world! {
        room hall "Hall" { "A hall." keeper says "Hello."; }
    }
}

mod text_pool {
    use loremaster::prelude::*;
    world! {
        rule r on turn { line keeper_remark; }
    }
}

mod plain_line_markup {
    use loremaster::prelude::*;
    world! {
        story "Tiny" { start: hall }
        room hall "Hall" {}
        thing lever "lever" in hall {}
        machine gate starts closed {
            state closed;
            state open;
            closed -> open on verb(pull, lever) {
                line "It opens. {bogus ? Yes. | No.}";
            }
        }
        rule r on turn { line "Tick. {bogus ? Yes. | No.}"; }
    }
}

mod speaker_without_string {
    use loremaster::prelude::*;
    world! {
        rule r on turn { line keeper says foo; }
    }
}

fn main() {}
