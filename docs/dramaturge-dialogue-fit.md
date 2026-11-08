# Fit test: dialogues through dramaturge

This report tells how well dramaturge 0.2 fits loremaster beyond conditions. The test case is the cantina example: the three Blackbeard talk rules became `examples/cantina/dialogues/blackbeard.yaml`, with a `trust` relation and an `asked_marigold` memory.

The boundary of the test:

- dramaturge owns facts, characters, relations, memories, dialogue structure and conditions.
- loremaster owns rooms, things, verbs, the parser, the clock, the map, the markup and the editor.
- Space, inventory and time stay out of dramaturge.

## Summary

The fit is good for structure and for state that belongs to characters. The fit is weak where dramaturge must touch the world.

| Area | Fit | Why |
| --- | --- | --- |
| Graph structure, option filtering | Good | `DialogueGraph` and `DefaultRuntime::available_options` work as they are. |
| Keyed ids | Good | `SymbolTable` gives stable, readable ids. Validator messages show the keys. |
| Conditions | Good | `RelationshipAtLeast`/`AtMost` and `HasMemory` lower directly from loremaster conditions. `CustomHandler` answers loremaster's derived values. |
| Relations | Good, with one trap | Untracked relations read as "missing", so `trust <= 0` is false at the start. |
| Memories | Medium | Memories are found by query, not by id. The memory key must also be a tag. |
| Outcomes on the world | Weak | `Outcome::Custom` is untyped. `add` has no outcome. |
| Text | Weak | Two text systems. loremaster keeps its own markup next to the graph. |
| Save state | Medium | Facts, relations and memories are stored twice. |
| Authoring validation | Medium | Useful checks, but issues are text only. loremaster filters them by message text. |

## Where dramaturge fits naturally

1. **The graph.** A node with `outcomes_on_enter` and options with `conditions`, `outcomes` and `target` matches the YAML shape one to one. Lowering is in `crates/loremaster-lang/src/dialogue.rs:59` (`compile_dialogue`).
2. **Option filtering and choice.** The runtime uses `DefaultRuntime::with_parts(DefaultConditionEvaluator::with_custom(DerivedValues), DefaultOutcomeApplier::with_custom(WorldEffects { .. }))` and calls `available_options` and `choose_option` without changes (`crates/loremaster-runtime/src/game/talk.rs:337`, `:364`).
3. **Game as narrative state.** `impl HasDialogWorldState for Game` (`talk.rs:27`) is three lines. Then the `CustomHandler` gets `&Game` and `&mut Game`. The handler can read the clock and the room, and it can apply loremaster effects. This is the best part of 0.2.
4. **Symbols.** `SymbolTable` interns the player, the things and the memories in story order (`crates/loremaster-lang/src/scope.rs:105`), and the node and option keys (`dialogue/node`, `dialogue/node#i`). `GraphValidator::validate_with_symbols` then writes messages such as "Option 'meet#0' in node 'meet' targets nonexistent node 'nmes'".
5. **Relation and memory conditions.** `blackbeard.trust >= 1` lowers to `Condition::RelationshipAtLeast` (`crates/loremaster-lang/src/lower.rs:92`). `remembers(blackbeard, asked_marigold)` lowers to `Condition::HasMemory` (`lower.rs:267`). dramaturge walks `All`/`Any`/`Not`, so these conditions also work inside loremaster markup, for example `{blackbeard.trust >= 1 ? … | …}`.
6. **The validator.** Missing targets, unreachable nodes and always-false conditions are real problems, and `loremaster check` reports them with the node key (`crates/loremaster-project/src/check.rs`, method `dialogues`).

## Where dramaturge fights loremaster

### 1. Two text systems

loremaster text is markup (`{blackbeard.The}`, `{cond ? a | b}`, `*emphasis*`). dramaturge has `DialogText::Fragments` with its own conditions and modes. The two systems do not mix: dramaturge can not render loremaster markup, and loremaster's renderer does not read dramaturge fragments.

What the test does: the graph holds the source as `DialogText::Inline`, and a side table holds the compiled markup (`CompiledDialogue::node_say`, `option_say`, `dialogue.rs:29`). dramaturge never reads the text. The validator checks "possibly empty text" and "unkeyed progress fragments" do not apply.

Request for dramaturge: a host text type, for example `DialogText::Host(Arc<str>)` or a generic text parameter on the graph, so a host can keep its own text without a side table.

### 2. Facts are stored twice

`State.facts` is the save state and the source for the explorer and the room matrix. `DialogWorldState` holds the same facts for dramaturge. `Outcome::SetFact` changes only `DialogWorldState`. After each outcome, `Game::mirror` (`talk.rs:270`) copies facts, relations and memories back into `State`.

The relations and memories need new fields in the save state (`crates/loremaster-runtime/src/state.rs:53-60`) and a rebuild on restore (`restore_dialog_state`, `talk.rs:530`).

`dramaturge-serde` has `SavedWorldState`, which stores keys and survives content changes. loremaster can not use it as the save state: it is not `Eq` and not `Hash` (the explorer compares states), and it holds the facts again.

