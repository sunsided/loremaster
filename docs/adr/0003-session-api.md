# ADR 0003: The shape of the Session API

- Status: accepted
- Date: 2026-11-07
- Issue: P1-09 (#50)
- Requirements: NF-3, SE-1 to SE-15, XP-1 to XP-4, XP-7
- Builds on: decision D1 (Session API with the golden slice), ADR 0002
- Decision D4 (#102, 2026-11-07): `loremaster-runtime` exports `Session`; a new crate `loremaster-workbench` holds the authoring views

## Context

Today each front end talks to `Game` in its own way. The web player calls the
wasm engine, the terminal calls `Game::input`, the AI tools keep `Game`s in a
map, and the walkthrough runner plays `Game` directly. `Game` returns `Turn`s
with text blocks, and a rule runs to its end in one call. That is not enough
for the hosts of the roadmap:

- A gamebook roll, a choice inside a rule, a walk in a stage host, and the
  death prompt must stop a rule, ask the host, and go on later (SE-5, SE-6).
- Game engines (Path B) need data, not text with markup, and must not get a
  private path into the core (SE-4, SE-11).
- A save in the middle of such a pause must restore, and play must replay the
  same way on native and WASM builds, because the walkthroughs run in CI and in
  the browser (SE-10).

## Decision

**One `Session` per running game. A host sends input down and gets effects
and awaits back. Everything is data.**

1. **Input.** `submit(input)` is the one entry point for input. The input is
   one of: a text command, a `Command` record (actor, verb, direct, indirect,
   preposition, topic), a choice id, or a tick (SE-2). `choose(id)` is a short
   form of `submit` with a choice id. `resume(await_id, value)` is separate: it
   answers an await request, not an input.
2. **Step result.** Each call returns a list of effects and one status: ready
   for input, an await request, or the ending (SE-3).
3. **Effects as data.** Kinds: text block (kind, runs, sources, links), state
   notice, media slot change, sound, music state, stage direction, ending.
   Effects carry no HTML, CSS or pixel positions (SE-4). Behaviors return
   effects too; the session applies, logs and undoes them like rule effects
   (XP-2).
4. **Awaits.** A request names what it waits for: a choice, a roll, a custom
   action (name, arguments, possible results), a walk to a place, or the death
   prompt (SE-5). The session answers a roll from its own RNG in the same step,
   unless the host asks to answer rolls itself (SE-7).
5. **A cursor, not `async`.** A rule that awaits stops at an effect. Its
   position (rule id, effect index, local variables) is data in `State`
   (SE-6, XP-3). Rust `async` frames cannot go into a save; machines already
   wait between states, so the cursor is the same idea inside one rule. A
   behavior cannot pause itself; it returns an await effect, and the rule
   continues later.
6. **Snapshots.** `snapshot` and `restore` use the save envelope (ADR 0001).
   `undo` restores the state before the last input; the host sets the undo
   depth (SE-8). A death ending sends the death prompt (restore, undo,
   restart) unless the story turns it off (SE-9).
7. **Determinism.** The same world, seed, inputs and resume values give the
   same effects on every build. The core reads no clock and no host state
   (SE-10). Behaviors get random numbers only through `Cx` (XP-7).
8. **Read-only queries** (the room, visible things, exits, verbs, the talk) are
   part of the Session API. Authoring views (atlas, rules, machines, explorer,
   matrix, checks, proposals) are not: they are the Workbench API, which reads
   the full `World` and `State` but changes the world only through files or
   packs (SE-13).
9. **Dispatch order** is part of the documented contract: phases, then scope
   (thing, room, region, story), then priority, then file order (SE-12).
10. **Stability.** The Session API follows semver (SE-11). The web player, the
    walkthrough runner, the AI tools and every other host use only the Session
    and Workbench APIs.

## Consequences

- `Game` becomes the engine behind `Session`; hosts stop calling it directly
  (P2-09, P2-10). The walkthrough runner is a host like any other.
- `State` grows a cursor for paused rules, and the save envelope keeps it, so
  a save in the middle of a rule restores (golden slice:
  `slice_undo_after_death`, `slice_talk_pauses_agenda`).
- Effects need stable data types with a version, because hosts outside the
  repository read them (JSON-RPC protocol mode, SE-15).
- D4 (#102) decided where the types live: `loremaster-runtime` exports
  `Session`, so that `Game` can become `pub(crate)` step by step and the
  compiler keeps hosts on the Session API; a new `loremaster-workbench` crate
  holds the authoring views. A separate session crate would need `Game` and
  `State` to stay public to reach them, which makes the boundary weaker.
