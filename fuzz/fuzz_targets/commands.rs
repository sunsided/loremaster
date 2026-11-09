//! Player commands: each line of the input is one command in the Lighthouse
//! (even input) or the Salt Tower (odd input), with undo and a save and load
//! at the end.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_runtime::Session;

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let world = if first % 2 == 0 {
        loremaster_fuzz::lighthouse()
    } else {
        loremaster_fuzz::salt_tower()
    };
    let mut game = Session::new(world.clone());
    game.start();
    loremaster_fuzz::play(&mut game, text.lines().take(64));
    let save = serde_json::to_value(game.save()).unwrap();
    let (again, _) = Session::load(world, save).unwrap();
    assert_eq!(again.state(), game.state());
});
