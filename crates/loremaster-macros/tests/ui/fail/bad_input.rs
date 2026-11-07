// Input that the grammar rejects with a clear error instead of a panic, a
// hang or dropped tokens.

mod too_many_repeats {
    use loremaster::prelude::*;
    world! {
        test t "T" { wait * 4294967295; }
    }
}

mod add_overflow {
    use loremaster::prelude::*;
    world! {
        rule r on turn { add score -= -9223372036854775808; }
    }
}

mod three_numbers {
    use loremaster::prelude::*;
    world! {
        room hall "Hall" { at [1, 2, 3]; }
    }
}

mod two_times {
    use loremaster::prelude::*;
    world! {
        rule r on time("07:00" "08:00") { stop; }
    }
}

mod infinite_position {
    use loremaster::prelude::*;
    world! {
        room hall "Hall" { at [1e40, 0]; }
    }
}

mod no_phases {
    use loremaster::prelude::*;
    world! {
        story "Tiny" { start: hall, clock { phases {} } }
    }
}

mod extern_machine_without_states {
    use loremaster::prelude::*;
    world! {
        extern { machine door; }
    }
}

mod extern_var_with_story {
    use loremaster::prelude::*;
    world! {
        extern { var visits: int; }
        story "Tiny" { start: hall }
        room hall "Hall" {}
    }
}

fn main() {}