Request for dramaturge: an outcome observer (a callback with each applied outcome and the new value), or a `NarrativeState` that the host implements fully, without `DialogWorldState`. Then the host can keep one store.

### 3. The explorer and the room matrix do not see relations and memories

The room matrix walks the operands of conditions. It skips relations (`crates/loremaster-runtime/src/matrix.rs:148`): it has no dimension for them. The explorer sees them only because the test added `talk`, `relations` and `memories` to its fingerprint (`crates/loremaster-runtime/src/explore.rs:103`) and lets it choose answers. The inspector (TalkInspector) is the only view that shows relations and memories.

This is loremaster work, not dramaturge work. But it shows the cost of a second state store: every tool that reads state must learn the new store.

### 4. One entry node, but a list of start nodes

`DialogueGraph::entry_node` is one node. loremaster's `start` is a list: the first node whose `when` holds starts the talk. The test sets the entry to the first start node (`dialogue.rs:146`) and picks the start in the runtime (`start_talk`, `talk.rs:184`). The validator then reports the other start nodes as unreachable. `check.rs` drops those issues.

Request for dramaturge: `entry_nodes: Vec<DialogueNodeId>` (or conditional entries), and the validator walks from all of them.

### 5. `Outcome::Custom` is untyped

World effects (`then: [goto: street]`) go through `Outcome::Custom { key: "loremaster.then", args: [Int(index)] }`. The index points into a list of compiled effects next to the graph (`CDialogue::effects`). The handler (`WorldEffects`, `talk.rs:49`) decodes it. A wrong index fails silently. The validator can not see these effects.

`add` has the same problem: dramaturge has `SetFact` but no "add to an integer fact". So `add` is also a custom outcome (`ADD_KEY`, `dialogue.rs:12`).

Requests for dramaturge: `Outcome::AddFact { scope, fact_id, delta }`, and a generic host outcome type (`Outcome<H>` with `Host(H)`), so a host can carry typed data instead of `Vec<FactValue>`.

### 6. The player needs a reserved entity

Relations are `source → target` between entities. The player is not a thing in loremaster, so the test reserves the entity key `@player` (`scope.rs:55`). An id can not start with `@`, so it can not meet a thing id. This works, but every host must invent it.

Request for dramaturge: a documented convention, or a `SymbolTable::player()` helper.

### 7. Things that the test found in addition

1. **A relation that was never set is "missing", not 0.** `RelationshipAtMost` on an untracked pair is false. So `trust <= 0` is false at the start of the game. The test sets every relation of every thing to 0 when the game starts or restores (`talk.rs:545`). Request: a default value per axis, or "missing reads as 0" as an option.
2. **Memories are found by query, not by id.** `MemoryQuery` filters by tag, subject, confidence and facts, but not by `MemoryId`. `remembers(thing, key)` therefore uses the key twice: as the interned `MemoryId` and as `MemoryTag::Custom(key)` (`lower.rs:287`). Request: `MemoryQuery::by_id`.
3. **No "enter node" in the runtime.** `DialogueRuntime` has `available_options` and `choose_option`, but nothing applies `outcomes_on_enter` or checks a node's `conditions`. The host does it (`enter`, `talk.rs:215`; `apply_outcomes`).
4. **No "back".** `DialogTarget::Return` means "return to the caller". The runtime does not keep a stack, so loremaster keeps the nodes before (`Talk::back`) and handles `go: back` itself.
5. **The event log grows.** `choose_option` emits a `DialogOptionChosen` event each time. loremaster does not read the log, so it drains it after each choice (`talk.rs:328`). Without that, a cloned game (the explorer clones many) carries a growing log.
6. **Validator issues are text only.** `ValidationIssue` has a severity, a message and ids, but no kind. To drop the issues that are by design (cycles: talks loop on purpose; other start nodes; dead ends at leaves), `check.rs` matches message text (`check.rs:325-327`). Request: an issue kind enum, and options to switch checks off.
7. **Orphaned facts are noise for a host with rules.** The validator reports facts that conditions read but no dialogue outcome sets. In loremaster, rules and story variables set them. The issue is `Info`, so `check.rs` drops it with all `Info` issues.

## What the test built

- `dialogues/<id>.yaml` with `with`, `start`, `nodes` (`when`, `say`, `set`, `add`, `relation`, `remember`, `then`, `options`), and `go: <node> | end | back`. `relations` in `story.yaml`.
- Conditions `<thing>.<axis>` and `remembers(<thing>, <memory>)`, also in markup.
- `loremaster check` with dramaturge-authoring.
- Play in the CLI and the web: "Talk to" starts the dialogue after the talk rules; numbered answers; the talk stays open until Leave; each answer takes one turn; save and restore keep the node, relations and memories.
- WASM: `choose`, `leave`, `choices`, `talkView`, `dialogues`.
- Editor: talk inspector in play mode, in-place edit of the line and the answers in write mode (Ctrl+E), a Dialogue section with `DialogueOutline` in the room desk. Stories for all new parts.

## Recommendation

Keep dramaturge for dialogue structure, option filtering, relations and memories. The friction is at the edges, and most of it can go away with small changes in dramaturge:

