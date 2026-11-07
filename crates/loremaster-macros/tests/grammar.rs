//! The grammar of `world!` makes the same model values as the builder, and
//! each entity gets an origin at its id in the block.

use loremaster::dsl::model::{
    Effect, EndingKind, EventPattern, FragmentMode, Layer, Location, States, Value, VerbSlots,
    WalkStep,
};
use loremaster::prelude::*;

mod lock {
    use loremaster::prelude::*;

    world! {
        extern {
            ending drowned;
        }

        story "The Lock" {
            author: "test",
            start: chamber,
            seed: 11,
            intro: "You arrive at night.",
            death_prompt: false,
            clock { start: "22:30", minutes_per_turn: 5, phases { night "00:00", day "07:00" } },
        }

        var lock_open: bool = false;
        var coins: int = 3;
        var mood: [calm, angry] = calm;

        room chamber "Lock Chamber" {
            at [1, -0.5];
            "Stone walls drop into the lock.";
            ambient cycle when night => ["Water knocks.", "A rope creaks."];
            first_visit "You have never seen it so dark.";
            exit west -> kitchen door hatch;
            exit up blocked "The ladder is gone.";
            tag dark;
            note "The lock is the centre of the slice.";
        }

        room kitchen "Kitchen" {
            exit east -> chamber visible not lock_open passable coins > 0 traversal "You duck.";
        }

        thing hatch "hatch" in kitchen {
            unlisted;
            prop open = false;
        }

        thing chest "chest" in chamber {
            container;
            capacity 2;
            key windlass;
        }

        thing windlass "windlass" inside chest {
            aliases [handle, "iron handle"];
            portable;
            examine "An iron handle.";
            presence when in chamber => "A windlass lies here.";
        }

        rule jump on verb(jump) instead once priority 2 when in(chamber) and not lock_open {
            line "The water closes over you.";
            set lock_open = true, mood = angry, hatch.open = true;
            add coins -= 1;
            move windlass -> carried, hatch -> nowhere;
            start drain for 2;
            emit splashed;
            end drowned;
        }

        rule splash on event(splashed) within chamber {
            goto kitchen;
            cancel drain;
            stop;
        }

        machine lock starts full {
            state full "The chamber is full.";
            state empty;
            full -> empty on verb(open, hatch) instead when windlass.carried {
                line "Water roars out.";
                line "Then it is quiet.";
                start drain;
            }
            [empty, full] -> full on timer(drain);
        }

        timer drain turns 3 {
            repeat;
            while not lock_open;
            at 1 { line "Almost."; }
            at 0 { line "Done."; }
        }

        ending free "Free" {
            kind win;
            rank 1;
            "You walk out at dawn.";
            todo "Write a longer text.";
        }

        verb jump "Jump" {
            words [jump, "jump in"];
            slots none;
            no_bar;
            default "You jump.";
        }

        verb pry "Pry" {
            words [pry];
            slots two;
            preposition with;
        }

        test drowning "Jump into the lock" {
            pending phase 4, needs "P4-07 #84";
            wait * 3;
            "open hatch" sees "Water" not_sees ["Nothing", "Never"] expect lock == empty, coins == 3;
            jump sees "closes over you" expect ended, ending drowned, room == chamber;
            expect ending drowned, not ended, mood == angry, not_sees "Nothing";
        }
    }
}

#[test]
fn the_story_header_and_vars() {
    let p = lock::pack();
    let h = p.header.as_ref().unwrap();
    assert_eq!(h.title, "The Lock");
    assert_eq!(h.author.as_deref(), Some("test"));
    assert_eq!(h.start.as_str(), "chamber");
    assert_eq!(h.seed, 11);
    assert!(!h.death_prompt);
    assert_eq!(h.clock.minutes_per_turn, 5);
    assert_eq!(h.clock.start, "22:30".parse().unwrap());
    assert_eq!(h.clock.phases.len(), 2);
    let vars: Vec<_> = h
        .vars
        .iter()
        .map(|v| (v.id.as_str(), v.initial.clone()))
        .collect();
    assert_eq!(
        vars,
        [
            ("lock_open", Value::Bool(false)),
            ("coins", Value::Int(3)),
            ("mood", Value::Text("calm".into())),
        ]
    );
}

