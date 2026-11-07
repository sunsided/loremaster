// Errors of the grammar: an unknown item, and a missing `;`.
mod unknown_item {
    use loremaster::prelude::*;
    world! {
        chamber hall "Hall" {}
    }
}

mod missing_semicolon {
    use loremaster::prelude::*;
    world! {
        room hall "Hall" {
            "A hall."
            exit north -> garden;
        }
    }
}

mod unknown_effect {
    use loremaster::prelude::*;
    world! {
        rule r on turn { say "Hello."; }
    }
}

mod end_with_text {
    use loremaster::prelude::*;
    world! {
        rule r on turn { end "You win."; }
    }
}

fn main() {}
