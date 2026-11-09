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

mod twice {
    use loremaster::prelude::*;
    world! {
        story "Twice" { start: hall, stats { luck "LUCK" short "LK" short "L" } }
        room hall "Hall" {}
    }
}

mod twice_win {
    use loremaster::prelude::*;
    world! {
        rule fight on turn {
            combat [warden] win yard win hall loss fallen;
        }
    }
}

mod twice_arg {
    use loremaster::prelude::*;
    world! {
        rule pick on turn {
            await action pick_lock { difficulty = 3, difficulty = 4 };
        }
    }
}

mod twice_into {
    use loremaster::prelude::*;
    world! {
        rule roll on turn {
            await roll "2d6" into r into s;
        }
    }
}

fn main() {}
