# The Salt Tower

A small gamebook in 9 numbered sections. You are a courier with a sealed letter for the keeper of a tower on the salt flats. The text is original.

Each section is a room, and each choice is an exit with a label. Two more rooms carry the ends: `lamp_room` (the win) and `fallen` (0 STAMINA).

## Play

```sh
cargo run -p loremaster --no-default-features -- play examples/salt-tower
```

Type the label of an exit, for example `ring the bell`. Type `take brass lantern` in section 6.

## Test

```sh
cargo run -p loremaster --no-default-features -- test examples/salt-tower
cargo test -p loremaster-runtime --test salt_tower
```

The walkthroughs are in `tests/`. `pay` uses a seed. `fight`, `grate` and `spent` force the rolls.
The Rust test also runs the state explorer: all 9 sections are reachable, and no state is stuck.

## How it uses the engine

- Choices are exits with a name. A rule `on: { check: [go, "pay 3 gold"] }` runs the cost of a choice.
- The Luck test and the Skill test are short machines of the same shape in `machines/`. A machine has no parameters, so each test is its own machine.
- The dice in section 1 run on a command, so a walkthrough can force them.
- Combat is the built-in `combat` effect in section 5.
