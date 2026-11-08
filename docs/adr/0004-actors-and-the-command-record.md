# ADR 0004: Actors, agendas and the Command record

- Status: accepted
- Date: 2026-11-08
- Issue: P4-02 (#79)
- Requirements: SE-2, WO-5, WO-6
- Decision D6 (2026-11-07): the actor model comes in phase 4, together with the agendas, because it changes `State` and `Command` before more hosts depend on them

## Context

Until phase 4 the player was the only one who acted. `State.room` was the
player's room, and `Command::Verb` was a verb with a list of objects. A rule
matched `verb: [verb, thing, other]`. The golden slice needs a keeper who
feeds the stove at midnight, and the rule `on: { verb: examine, direct: stove }`
names a slot of the command.

## Decision

- **Actors.** The player is the active actor. Each thing with an agenda is an
  actor too. An actor's place is its location; there is no second store. The
  player keeps `State.room`. Switching the active actor (the rest of WO-5) and
  orders (WO-14) come with the two-robot demo; until then, a command with an
  `actor` other than `player` is refused.
- **The Command record** has six slots: actor, verb, direct, indirect,
  preposition, topic. On the wire, `Command::Verb` keeps `objects` (direct,
  then indirect) and gets optional `actor`, `preposition` and `topic`, so
  hosts send the same JSON as before. `CommandRecord` is the resolved form,
  with the actor always set (`player` for the player).
- **Event patterns.** `verb` and `check` take a `CommandPattern`: the list
  form `[verb, direct, indirect]`, or the record form with named slots. A
  pattern that has only a verb and objects is written in the list form, so
  models and their hashes do not change. A pattern without `actor` matches
  only the player: the many rules that stories have for the player do not
  fire when a character acts.
- **Agendas.** One agenda per actor, in `agendas/`. Steps run in order. A
  step's due time is fixed by the story: the first time the clock reaches its
  `at`, at or after the time the step before was due (or the start). A timed
  step whose `when` does not hold when it is due is skipped (an appointment
  that did not happen); a step without `at` waits for its `when`. A due step
  walks the actor to its `place`, one room per turn along the exits, then
  runs its `do` command and its `then` effects. An actor does one thing per
  turn, after the clock, the timers and the time rules. A talk with the actor
  holds its step back, so the step runs late.
- **Commands of actors** fire `check`, `instead` and `verb` rules with the
  actor in the record. The default actions of the standard verbs stay the
  player's: they move things into the player's hands and describe the
  player's room. A refused command skips its step but takes the actor's
  turn, as a walk does: the actor tried. A step skipped for its `when`
  takes no time.
- **`closes`** is plain data on a step: a list of ending ids and a `reason`.
  The ending checks (P4-04) compare it with the endings that really drop out.
- **The save** keeps, for each actor, the steps that are over (`ran` or
  `skipped`) by step key. A step key is the step's `id`, or its place from 1.
  This is save version 2; see ADR 0001.

## Consequences

- `EventPattern::Verb` and `Check` hold a `CommandPattern`, and `Command::Verb`
  and `Event::Verb` have new fields: a breaking change for library users.
- The explorer offers `wait` in a story with agendas. The actors and the
  minutes until each agenda's next step are part of its state fingerprint,
  so waiting reaches a step in the middle of a phase. The ending checks
  (P4-04) read an agenda step's `closes` as a rule's, as the closer
  `agenda:<id>/steps[<i>]`.
- A step key from the position in the list moves when an author inserts a
  step before it; an `id` keeps it stable for saves.
- `world!` has no agenda syntax yet. Its form should give the same `Agenda`
  values as YAML; the drafts are in `examples/lock-keeper-slice/dsl/agenda.rs`
  and `mixed/agendas/keeper.rs`.
