//! DS-3: the builder makes the same model values as YAML. The Lighthouse
//! (`examples/lighthouse`) written with the builder must equal the Lighthouse
//! from YAML, value by value and by `model_hash`.
//!
//! The entities are written in the order in which the YAML loader reads the
//! files (by path), so that the lists of the two stories have the same order.

use loremaster_dsl::prelude::*;
use loremaster_project::{Project, model_hash};
use std::path::{Path, PathBuf};

fn yaml_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse")
}

/// The Lighthouse, written with the builder.
fn lighthouse() -> Pack {
    let mut p = Pack::new("lighthouse");

    p.story("The Lighthouse", "shore")
        .author("loremaster example")
        .seed(7)
        .intro(
            "The ferry left you on the shingle an hour ago. Old Hal has not lit the lighthouse \
             in three nights, and the *Marigold* is due in after dark.",
        )
        .clock_start("20:30")
        .minutes_per_turn(5)
        .phases([
            ("night", "00:00"),
            ("dawn", "05:00"),
            ("day", "07:00"),
            ("dusk", "19:00"),
            ("night", "21:00"),
        ])
        .var("beacon_lit", false)
        .var("drawer_searched", false)
        // "Look around" teases a little when the player repeats it.
        .frame_before("look", frag("You look around.").when("action.repeat == 1"))
        .frame_before(
            "look",
            frag("You look around, again.").when("action.repeat == 2"),
        )
        .frame_before(
            "look",
            frag("You keep looking around, and nothing seems to change.")
                .when("action.repeat >= 3"),
        );

    p.ending("beacon_lit", EndingKind::Win, "The beacon is lit")
        .rank(1)
        .text(
            "{night: Far below, the *Marigold*'s masthead light turns, and steadies, and comes \
             home. | Hours early, the beam goes round and round in the {time.phase} light. By \
             dark, the *Marigold* will see it.}",
        );

    p.room("cottage", "Keeper's Cottage")
        .at(0.0, -1.0)
        .text(
            "One room, smelling of tar and pipe smoke. A narrow bed, a table with a single \
             chair, and a dresser with a deep drawer{drawer.open ?, hanging open | }.",
        )
        .text(
            frag("Through the salt-crusted window the sea has gone black.")
                .ambient()
                .when("night"),
        )
        .text(
            frag("Hal's oilskin still hangs by the door. Wherever he went, he went without it.")
                .when("first_visit")
                .once(),
        )
        .exit(Direction::South, "shore")
        .door("cottage_door");

    p.room("lantern_room", "Lantern Room")
        .at(1.0, -1.0)
        .text(
            "A glass room at the top of the world. The great lens sits in its brass cradle, a \
             beehive of prisms {beacon_lit ? throwing light in every direction | dull and cold}.",
        )
        .text(
            frag(
                "Beyond the glass there is only dark water, and the one small light of the \
                 *Marigold*.",
            )
            .ambient()
            .when("night"),
        )
        .exit(Direction::Down, "stairwell");

    p.room("shore", "The Shore")
        .tag("outdoors")
        .at(0.0, 0.0)
        .text(
            "Shingle crunches underfoot. To the north a whitewashed cottage huddles against the \
             wind{first: , its windows dark}. East, the lighthouse rises {night: black against \
             the stars | pale against the {time.phase} sky}.",
        )
        .text(
            frag("The sun is a red coal sinking into the sea.")
                .ambient()
                .when("time.phase == dusk"),
        )
        .text(
            variants([
                "Waves hiss on the stones.",
                "Somewhere out at sea a bell buoy clangs.",
                "The wind drops, then gusts again, cold off the water.",
            ])
            .ambient()
            .when("time.phase == night")
            .shuffle(),
        )
        .text(
            frag(
                "Far out, a single masthead light rocks on the swell. The *Marigold*, feeling \
                 her way in.",
            )
            .event()
            .when("night and not beacon_lit"),
        )
        .exit(Direction::North, "cottage")
        .door("cottage_door")
        .exit(Direction::East, "stairwell")
        .door("lighthouse_door")
        .exit_blocked(
            Direction::West,
            "Cold surf breaks over the rocks. There is only the sea that way.",
        );

    p.room("stairwell", "Lighthouse Stairwell")
        .tag("dark")
        .at(1.0, 0.0)
        .text(
            "Iron stairs spiral up the inside of the tower, cold to the touch and gritty with \
             sand. Every footstep rings.",
        )
        .text(
            variants([
                "Far above, something creaks.",
                "The tower hums faintly in the wind.",
            ])
            .ambient()
            .cycle(),
        )
        .exit(Direction::West, "shore")
        .door("lighthouse_door")
        .exit(Direction::Up, "lantern_room")
        .traversal("You climb, and climb, and climb. Ninety-one steps; you count them.");

    p.rule("light_the_beacon", on::verb(["turn_on", "lens"]))
        .when("oil_lamp.carried and oil_lamp.lit")
        .instead()
        .set("beacon_lit", true)
        .set("lens.lit", true)
        .say(
            "You touch the lamp's flame to the wick. It catches, steadies, and then the lens \
             takes it and throws it out across the water, a great slow wheel of light.",
        )
        .end("beacon_lit");

    p.rule("dusk_falls", on::time("21:00"))
        .say("The last of the light drains out of the sky.");

    p.rule("first_climb", on::enter("lantern_room"))
        .once()
        .say("You are out of breath, and the wind leans on the glass.");

    p.rule("find_iron_key", on::verb(["open", "drawer"]))
        .when("not drawer_searched")
        .set("drawer_searched", true)
        .move_to("iron_key", "cottage")
        .say("Under a fold of oilcloth lies an iron key.");

    p.rule("lamp_needs_matches", on::verb(["turn_on", "oil_lamp"]))
        .when("not matches.carried")
        .instead()
        .say("You have nothing to light it with.");

    p.rule("no_flame_for_lens", on::verb(["turn_on", "lens"]))
        .when("not (oil_lamp.carried and oil_lamp.lit)")
        .instead()
        .say("You need a flame to light the wick.");

    // The shortest way to light the beacon.
    p.test("Light the beacon")
        .step("n")
        .step("open drawer")
        .sees("Under a fold of oilcloth lies an iron key.")
        .step("take key")
        .step("take lamp")
        .step("take matches")
        .step("light lamp")
        .step("s")
        .step("unlock iron door with key")
        .step("e")
        .step("up")
        .step("light beacon")
        .expect_fact("beacon_lit", true)
        .expect_room("lantern_room")
        .expect_ended(true)
        .expect_ending("beacon_lit");

    p.thing("cottage_door", "cottage door")
        .aliases(["door", "cottage door"])
        .in_room("shore")
        .unlisted()
        .prop("open", false)
        .text("Weathered planks, a latch worn smooth by years of thumbs. It is {cottage_door.open ? open | shut}.");

    p.thing("drawer", "drawer")
        .aliases(["dresser"])
        .in_room("cottage")
        .unlisted()
        .prop("open", false)
        .text("A deep drawer in an old pine dresser. It is {drawer.open ? open | closed}.");

    p.thing("iron_key", "iron key")
        .aliases(["key"])
        .nowhere()
        .portable()
        .text("Long and black, with a tag in Hal's hand: TOWER.");

    p.thing("lens", "great lens")
        .aliases(["lens", "beacon", "light"])
        .in_room("lantern_room")
        .unlisted()
        .prop("lit", false)
        .text(
            "Hundreds of glass prisms, stacked like the rings of a hive. At its heart a wick \
             sits in a reservoir of oil, waiting for a flame.",
        );

    p.thing("lighthouse_door", "lighthouse door")
        .aliases(["door", "iron door"])
        .in_room("shore")
        .unlisted()
        .key("iron_key")
        .prop("open", false)
        .prop("locked", true)
        .text(
            "Riveted iron, streaked with rust. {lighthouse_door.locked ? A heavy lock holds it \
             fast. | It stands unlocked.}",
        );

    p.thing("matches", "box of matches")
        .aliases(["matches", "box"])
        .in_room("cottage")
        .portable()
        .text("Swan Vestas. Six left.")
        .presence("A box of matches lies on the table.");

    p.thing("oil_lamp", "oil lamp")
        .aliases(["lamp", "lantern"])
        .in_room("cottage")
        .portable()
        .prop("lit", false)
        .text(
            "A storm lantern with a cracked chimney, half full of oil. It is {oil_lamp.lit ? \
             burning steadily | unlit}.",
        )
        .presence("An oil lamp sits on the table{oil_lamp.lit ?, burning | }.");

    p
}

