# Terminal host

An example host of the Session API. It plays a story in a terminal through
`loremaster_runtime::Session` only: it uses no game internals and no authoring
module. A game engine, a web page and this host do the same work.

```sh
cargo run -p loremaster-terminal-host -- examples/lighthouse
cargo run -p loremaster-terminal-host -- examples/salt-tower --own-dice
```

Lines that start with `:` are the host's: `:save <file>`, `:load <file>`, `:quit`.

## What a host must do

1. Load a project, compile a `World`, make a `Session` and call `start`.
   A host that ships the story in its binary writes a `Bundle` in its
   `build.rs` (`Bundle::new(project.story, project.walkthroughs).to_json()`),
   and at run time calls `Bundle::from_json(..)?.session()`: the format
   version is checked and the hint ladder is set.
2. Show each `Effect` of the `Step`, in order:
   - `Text`: a block of runs. A run in speech names its `speaker`. A run
     with `emphasis` is `*emphasis*` in the story; its text has no stars.
     `Block::plain_text` puts the stars back. The answers of a talk
     (`Choice::text`) are plain strings and keep their stars.
   - `Notice`: a change of state. A host may skip it.
   - `Choices`: the answers of a talk. Answer with `Input::Choice(index)`.
   - `Roll`: dice to show.
   - `Ending`: the story has ended.
3. Act on the `Status` of the step. Only the status says what may come next:
   - `Ready`: send the next `Input`.
   - `Await`: call `resume(id, value)` with an answer to the `AwaitRequest`.
     - `Choice`, `Action`, `WalkTo`: `answers()` lists the values, if there is a list.
     - `Roll`: an integer. It comes up only with `set_host_rolls(true)` (`--own-dice` here).
     - `DeathPrompt`: `undo` and `restart` go to `resume`. For `restore`, the host loads a save.
   - `Ended`: stop. `ending` is `None` for a save restored into a changed story.
4. `Effect`, `Status` and `AwaitRequest` are `non_exhaustive`. Keep a fallback arm.
5. Save with `Session::save`, and load with `Session::load`. After a load, set
   `set_host_rolls` again: it belongs to the session, not to the save. `load`
   returns no step, so restore the session's own `snapshot` to show the room.
   Then call `Session::reseed` with a fresh, unpredictable number: for a story with
   `reload_rolls: fresh` the next roll is new; for `same` it changes nothing.
   `snapshot` and `restore` also keep a state in memory.
