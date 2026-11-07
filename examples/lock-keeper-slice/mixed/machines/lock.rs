// The lock machine in world!, in place of ../../yaml/machines/lock.yaml. The
// chamber empties when the player opens the lower paddles with the windlass.
// The pages are in the gear housing at the bottom. Part "Today": the YAML
// form loads now. This form waits for Pack and assemble (DSL step L1) and
// for machines in world! (step L4).
//
// Draft syntax. It does not compile. Read ../README.md.

use loremaster::prelude::*;

world! {
    // The YAML pack defines these ids. The macro cannot see them, so they
    // are declared here, and `check` finds them at load time (open mode).
    // draft: `extern` with the kind timer. The DSL doc shows only thing
    // and room.
    extern {
        thing windlass;
        thing lower_paddles;
        room lock_chamber;
        timer lock_drain;
    }

    machine lock starts full {
        state full "The chamber is full. The gear housing is under water.";
        state emptying "The lower paddles are open. The chamber is empty 3 turns after they open, the opening turn included (timer lock_drain).";
        state empty "The chamber is empty. The player can open the gear housing.";

        full -> emptying on verb(open, lower_paddles) instead when windlass.carried {
            line "You fit the windlass and turn. Water roars out under the lower gate.";
            start lock_drain;
        }
        emptying -> empty on timer(lock_drain) {
            line "{in(lock_chamber) ? The chamber is empty. Wet ladder rungs go down into the dark. | }";
        }
    }
}
