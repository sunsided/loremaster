//! `loremaster play` in the terminal (#222): a sound shows as its caption.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn the_terminal_player_prints_the_caption_of_a_sound() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/gatehouse");
    let mut child = Command::new(env!("CARGO_BIN_EXE_loremaster"))
        .arg("play")
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    // The end of the input ends the game.
    (child.stdin.take().unwrap())
        .write_all(b"pull rope\npull rope\nopen gate\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("[The gate creaks on its hinges.]"), "{text}");
}
