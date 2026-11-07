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

fn main() {}
