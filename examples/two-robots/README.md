# Two Robots in the Pump Station

A small demo of several player actors. The river is rising, and the player steers two maintenance robots in a flooded pump station: Wick, who has a camera and no ears, and Bilge, who has a microphone and no eyes. Only together can they get the water down. The setting and the text are original.

## Play

```sh
cargo run -p loremaster --no-default-features -- play examples/two-robots
```

You start as Wick. Type `switch to bilge` to take over the other robot (it takes no turn). Give an order to the one you do not steer: `bilge, take spanner` or `bilge, s then turn port wheel`. An order runs one step per turn, and the robot reports when it is done. The lever must stay held for anyone to pass the door, so the other robot must do the rest. The river does not wait: after eight turns it takes the station.

## Test

```sh
cargo run -p loremaster --no-default-features -- test examples/two-robots
cargo test -p loremaster-runtime --test two_robots
```

The Rust test runs the state explorer too: it ends, and it reaches both endings. The water clock multiplies the states by its turns left, so the test raises the budget from 2000 to 20,000; `loremaster explore` with the default budget stops early.

## How it uses the engine

- `playable: [wick, bilge]` in `story.yaml` makes the things player actors; the first starts active.
- `seen_by(wick)` and `seen_by(bilge)` in a room fragment show text per perceiver.
- `wick.active` is true while the player steers Wick, for rules.
- The timer `water` runs from the start. When it runs out, the rule `river_wins` ends the game; its `closes: [drained]` accepts the turns that make the robots too late.
