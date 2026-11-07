# ADR 0001: Save compatibility before 1.0

- Status: accepted
- Date: 2026-11-07
- Decision: D5 (#103)
- Implemented in: P1-04 (#45)
- Requirements: ST-4, ST-5, ST-6

## Context

Until loremaster 0.1, a save game was the bare `State` as JSON. It had no
version and did not say which story it belonged to. A change of the state
format, or a change of the story, could make an old save fail to load, or load
with wrong values.

loremaster 0.x is early access. The engine and the story format still change
often. A strict promise that every old save loads would freeze the state format
too early. No promise at all would make players lose their games after each
update.

## Decision

Best effort. Before 1.0 there is no compatibility promise, but we try:

- A save is an envelope: `format` (`loremaster-save`), `version` (an integer),
  `story` (an id from the title, a hash of the story content, and the timers
  of the story) and `state`.
- A restore migrates an older version one step at a time. Version 0 is the bare
  `State` of loremaster 0.1.
- A restore refuses a save that it cannot read, and never loads it half-way:
  - another format,
  - a newer version (the message names both versions),
  - another story (the message names both ids),
  - broken content.
- When the story hash differs, the restore keeps all state that is still valid
  and reports what it dropped, for example `fact:lamp.lit` or `thing:key`.
  New values start at their start value. A timer that the story did not have
  at the time of the save starts as at the start of a game; the timer list in
  the envelope tells a new timer from one that ran out.

A change of the state format raises `SAVE_VERSION` and adds a migration step
from the version before. A step may give up; then the restore refuses the save.

## Consequences

- `Game::save` and `Game::load_save` replace `Game::state` and `Game::restore`
  for save games. `Game::restore` stays for a state that comes from the same
  world, for example in tests.
- The web editor, the AI tools (`save`, `session_start`) and the MCP server pass
  envelopes. `session_start` and the web player still accept a bare 0.1 state.
- After 1.0, this decision gets a successor with a real promise, for example
  "every save of the same major version loads".
