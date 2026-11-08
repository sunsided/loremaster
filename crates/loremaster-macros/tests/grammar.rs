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

        rule talk on verb(examine, chest) {
            line "He shrugs." keeper says "Prove it," narrate "he says.";
            line says "Who is there?" player says "Me." when lock_open then "Water drips."
                when coins > 1 then keeper says "Pay up.";
        }

        thing keeper "keeper" in chamber {}

        // Speakers with the ids of segment words.
        thing narrate "narrator" in chamber {}
        thing says "parrot" in chamber {}

        rule odd_speakers on verb(examine, says) {
            line narrate says "I tell the story." says says "Squawk.";
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
            [empty, full] -> full on timer(drain) {
                line when in chamber then "Back to full.";
            }
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
            "open hatch" sees "Water" not_sees ["Nothing", "Never"] expect lock == empty, coins == 3, not ended;
            jump sees "closes over you" expect ended, ending drowned, room == chamber;
            expect ending drowned, ended, mood == angry, not_sees "Nothing";
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
    assert_eq!(jump.on, EventPattern::Verb("jump".into()));
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
    let splash = p.rules.iter().find(|r| r.id.as_str() == "splash").unwrap();
    assert_eq!(splash.within.as_ref().unwrap().as_str(), "chamber");
    assert_eq!(splash.then.len(), 3);
}

