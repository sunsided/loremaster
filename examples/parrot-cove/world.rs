// The reference example of DSL step L5 (P5-05, #92): Rust behaviors in
// world!. It has the three forms: a behavior instance (`behavior <expr>;`),
// a `#[behavior]` function and inline `rust |cx| { .. }` blocks, in a rule
// and in a machine step. Test: crates/loremaster-cli/tests/behaviors_dsl.rs.

use loremaster::prelude::*;

/// The parrot squawks on some turns. The roll is in the save state.
#[behavior(on = turn)]
pub fn parrot(cx: &mut Cx<'_>) -> Flow {
    if cx.roll(2) == 0 {
        cx.say("\"Pieces of eight!\" squawks the parrot.");
    }
    Flow::Continue
}

/// A bell that rings every `every` turns. A hand-written `Behavior`.
#[derive(Debug)]
pub struct Bell {
    every: u32,
}

impl Bell {
    pub fn new(every: u32) -> Self {
        Bell { every }
    }
}

impl Behavior for Bell {
    fn id(&self) -> &str {
        "bell"
    }

    fn manifest(&self) -> Manifest {
        Manifest::new().events([loremaster::dsl::runtime::EventKind::Turn])
    }

    fn handle(&self, _: &Event, cx: &mut Cx<'_>) -> Flow {
        if cx.turns().is_multiple_of(self.every) {
            cx.say("A bell rings over the water.");
        }
        Flow::Continue
    }
}

world! {
    story "Parrot Cove" { start: cove, seed: 7 }

    behavior Parrot;
    behavior Bell::new(3);

    room cove "The Cove" { "Gold glints beside an old scale."; }
    thing gold "gold coin" in cove { aliases [gold, coin]; portable; }
    thing scale "brass scale" in cove { unlisted; prop weight = 0; }

    verb weigh "Weigh" { words [weigh]; slots none; }
    verb pull "Pull" { words [pull]; slots none; }

    rule weigh_gold on verb(weigh) instead {
        line "The scale tips.";
        rust |cx| {
            let n = cx.carried().len() as i64;
            cx.set("scale.weight", Value::Int(n * 3));
            cx.say(format!("It reads {}.", n * 3));
        }
        line "It settles.";
    }

    machine sluice starts shut {
        state shut;
        state open;
        shut -> open on verb(pull) instead {
            line "The sluice grinds open.";
            rust |cx| { cx.say(format!("Turn {}.", cx.turns())); }
        }
    }

    test weighing "Weigh the gold" {
        "take gold";
        weigh sees "It reads 3.";
        pull sees "The sluice grinds open.";
        expect scale.weight == 3, sluice == open;
    }
}