#[test]
fn rooms_and_things() {
    let p = lock::pack();
    let chamber = &p.rooms[0];
    assert_eq!(chamber.position, Some([1.0, -0.5]));
    assert_eq!(chamber.description.len(), 3);
    assert_eq!(chamber.description[1].layer, Layer::Ambient);
    assert_eq!(chamber.description[1].mode, FragmentMode::Cycle);
    assert_eq!(chamber.description[1].when.as_deref(), Some("night"));
    assert_eq!(chamber.description[1].variants.len(), 2);
    assert_eq!(chamber.description[2].mode, FragmentMode::FirstVisit);
    assert_eq!(chamber.exits[0].door.as_ref().unwrap().as_str(), "hatch");
    assert_eq!(
        chamber.exits[1].blocked.as_deref(),
        Some("The ladder is gone.")
    );
    assert_eq!(chamber.tags, ["dark"]);
    assert_eq!(chamber.notes.len(), 1);
    let back = &p.rooms[1].exits[0];
    assert_eq!(back.visible.as_deref(), Some("not lock_open"));
    assert_eq!(back.passable.as_deref(), Some("coins > 0"));
    assert_eq!(back.traversal.as_deref(), Some("You duck."));

    let windlass = p
        .things
        .iter()
        .find(|t| t.id.as_str() == "windlass")
        .unwrap();
    assert_eq!(windlass.location, Location::In("chest".into()));
    assert_eq!(windlass.aliases, ["handle", "iron handle"]);
    assert!(windlass.portable);
    assert_eq!(windlass.presence[0].when.as_deref(), Some("in(chamber)"));
    let chest = p.things.iter().find(|t| t.id.as_str() == "chest").unwrap();
    assert!(chest.container);
    assert_eq!(chest.capacity, Some(2));
    assert_eq!(chest.key.as_ref().unwrap().as_str(), "windlass");
}

#[test]
fn rules_and_effects() {
    let p = lock::pack();
    let jump = &p.rules[0];
    assert_eq!(jump.on, EventPattern::Verb(vec!["jump".into()]));
    assert!(jump.instead && jump.once);
    assert_eq!(jump.priority, 2);
    assert_eq!(jump.when.as_deref(), Some("in(chamber) and not lock_open"));
    assert!(
        matches!(&jump.then[0], Effect::Say(t) if t.as_plain() == Some("The water closes over you."))
    );
    assert!(matches!(&jump.then[1], Effect::Set(m) if m.len() == 3));
    assert!(matches!(&jump.then[2], Effect::Add(m) if m["coins"] == -1));
    assert!(matches!(&jump.then[3], Effect::Move(m) if m.len() == 2));
    assert!(matches!(&jump.then[4], Effect::Start(_)));
    assert!(matches!(&jump.then[5], Effect::Emit(e) if e.as_str() == "splashed"));
    assert!(matches!(&jump.then[6], Effect::End(e) if e.as_str() == "drowned"));
    let splash = &p.rules[1];
    assert_eq!(splash.within.as_ref().unwrap().as_str(), "chamber");
    assert_eq!(splash.then.len(), 3);
}

