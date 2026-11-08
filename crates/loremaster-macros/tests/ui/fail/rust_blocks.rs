use loremaster::prelude::*;

world! {
    pack two_blocks;
    story "T" { start: a }
    room a "A" {}
    rule r on turn {
        rust |cx| { cx.say("one"); }
        rust |cx| { cx.say("two"); }
    }
}

world! {
    pack in_a_timer;
    story "T" { start: a }
    room a "A" {}
    timer t turns 1 {
        at 0 { rust |cx| { cx.say("x"); } }
    }
}

world! {
    pack not_a_closure;
    story "T" { start: a }
    room a "A" {}
    rule r on turn {
        rust { }
    }
}

world! {
    pack two_in_a_step;
    story "T" { start: a }
    room a "A" {}
    machine m starts x {
        state x;
        state y;
        x -> y on turn {
            rust |cx| { cx.say("one"); }
            rust |cx| { cx.say("two"); }
        }
    }
}

world! {
    pack call_an_instance;
    story "T" { start: a }
    room a "A" {}
    rule r on turn { call bell; }
}

fn main() {}
