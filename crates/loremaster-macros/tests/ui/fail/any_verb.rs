// `*` is any verb only in the verb place of `verb(..)` and `check(..)`.
mod object {
    use loremaster::prelude::*;
    world! {
        rule r on verb(take, *) {
            stop;
        }
    }
}

mod talk {
    use loremaster::prelude::*;
    world! {
        rule r on talk(*) {
            stop;
        }
    }
}

fn main() {}
