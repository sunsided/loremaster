// Threads, frames, name stages, the score, `replace`, `pack` and extern
// properties: what the grammar rejects, and what the checks find.

mod prop_type {
    use loremaster::prelude::*;
    world! {
        extern { thing stove [hot: float]; }
    }
}

mod replace_rule {
    use loremaster::prelude::*;
    world! {
        replace rule r on turn { stop; }
    }
}

mod pack_late {
    use loremaster::prelude::*;
    world! {
        room hall "Hall" {}
        pack hall;
    }
}

mod ranks_without_score {
    use loremaster::prelude::*;
    world! {
        story "Tiny" { start: hall, ranks { 0 "Beginner" } }
    }
}

mod thread_status {
    use loremaster::prelude::*;
    world! {
        thread t "T" { status closed; }
    }
}

mod pack_keyword {
    use loremaster::prelude::*;
    world! {
        pack loop;
    }
}

mod duplicate_property {
    use loremaster::prelude::*;
    world! {
        extern { thing stove [hot: bool, hot: int]; }
    }
}

mod extern_relation_with_story {
    use loremaster::prelude::*;
    world! {
        extern { relation trust; }
        story "Tiny" { start: hall }
        room hall "Hall" {}
    }
}

mod checks {
    use loremaster::prelude::*;
    world! {
        extern { thing stove [hot: bool]; room kitchen; }
        story "Tiny" { start: kitchen, score: 10, score_var: points }
        thing poker "poker" in kitchen {
            name when stove.cold => "hot poker";
            before juggle "You lean closer.";
            instead take when stove.hot and poker.bent => "It is too hot.";
        }
    }
}

fn main() {}
