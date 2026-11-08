//! `interface` makes the host types, and the story checks against it.

use loremaster::prelude::*;

world! {
    story "Host" { start: hall }
    var lock: [closed, open_it, shut] = closed;
    room hall "Hall" { "A hall."; }
    rule hear on event(killed) { line "Dead."; }

    interface {
        action lockpick(difficulty: int) -> open_it | shut;
        event killed(boss: text);
        event found;
    }
}

fn handled(r: interface::Request) -> i64 {
    // Non-exhaustive without the arm: an undeclared request does not compile.
    match r {
        interface::Request::Lockpick { difficulty } => difficulty,
    }
}

fn main() {
    assert_eq!(handled(interface::Request::Lockpick { difficulty: 2 }), 2);
    let event = interface::HostEvent::Killed { boss: "ogre".into() };
    assert_eq!(event.name(), "killed");
    assert_eq!(event.args()["boss"], loremaster::runtime::Value::Text("ogre".into()));
    assert_eq!(interface::HostEvent::Found {}.name(), "found");
    assert_eq!(interface::LockpickResult::OpenIt.id(), "open_it");
    assert_eq!(interface::LockpickResult::parse("shut"), Some(interface::LockpickResult::Shut));
    assert_eq!(interface::INTERFACE_HASH.len(), 64);

    let world = Loremaster::new().pack(pack()).build().unwrap();
    assert!(world.story.interface.is_some());
}
