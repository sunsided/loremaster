// Checks of the block before the schema: vars need a story header, an
// extern id is not defined in the block, times are times of day, and test
// ids are unique.
mod var_without_story {
    use loremaster::prelude::*;
    world! {
        var lit: bool = false;
    }
}

mod extern_and_defined {
    use loremaster::prelude::*;
    world! {
        extern { room hall; }
        story "Tiny" { start: hall }
        room hall "Hall" {}
    }
}

mod bad_time {
    use loremaster::prelude::*;
    world! {
        story "Tiny" { start: hall, clock { start: "25:00" } }
        room hall "Hall" {}
        rule r on time("7 pm") { stop; }
    }
}

mod two_tests {
    use loremaster::prelude::*;
    world! {
        test t "One" { wait; }
        test t "Two" { wait; }
    }
}

mod bad_var {
    use loremaster::prelude::*;
    world! {
        story "Tiny" { start: hall }
        var mood: [calm, angry] = sad;
    }
}

fn main() {}
