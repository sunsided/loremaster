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
   - `Text`: a block of runs. A run in speech names its `speaker`.
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
4. Read the text by its runs, not by its words:
   - A run with a `Thing` link is a mention of a thing in scope, in any
     block but the command echo, the title and the exits: room prose too, and
     the room after `undo` and `restore`. The engine links each name and
     alias apart, so "Hester Reed, Amos's daughter" has two links, "Hester
     Reed" and "daughter". A host that marks words itself would miss the
     aliases and the name stages.
   - A word with a link is a noun, also when a verb has the same word
     ("lock"). The engine links no verbs.
   - `VerbInfo::slots` says how many objects a verb takes (`none`, `one`,
     `two`), not which things it applies to: any verb with a slot takes any
     thing in scope, and a rule or the default response answers. A verb bar
     offers each `bar` verb; `preview(text)` says what a half-typed command
     refers to.
   - `ThingInfo::aliases` lists the other words the parser accepts for a
     thing now.
5. Queries take no step: `scene()` (the room, the time as `time` and as
   `minutes`, the things, the exits), `fact(name)` (one variable, property
   or machine state, without a copy of the state), `verbs()`, `journal()`,
   `stats()`, `media()`, `choices()`, `combat()`.
6. `Effect`, `Status` and `AwaitRequest` are `non_exhaustive`. Keep a fallback arm.
7. Save with `Session::save`, and load with `Session::load`. After a load, set
   `set_host_rolls` again: it belongs to the session, not to the save. `load`
   returns no step, so restore the session's own `snapshot` to show the room.
   `snapshot` and `restore` also keep a state in memory.