#[test]
fn speech_segments() {
    use loremaster::dsl::model::{Line, LineSegment};
    let p = lock::pack();
    let talk = &p.rules[1];
    let lines: Vec<&Line> = talk
        .then
        .iter()
        .map(|e| match e {
            Effect::Say(l) => l,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        lines[0],
        &Line::new(vec![
            LineSegment::plain("He shrugs."),
            LineSegment::says("Prove it,").by("keeper"),
            LineSegment::narrate("he says."),
        ])
    );
    assert_eq!(
        lines[1],
        &Line::new(vec![
            LineSegment::says("Who is there?"),
            LineSegment::says("Me.").by("player"),
            LineSegment::narrate("Water drips.").when("lock_open"),
            LineSegment::says("Pay up.").by("keeper").when("coins > 1"),
        ])
    );
    let odd = p
        .rules
        .iter()
        .find(|r| r.id.as_str() == "odd_speakers")
        .unwrap();
    assert!(
        matches!(&odd.then[0], Effect::Say(l) if *l == Line::new(vec![
            LineSegment::says("I tell the story.").by("narrate"),
            LineSegment::says("Squawk.").by("says"),
        ]))
    );
    // A machine step's conditional line is its `say`.
    let m = &p.machines[0];
    assert_eq!(
        m.steps[1].say,
        Some(Line::new(vec![
            LineSegment::narrate("Back to full.").when("in(chamber)")
        ]))
    );
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
    let WalkStep::Checked {
        command, expect, ..
    } = &w.steps[3]
    else {
        panic!("{:?}", w.steps[3]);
    };
    assert_eq!(command, "open hatch");
    assert_eq!(expect.not_see, ["Nothing", "Never"]);
    assert_eq!(expect.facts["lock"], Value::Text("empty".into()));
    assert_eq!(expect.facts["coins"], Value::Int(3));
    assert_eq!(expect.ended, Some(false));
    let WalkStep::Checked { expect, .. } = &w.steps[4] else {
        panic!("{:?}", w.steps[4]);
    };
    assert_eq!(expect.ended, Some(true));
    assert_eq!(expect.room.as_ref().unwrap().as_str(), "chamber");
    assert_eq!(w.expect.ended, Some(true));
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

/// A condition that starts with an id `says` is a condition, not a speaker:
/// `<speaker> says` needs a string right after `says`.
mod says_var {
    use loremaster::prelude::*;

    world! {
        story "Tiny" { start: hall }
        var says: bool = false;
        room hall "Hall" {}
        rule dark on turn {
            line when says then "Dark." when not says then says "Light.";
        }
    }
}

#[test]
fn a_condition_can_start_with_an_id_says() {
    use loremaster::dsl::model::{Line, LineSegment};
    let p = says_var::pack();
    assert!(
        matches!(&p.rules[0].then[0], Effect::Say(l) if *l == Line::new(vec![
            LineSegment::narrate("Dark.").when("says"),
            LineSegment::says("Light.").when("not says"),
        ]))
    );
}

/// A room `then` is an operand of a segment condition: the marker is the
/// `then` that a segment follows.
mod then_room {
    use loremaster::prelude::*;

    world! {
        story "Then" { start: then }
        var mood: [calm, then] = calm;
        room then "Then" {}
        thing mara "Mara" in then {}
        rule dark on turn {
            line when in then then "Dark." when mood == then then says "Hm.";
        }
        dialogue with mara start [hi] {
            hi: "Hi." {
                says "Bye." when in then => end;
            }
        }
    }
}

#[test]
fn a_then_operand_is_not_the_marker() {
    use loremaster::dsl::model::{Line, LineSegment};
    let p = then_room::pack();
    assert_eq!(
        p.rules[0].then[0],
        Effect::Say(Line::new(vec![
            LineSegment::narrate("Dark.").when("in(then)"),
            LineSegment::says("Hm.").when("mood == then"),
        ]))
    );
    assert_eq!(
        p.dialogues[0].nodes[0].options[0].when.as_deref(),
        Some("in(then)")
    );
}

/// Dialogues (DSL step L4): dramaturge's token form, with the conditions,
/// lines and effects of `world!`. Two blocks share a module with `pack`.
mod talks {
    pub mod cantina {
        use loremaster::prelude::*;

        world! {
            pack cantina;

            story "Cantina" { start: deck, relations: [trust] }
            var knows_blackbeard_name: bool = false;
            room deck "Deck" {}
            thing blackbeard "Blackbeard" in deck {}

            dialogue blackbeard_talk with blackbeard start [again, meet] {
                meet: "The figure looks up." says "Sit, if you like." {
                    says "Who are you?" => names;
                    says "Not tonight." => end;
                }
                names: says "They call me Blackbeard." {
                    set knows_blackbeard_name = true;
                    relation trust += 1;
                    says "Where did the *Marigold* go?" when blackbeard.trust >= 1 => back {
                        id marigold;
                        remember asked_marigold;
                    }
                }
                again: says "Back again." {
                    when blackbeard.talked > 1;
                    says "Goodbye." => end { then goto deck, line "You leave."; }
                }
            }
        }
    }

    pub mod keeper {
        use loremaster::prelude::*;

        world! {
            pack keeper;

            extern { thing keeper; }

            // Without an id, the dialogue has the id of its thing.
            dialogue with keeper start [hello] {
                hello: "He nods." {
                    narrate "You nod back." player says "Evening." => end;
                }
            }
        }
    }
}

/// More lines of a node and `-> <node>` (`next`): the same dialogue as
/// the YAML form.
mod flow {
    use loremaster::prelude::*;

    world! {
        pack flow;

        extern { thing keeper; thing dog; var late; }

        dialogue with keeper start [hi] {
            hi: "He looks up." {
                player: says "Evening.";
                dog: says "Woof." when late then narrate "It is late.";
                -> ask;
            }
            ask {
                keeper: says "Anything else?";
                says "Again." => back;
                says "Bye." => end;
            }
            story: "He tells a story." {
                -> end;
            }
        }
    }
}

const FLOW_YAML: &str = r#"
id: keeper
with: keeper
start: [hi]
nodes:
  hi:
    say: "He looks up."
    lines:
      - { speaker: player, say: [{ says: "Evening." }] }
      - { speaker: dog, say: [{ says: "Woof." }, { narrate: "It is late.", when: late }] }
    next: ask
  ask:
    speaker: keeper
    say: [{ says: "Anything else?" }]
    options:
      - { say: [{ says: "Again." }], go: back }
      - { say: [{ says: "Bye." }], go: end }
  story:
    say: "He tells a story."
    next: end
"#;

#[test]
fn lines_and_next_are_the_yaml_dialogue() {
    use loremaster::dsl::project::SourceFile;
    let yaml = Pack::from_files(vec![SourceFile {
        path: "dialogues/keeper.yaml".into(),
        text: FLOW_YAML.into(),
    }])
    .unwrap();
    assert_eq!(flow::flow().dialogues, yaml.dialogues);
}

const BLACKBEARD_YAML: &str = r#"
id: blackbeard_talk
with: blackbeard
start: [again, meet]
nodes:
  meet:
    say: ["The figure looks up.", { says: "Sit, if you like." }]
    options:
      - say: [{ says: "Who are you?" }]
        go: names
      - say: [{ says: "Not tonight." }]
        go: end
  names:
    say: [{ says: "They call me Blackbeard." }]
    set: { knows_blackbeard_name: true }
    relation: { trust: 1 }
    options:
      - id: marigold
        say: [{ says: "Where did the *Marigold* go?" }]
        when: blackbeard.trust >= 1
        remember: [asked_marigold]
        go: back
  again:
    when: blackbeard.talked > 1
    say: [{ says: "Back again." }]
    options:
      - say: [{ says: "Goodbye." }]
        go: end
        then:
          - goto: deck
          - say: "You leave."
"#;

#[test]
fn dialogues_are_the_yaml_dialogues() {
    use loremaster::dsl::project::SourceFile;
    let yaml = Pack::from_files(vec![SourceFile {
        path: "dialogues/blackbeard_talk.yaml".into(),
        text: BLACKBEARD_YAML.into(),
    }])
    .unwrap();
    let p = talks::cantina::cantina();
    assert_eq!(p.dialogues, yaml.dialogues);
    let Origin::Rust { line, .. } = &p.origins["dialogue:blackbeard_talk"] else {
        panic!("not from Rust");
    };
    let source = include_str!("grammar.rs");
    let at = source.lines().nth(*line as usize - 1).unwrap();
    assert!(at.contains("dialogue blackbeard_talk with"), "{at}");

    let keeper = talks::keeper::keeper();
    assert_eq!(keeper.dialogues[0].id.as_deref(), Some("keeper"));
    assert!(
        keeper.name.ends_with("talks::keeper::keeper"),
        "{}",
        keeper.name
    );
    let (project, warnings) = Loremaster::new()
        .pack(p)
        .check()
        .unwrap_or_else(|e| panic!("{e:#?}"));
    assert!(warnings.is_empty(), "{warnings:#?}");
    assert_eq!(project.story.dialogues.len(), 1);
}

/// Threads, behaviors, frames, name stages, the score, fragment fields,
/// `replace` and extern properties (DSL step L4).
mod more {
    use loremaster::prelude::*;

    world! {
        extern {
            thing stove [hot: bool, fuel: int];
            room kitchen;
        }

        thread stove_secret "Who fed the stove?" {
            status resolved;
            summary "The stove is warm at dawn.";
            tracks ash_seen;
            note "Pays off in the keeper's room.";
        }

        behavior thief;

        replace thing poker "poker" in kitchen {
            name "iron rod" aliases [rod];
            name when poker.cleaned => "Poker" proper;
            prop cleaned = false;
            "A poker." id poker_text replaces note "The first text.";
            before examine "You lean closer.";
            instead take when stove.hot => "It is too hot.";
            after take "It is heavy.";
            when stove.fuel > 2 => "Ash clings to it.";
        }
    }
}

mod scored {
    use loremaster::prelude::*;

    world! {
        story "Score" {
            start: hall,
            score: 10,
            score_var: points,
            ranks { 0 "Beginner", 5 "Amateur" },
        }
        var points: int = 0;
        var ash_seen: bool = false;
        room hall "Hall" {
            before look "The hall is quiet.";
        }
        timer bell turns 3 {
            note "Rings at the third turn.";
        }
        machine door starts shut {
            todo "Add a locked state.";
            state shut;
            state open;
            shut -> open on verb(open);
        }
    }
}

#[test]
fn threads_frames_names_and_the_score() {
    use loremaster::dsl::model::{Id, NameStage, ThreadStatus};
    let p = more::pack();
    let t = &p.threads[0];
    assert_eq!(t.id.as_str(), "stove_secret");
    assert_eq!(t.status, ThreadStatus::Resolved);
    assert_eq!(t.summary.as_deref(), Some("The stove is warm at dawn."));
    assert_eq!(t.notes.len(), 1);
    assert_eq!(t.tracks, ["ash_seen"]);
    assert_eq!(p.behaviors[0].as_str(), "thief");
    assert!(p.replaces.contains("thing:poker"));
    let poker = &p.things[0];
    assert_eq!(
        poker.names,
        [
            NameStage {
                text: "iron rod".into(),
                aliases: vec!["rod".into()],
                when: None,
                proper: false,
            },
            NameStage {
                text: "Poker".into(),
                aliases: vec![],
                when: Some("poker.cleaned".into()),
                proper: true,
            },
        ]
    );
    let first = &poker.description[0];
    assert_eq!(first.id.as_ref().map(|i| i.as_str()), Some("poker_text"));
    assert!(first.replaces);
    assert_eq!(first.notes.len(), 1);
    let take = &poker.frames[&Id::new("take")];
    assert_eq!(take.instead[0].when.as_deref(), Some("stove.hot"));
    assert_eq!(take.after.len(), 1);
    assert_eq!(poker.frames[&Id::new("examine")].before.len(), 1);

    let s = scored::pack();
    let score = s.header.as_ref().unwrap().score.as_ref().unwrap();
    assert_eq!((score.var.as_str(), score.max), ("points", 10));
    assert_eq!(score.ranks[1].name, "Amateur");
    assert_eq!(s.rooms[0].frames.len(), 1);
    assert_eq!(s.timers[0].notes.len(), 1);
    assert!(s.machines[0].notes[0].todo);
}

/// A `then` list in a dialogue: an effect that takes a list (`set`) ends
/// where the next effect starts, and `x.then` in a condition is a
/// property, not the marker of a segment condition.
mod then_lists {
    use loremaster::prelude::*;

    world! {
        story "Then" { start: deck }
        var flag: bool = false;
        room deck "Deck" {}
        thing mara "Mara" in deck { prop then = 0; }

        dialogue with mara start [hi] {
            hi: "Hi." when mara.then == 1 then "She smiles." {
                says "Bye." when mara.then == 0 => end {
                    then set flag = true, mara.then = 2, goto deck, line "You go.", stop;
                }
            }
        }
    }
}

#[test]
fn then_lists_and_then_properties() {
    use loremaster::dsl::model::{Line, LineSegment};
    let p = then_lists::pack();
    let node = &p.dialogues[0].nodes[0];
    assert_eq!(
        node.say,
        Line::new(vec![
            LineSegment::plain("Hi."),
            LineSegment::narrate("She smiles.").when("mara.then == 1"),
        ])
    );
    let option = &node.options[0];
    assert_eq!(option.when.as_deref(), Some("mara.then == 0"));
    assert_eq!(
        option.outcomes.then,
        [
            Effect::Set(
                [
                    ("flag".to_string(), Value::Bool(true)),
                    ("mara.then".to_string(), Value::Int(2)),
                ]
                .into()
            ),
            Effect::Goto(loremaster::dsl::model::Id::new("deck")),
            Effect::Say(Line::from("You go.")),
            Effect::Stop,
        ]
    );
}
