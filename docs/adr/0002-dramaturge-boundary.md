# ADR 0002: The boundary between loremaster and dramaturge

- Status: accepted
- Date: 2026-11-07
- Issue: P1-08 (#49)
- Requirements: NF-3, ST-1 to ST-7
- Builds on: #31 (one state store), decision D2 (text selection)

## Context

loremaster plays a world: rooms, things, verbs, the parser, the clock,
rules, timers, machines and Rust behaviors. dramaturge is a narrative
library: dialogue graphs, conditions and outcomes over facts, relations and
memories, and (from D2) the text selection core.

Until #31, both kept their own state. dramaturge held a `DialogWorldState`,
loremaster held `State`, and `Game::mirror` copied each change from one to
the other. Each tool that reads the state (save games, the explorer, the
room matrix, walkthroughs) had to know both stores, and each copy was a place
for a bug.

## Decision

**dramaturge owns narrative logic. loremaster owns the store and the world.**

- There is one `State`: world and narrative, in one save envelope, with one
  RNG (ADR 0001). dramaturge holds no play state while a loremaster session
  runs.
- dramaturge reads and writes the host's store only through its
  `NarrativeState` trait. loremaster implements the trait on `Game`, backed by
  `State` (`crates/loremaster-runtime/src/game/narrative.rs`): facts are
  `State::facts`, relations are `State::relations` (key `thing.axis`),
  memories are `State::memories`.
  `DialogWorldState` stays in dramaturge as the default store for users of
  dramaturge alone.
- dramaturge owns the shape of narrative state (facts, relations, memories,
  alignments), dialogue graphs, conditions, outcomes, validation, and the text
  selection core (`one_of`, `pick`, perceiver, `learn`; D2), generic over a
  host text leaf.
- loremaster owns everything that is world: rooms, things, the parser,
  verbs, the clock, rules and their phases, timers, machines, agendas,
  behaviors, endings, and the assembly of text from fragments (markup,
  layers, room assembly). loremaster markup is a host text leaf for
  dramaturge.
- dramaturge stays below the Session API. A host never calls dramaturge
  directly while a session runs; it calls the session (ADR 0003).
- What dramaturge cannot express yet goes through its extension points
  (`CustomHandler` conditions and outcomes), not through a second store. When
  an extension point is a workaround, it gets a dramaturge roadmap item
  (R0e `AddFact`, R3 typed host outcome, R8 host text leaf) and goes away when
  that item lands.
- A loremaster change that needs dramaturge goes to dramaturge first, on the
  0.3 branch, and comes to loremaster through an alpha release.

## Consequences

- The save game, the explorer, the room matrix and the walkthroughs read one
  store. A new narrative value needs one field in `State`, not two.
- `NarrativeState` is a public contract between the crates. A change to it is
  a breaking dramaturge change and waits for a minor release.
- The turn log of a talk comes from a wrapper state that forwards to `Game`
  and logs each change (P1-01b, #40), not from a copy of the outcomes.
- Features that cross the boundary (ask and show, perceiver text) live in
  dramaturge, so that other hosts can use them too.
