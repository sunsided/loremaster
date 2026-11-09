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
   version is checked and the hint ladder is set. A story with Rust behaviors
   uses `session_with(|world| { world.register(..); })`.
2. Show each `Effect` of the `Step`, in order:
   - `Text`: a block of runs. A run in speech names its `speaker`. The
     block's `cause` says why it showed: `Command` for the player's input
     (its response, the rules it fired, the room after a move), `World` for
     the opening and the turn passing (`on: turn`, timers, time rules,
     agendas). A stage host can draw the first over the player and the
     second as ambient narration. `Run::source` says who wrote the text, not
     why it showed. A run with `emphasis` is `*emphasis*` in the story; its
     text has no stars. `Block::plain_text` puts the stars back. The answers
     of a talk (`Choice::text`) are plain strings and keep their stars.
   - `Notice`: a change of state. A host may skip it.
   - `Choices`: the answers of a talk. Answer with `Input::Choice(index)`.
   - `Roll`: dice to show.
   - `Sound`: a one-shot sound (`play_sound`), with the asset's `file` and
     `caption`. This host plays no audio and prints the caption. A sound
     line of an NPC is such a sound: float the caption beside the NPC, and
     do not look for a text prefix such as `sfx:` in `say` text.
   - `Ending`: the story has ended.
3. Act on the `Status` of the step. Only the status says what may come next:
   - `Ready`: send the next `Input`.
   - `Await`: call `resume(id, value)` with an answer to the `AwaitRequest`.
     - `Choice`, `Action`, `WalkTo`: `answers()` lists the values, if there is a list.
     - `Roll`: an integer. It comes up only with `set_host_rolls(true)` (`--own-dice` here).
     - `DeathPrompt`: `undo` and `restart` go to `resume`. For `restore`, the host loads a save.
   - `Ended`: stop. `ending` is `None` for a save restored into a changed story.
4. A move as a command record is `Command::Go { direction }`; `Command::verb("go", vec![dir])`
   with a compass word or an exit of the room is the same move. `go` with a
   thing stays a verb.
5. `scene()` covers the player's room and the inventory. `thing(id)` gives
   any thing of the story with its current name and location, for a host
   that shows more than the player's room. Like `snapshot`, it sees
   everything (concealed things, closed containers); the host decides what
   to show.
6. Read the text by its runs, not by its words:
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
   - `verbs_for(id)` lists the verbs that apply to a thing in scope now,
     sorted by id, for a menu on a click: each verb whose rule for the
     player (`verb` or `check`) names the thing as an object or that has a
     frame on it, and each standard verb whose default does something for
     it in this state (`examine` always, `take` when portable, not held,
     not in a closed container and the hands are not full, `drop` and `put`
     when carried, `open` when closed and unlocked, `close` when open,
     `unlock`/`lock` for a lock and its key while both are in scope,
     `turn_on`/`turn_off` for a thing with `lit`, `use`, `talk` when a
     start node of its dialogue applies, `search` when it hides
     something). Rule conditions are not checked.
   - `ThingInfo::aliases` lists a thing's aliases and the names of its
     earlier name stages.
7. Queries take no step: `scene()` (the room, the time as `time` and as
   `minutes`, the things, the exits), `thing(id)` (any thing), `fact(name)` (one variable, property
   or machine state, without a copy of the state), `verbs()`, `verbs_for(id)`, `journal()`,
   `stats()`, `media()`, `choices()`, `combat()`.
8. `Effect`, `Status` and `AwaitRequest` are `non_exhaustive`. Keep a fallback arm.
9. Save with `Session::save`, and load with `Session::load`. After a load, set
   `set_host_rolls` again: it belongs to the session, not to the save. `load`
   returns no step, so restore the session's own `snapshot` to show the room.
   Then call `Session::reseed` with a fresh, unpredictable number: for a story with
   `reload_rolls: fresh` the next roll is new; for `same` it changes nothing.
   `snapshot` and `restore` also keep a state in memory.