fn from_builder() -> Project {
    Loremaster::new().pack(lighthouse()).assemble().unwrap()
}

fn from_yaml() -> Project {
    Loremaster::new().yaml(yaml_dir()).assemble().unwrap()
}

#[test]
fn the_builder_lighthouse_equals_the_yaml_lighthouse() {
    let (built, yaml) = (from_builder(), from_yaml());
    assert_eq!(built.story, yaml.story);
    assert_eq!(model_hash(&built.story), model_hash(&yaml.story));
    // The load path of the CLI gives the same story.
    let loaded =
        loremaster_project::load(loremaster_project::read_dir(&yaml_dir()).unwrap()).unwrap();
    assert_eq!(built.story, loaded.story);
}

#[test]
fn the_builder_walkthrough_equals_the_yaml_walkthrough() {
    let (built, yaml) = (from_builder(), from_yaml());
    let built: Vec<_> = built.walkthroughs.iter().map(|(_, w)| w).collect();
    let yaml: Vec<_> = yaml.walkthroughs.iter().map(|(_, w)| w).collect();
    assert_eq!(built, yaml);
}

#[test]
fn each_entity_has_its_rust_origin() {
    let project = from_builder();
    let origin = &project.origins["room:cottage"];
    let Origin::Rust {
        file,
        line,
        column,
        pack,
    } = origin
    else {
        panic!("not a Rust origin: {origin}");
    };
    assert!(file.ends_with("tests/lighthouse.rs"), "{file}");
    assert_eq!(pack, "lighthouse");
    assert!(*line > 0 && *column > 0);
    // Each entity has its own line: the call that made it.
    let lines: std::collections::BTreeSet<u32> = project
        .origins
        .values()
        .filter_map(|o| match o {
            Origin::Rust { line, .. } => Some(*line),
            _ => None,
        })
        .collect();
    // The story header, 1 ending, 4 rooms, 6 rules and 7 things.
    assert_eq!(lines.len(), 19);
    // The walkthrough is keyed by its Rust file and line.
    assert!(project.walkthroughs[0].0.contains("tests/lighthouse.rs:"));
}

#[test]
fn the_builder_lighthouse_checks_and_plays() {
    let (_, warnings) = Loremaster::new().pack(lighthouse()).check().unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let world = Loremaster::new().pack(lighthouse()).build().unwrap();
    assert_eq!(world.story.title, "The Lighthouse");
}

#[test]
fn a_builder_pack_and_a_yaml_pack_join() {
    // The YAML Lighthouse, and a Rust pack that adds a room.
    let mut extra = Pack::new("extra");
    extra.room("gallery", "Gallery").text("Wind.");
    let project = Loremaster::new()
        .yaml(yaml_dir())
        .pack(extra)
        .assemble()
        .unwrap();
    assert!(project.story.room("gallery").is_some());
    assert_eq!(
        project.origins["room:shore"],
        Origin::yaml("rooms/shore.yaml")
    );
    // The same id in both packs is an error that names both origins.
    let mut twice = Pack::new("twice");
    twice.room("shore", "Shore");
    let errors = Loremaster::new()
        .yaml(yaml_dir())
        .pack(twice)
        .assemble()
        .unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("rooms/shore.yaml")
            && errors[0].message.contains("(pack twice)"),
        "{}",
        errors[0].message
    );
}