#[test]
fn machines_timers_endings_verbs() {
    let p = lock::pack();
    let m = &p.machines[0];
    assert_eq!(m.start, "full");
    assert_eq!(m.states.len(), 2);
    // The first line of a step is its `say`; the next one is an effect.
    assert_eq!(
        m.steps[0].say.as_ref().and_then(|l| l.as_plain()),
        Some("Water roars out.")
    );
    assert!(
        matches!(&m.steps[0].then[0], Effect::Say(t) if t.as_plain() == Some("Then it is quiet."))
    );
    assert!(m.steps[0].instead);
    assert_eq!(
        m.steps[1].from,
        States::Many(vec!["empty".into(), "full".into()])
    );

    let t = &p.timers[0];
    assert_eq!(t.turns, 3);
    assert!(t.repeat);
    assert_eq!(t.runs_while.as_deref(), Some("not lock_open"));
    assert_eq!(t.at.len(), 2);

    let e = &p.endings[0];
    assert_eq!(e.kind, EndingKind::Win);
    assert_eq!(e.rank, Some(1));
    assert_eq!(e.text, "You walk out at dawn.");
    assert!(e.notes[0].todo);

    assert_eq!(p.verbs[0].slots, VerbSlots::None);
    assert!(!p.verbs[0].bar);
    assert_eq!(p.verbs[1].preposition.as_deref(), Some("with"));
}

#[test]
fn walkthrough_tests() {
    let p = lock::pack();
    let (key, w) = &p.walkthroughs[0];
    assert_eq!(key, "drowning");
    assert_eq!(w.name.as_deref(), Some("Jump into the lock"));
    assert_eq!(w.pending.as_ref().unwrap().phase, 4);
    assert_eq!(w.steps.len(), 5);
    assert!(matches!(&w.steps[2], WalkStep::Command(c) if c == "wait"));
    let WalkStep::Checked { command, expect } = &w.steps[3] else {
        panic!("{:?}", w.steps[3]);
    };
    assert_eq!(command, "open hatch");
    assert_eq!(expect.not_see, ["Nothing", "Never"]);
    assert_eq!(expect.facts["lock"], Value::Text("empty".into()));
    assert_eq!(expect.facts["coins"], Value::Int(3));
    let WalkStep::Checked { expect, .. } = &w.steps[4] else {
        panic!("{:?}", w.steps[4]);
    };
    assert_eq!(expect.ended, Some(true));
    assert_eq!(expect.room.as_ref().unwrap().as_str(), "chamber");
    assert_eq!(w.expect.ended, Some(false));
    assert_eq!(w.expect.not_see, ["Nothing"]);
    assert_eq!(w.expect.facts["mood"], Value::Text("angry".into()));
}

#[test]
fn origins_point_at_the_ids_in_the_block() {
    let p = lock::pack();
    let source = include_str!("grammar.rs");
    for (key, id) in [
        ("story", "\"The Lock\" {"),
        ("room:chamber", "chamber \"Lock Chamber\""),
        ("thing:windlass", "windlass \"windlass\" inside"),
        ("rule:jump", "jump on verb(jump)"),
        ("machine:lock", "lock starts full"),
        ("timer:drain", "drain turns 3"),
        ("ending:free", "free \"Free\""),
        ("verb:pry", "pry \"Pry\""),
    ] {
        let Origin::Rust {
            file,
            line,
            column,
            pack,
        } = &p.origins[key]
        else {
            panic!("{key}: {:?}", p.origins[key]);
        };
        assert!(file.ends_with("tests/grammar.rs"), "{key}: {file}");
        assert_eq!(pack, "grammar::lock");
        let at = source.lines().nth(*line as usize - 1).unwrap();
        assert!(
            at[*column as usize - 1..].starts_with(id),
            "{key}: line {line}, column {column}: {at}"
        );
    }
}

/// A pack with ids of another pack (`extern`) joins with that pack.
mod endings {
    use loremaster::prelude::*;

    world! {
        ending drowned "Drowned" {
            kind death;
            "The lock keeps what falls into it.";
        }
    }
}

#[test]
fn packs_join_and_check() {
    let (project, _warnings) = Loremaster::new()
        .pack(lock::pack())
        .pack(endings::pack())
        .check()
        .unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(project.story.endings.len(), 2);
    // Without the endings pack, `end drowned` is unknown at load time.
    let errors = Loremaster::new().pack(lock::pack()).check().unwrap_err();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("unknown ending `drowned`")),
        "{errors:#?}"
    );
}
