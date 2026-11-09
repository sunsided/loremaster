//! Save files, which can come from other people. The input is a whole save,
//! or (when it has no `format`) fields that replace those of the `state` of
//! a real save: in the Lighthouse after a few moves, or in the Salt Tower in
//! the middle of a combat. A save that loads plays on and saves again.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_runtime::{Session, World};
use serde_json::Value;
use std::sync::{Arc, OnceLock};

fn base(world: Arc<World>, moves: &[&str]) -> Value {
    let mut game = Session::new(world);
    game.start();
    loremaster_fuzz::play(&mut game, moves.iter().copied());
    serde_json::to_value(game.save()).unwrap()
}

fn lighthouse_save() -> &'static Value {
    static SAVE: OnceLock<Value> = OnceLock::new();
    SAVE.get_or_init(|| {
        base(
            loremaster_fuzz::lighthouse(),
            &["look", "n", "take all", "s"],
        )
    })
}

fn salt_tower_save() -> &'static Value {
    static SAVE: OnceLock<Value> = OnceLock::new();
    SAVE.get_or_init(|| {
        base(
            loremaster_fuzz::salt_tower(),
            &["roll your character", "ring the bell", "refuse"],
        )
    })
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let Ok(input) = serde_json::from_slice::<Value>(rest) else {
        return;
    };
    let (world, base) = if first % 2 == 0 {
        (loremaster_fuzz::lighthouse(), lighthouse_save())
    } else {
        (loremaster_fuzz::salt_tower(), salt_tower_save())
    };
    let save = match input {
        Value::Object(fields) if !fields.contains_key("format") => {
            let mut save = base.clone();
            save["state"].as_object_mut().unwrap().extend(fields);
            save
        }
        whole => whole,
    };
    let Ok((mut game, _)) = Session::load(world.clone(), save) else {
        return;
    };
    game.status();
    game.scene();
    loremaster_fuzz::play(&mut game, ["look", "attack", "i", "n", "wait", "undo"]);
    let again = serde_json::to_value(game.save()).unwrap();
    Session::load(world, again).unwrap();
});
