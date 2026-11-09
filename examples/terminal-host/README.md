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
2. Show each `Effect` of the `Step`, in order:
   - `Text`: a block of runs. A run in speech names its `speaker`. The
     block's `cause` says why it showed: `Command` for the player's input
     (its response, the rules it fired, the room after a move), `World` for
     the opening and the turn passing (`on: turn`, timers, time rules,
     agendas). A stage host can draw the first over the player and the
     second as ambient narration. `Run::source` says who wrote the text, not
     why it showed.
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
4. A move as a command record is `Command::Go { direction }`; `Command::verb("go", vec![dir])`
   is the same move.
5. `scene()` covers the player's room and the inventory. `thing(id)` gives
   any thing of the story with its current name and location, for a host
   that shows more than the player's room.
6. `Effect`, `Status` and `AwaitRequest` are `non_exhaustive`. Keep a fallback arm.
7. Save with `Session::save`, and load with `Session::load`. After a load, set
   `set_host_rolls` again: it belongs to the session, not to the save. `load`
   returns no step, so restore the session's own `snapshot` to show the room.
   `snapshot` and `restore` also keep a state in memory.