1. A host text type in `DialogText`.
2. An outcome observer, or a host-owned `NarrativeState` without `DialogWorldState`, to end the double store.
3. A list of entry nodes.
4. `Outcome::AddFact` and a typed host outcome.
5. Relation defaults, `MemoryQuery::by_id`, and a validator issue kind.

Without these changes, the cost stays in loremaster: about 900 lines of glue with tests (`talk.rs`, `dialogue.rs`) and a second state store that every state tool must learn.

## Update: dramaturge 0.3 (phase 3)

This section is from 2026-11-07, after loremaster moved to dramaturge 0.3.0-alpha.4. The report above describes dramaturge 0.2 and stays as it was, as the record of the test.

Each point of the report, and what happened to it:

| Point | Status | Where |
| --- | --- | --- |
| 1. Two text systems | Solved. A dialogue line is host text in the graph (`DialogText::Host`, dramaturge R8); the side tables `node_say` and `option_say` are gone. | dramaturge #91; loremaster #150 (P3-08) |
| 2. Facts are stored twice | Solved. loremaster implements dramaturge's `NarrativeState` on its own state (ADR 0002); `DialogWorldState`, `Game::mirror` and the restore rebuild are gone. | loremaster #39 (P1-01, phase 1) |
| 3. The explorer and the room matrix do not see relations and memories | Solved. Both read relations and memories from the one store. | loremaster #43 (P1-02, phase 1) |
| 4. One entry node, but a list of start nodes | Solved. `start` is the graph's entry node list, `select_entry` picks the first that holds, and the unreachable-start filter is gone. | dramaturge #87 (R1); loremaster #150 (P3-09) |
| 5. `Outcome::Custom` is untyped | Solved. `then` is a typed host outcome (`Outcome::Host(Vec<Effect>)`, R3), and `add` is `Outcome::AddFact`. `THEN_KEY`, `ADD_KEY` and the effects index are gone. | dramaturge #86 (R3), alpha.2 (R0e); loremaster #147 (P3-06), #150 (P3-07) |
| 6. The player needs a reserved entity | Solved. loremaster uses `SymbolTable::PLAYER_KEY` and `SymbolTable::player()`; the key `@player` is the same, so saves stay compatible. | alpha.2 (R0c); loremaster #147 (P3-13) |
| 7.1 A relation that was never set is "missing" | Solved. Unset relations read as 0; no start-of-game zero fill. | alpha.2 (R4); loremaster #39 |
| 7.2 Memories are found by query, not by id | Solved. `remembers()` queries `MemoryQuery::by_id`; memories carry no key tag. | alpha.2 (R5); loremaster #147 (P3-11) |
| 7.3 No "enter node" in the runtime | Solved. Nodes are entered with `enter_node`, which checks the node's conditions and applies its enter outcomes. A `go` to a node whose `when` does not hold now ends the talk. | dramaturge #88 (R7); loremaster #150 (P3-12) |
| 7.4 No "back" | Solved. `go` and `go: back` move a `DialogCursor` with a return stack. The save keeps the node keys (`Talk::back`), so old saves load. | dramaturge #88 (R7); loremaster #150 (P3-12) |
| 7.5 The event log grows | Solved. The event log is optional; loremaster does not keep it. | alpha.2 (R0c) |
| 7.6 Validator issues are text only | Solved. The checker filters by `ValidationIssueKind` and skips cycles through `ValidationOptions`. | alpha.2 (R6); loremaster #147 (P3-10) |
| 7.7 Orphaned facts are noise for a host with rules | Unchanged, on purpose. The issue is `Info`, and `check.rs` drops `Info` issues. | — |

In addition, decision D3 made loremaster's YAML dialogue shape the shape of dramaturge 0.3: both read dialogues into one AST in `dramaturge-syntax` (R12). loremaster lowers it through a `Lower` host (`crates/loremaster-lang/src/dialogue.rs`), and dramaturge has a `dialogue!` macro for the same shape (loremaster #153, P3-02; dramaturge #93).

### The glue now

| File | 0.2 | 0.3 |
| --- | --- | --- |
| `crates/loremaster-runtime/src/game/talk.rs` | about 600 lines | 268 lines |
| `crates/loremaster-lang/src/dialogue.rs` | about 335 lines | 236 lines |

Not all of the drop in `talk.rs` is removed code. The logging `NarrativeState` wrapper, the inspector view and the talk input moved into `game/talk/turned.rs`, `view.rs` and `input.rs`. The code that went away is the glue that the table above names: the side tables, the custom outcome keys, the effects index, the double store, the start-node and validator filters, and the host-side enter and back logic.

### Still open

- Auto-continue nodes and lines with their own speaker (R2): played since #155 (phase 4). loremaster does not call `advance`: it shows each node's lines before the node's outcomes and sends a talk event per node, so `talk.rs` has its own loop over `next` with `enter_node`, `DialogCursor::follow` and dramaturge's `ADVANCE_LIMIT`. dramaturge's validator reports a loop of `next` without a choice, and `check` shows it.
- Text pools with selection (R9) are the next dramaturge item (phase 4). The keeper's remark in the golden slice waits for it (`slice_keeper_dialogue`).
