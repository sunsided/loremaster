// `combat` needs its rooms, `await` its kind, and the dice must read.

mod no_loss {
    use loremaster::prelude::*;
    world! {
        rule fight on turn {
            combat [warden] win yard;
        }
    }
}

mod no_kind {
    use loremaster::prelude::*;
    world! {
        rule wait on turn {
            await dice "2d6";
        }
    }
}

mod bad_dice {
    use loremaster::prelude::*;
    world! {
        story "Dice" { start: hall }
        var r = 0;
        room hall "Hall" {}
        rule roll on turn {
            await roll "2x6" into r;
        }
    }
}

fn main() {}
