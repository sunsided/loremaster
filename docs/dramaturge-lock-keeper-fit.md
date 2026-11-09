# Fit test: dramaturge on the Lock-Keeper

This is the second fit report (P4-09, #86). The first one, `docs/dramaturge-dialogue-fit.md`, tested dramaturge 0.2 on the cantina and has an update for 0.3 (phase 3). This report tests dramaturge 0.3.0-alpha.5 on the Lock-Keeper (phase 4).

## 1. What was tested, and how

Versions:

- dramaturge `=0.3.0-alpha.5` from crates.io (dramaturge #96, R9; #97, the release).
- loremaster at `b48f52e`, the branch of the demo (#167, not merged when this was written). It has #158 (`world!` dialogues, L4), #164 (`lines` and `next` in talks) and #166 (the text core, P4-01).
- The phase 3 numbers are from `1020419`, the commit that added the 0.3 update to the first report.

What was looked at:

- The demo `examples/lock-keeper-slice/demo/`: nine rooms, four suspects with agendas, a coroner agenda, three quests, five endings, three walkthroughs. `crates/loremaster-runtime/tests/demo.rs` plays the walkthroughs and runs the ending checks over the night.
- The golden slice `examples/lock-keeper-slice/yaml/` (and `dsl/`, `mixed/`), with its feel log in `PENDING.md`.
- The loremaster glue: the talk runtime, the text host types, the pool lowering, the checks and the `world!` dialogue host.
- The PR threads of #96, #97, #158, #164 and #166.

What the demo uses of dramaturge, counted in its files:

| Feature | In the demo | In the slice | Elsewhere |
| --- | --- | --- | --- |
| Dialogues | 4 (`bert`, `crane`, `hester`, `marsh`), 11 nodes, 8 options | 1 (`keeper`) | cantina, tests |
| `start` lists, node `when` | Yes (`bert`, `crane`) | — | — |
| Relations, memories | No | No | cantina |
| Named text pools (`texts/`) | No | `keeper_remark` | `tests/text_pools.rs`, `loremaster-project/tests/texts.rs` |
| `one_of` groups, `pick: best` | 1 (the stove) | 1 (the stove, with `seen_by` and `learn`) | — |
| Speech with `by` | 1 (`rules/accuse.yaml`) | 2 (`texts/keeper_remark.yaml`, `rules/accuse.yaml`) | `tests/speech.rs` |
| `lines` and `next` (R2) | No | No | `tests/talk_flow.rs` only |
| `world!` dialogue | — | `dsl/dialogue.rs`, still behind `cfg(any())` | `loremaster-macros/tests/` |

So the demo is a big test of loremaster (agendas, the ending checks, quests, hints), but a small test of dramaturge. Most of the new dramaturge features are proven by the slice and by tests, not by a story. No example story uses `lines` or `next`. Bert's trust was a stored relation in the design. The demo keeps it inside one talk, to keep the explorer within its budget (`README.md`, "What changed from the design").

## 2. What dramaturge now does for loremaster

| Piece | dramaturge | Where loremaster uses it | Glue |
| --- | --- | --- | --- |
| Text selection | `select`, `select_named`, `commit` (R9) | All texts: room, thing, presence, frame and intro texts are `Pick::All` pools; a `one_of` is an inline pool; `texts/*.yaml` are named pools (`crates/loremaster-runtime/src/text.rs:176`, `:197`, `:253`) | `text.rs` 323, `text/pool.rs` 138 |
| Host types | `TextTypes`, `Judge`, `ProgressStore`, `TextRng` | `Lore`, `LoreJudge`, `impl ProgressStore for State`, `Rng` (`text/pool.rs:19`, `:59`, `:99`, `:130`) | in `text/pool.rs` |
| A node that says a pool | `DialogText::Pool` | `say: { text: <id> }` lowers in 4 lines (`crates/loremaster-lang/src/dialogue.rs:173`); the talk shows it with `say_pool` (`talk.rs:268`) | small |
| Pool checks | `validate_pools`, `validate_pool` (`PickTie`, `UnkeyedProgress`, `UnknownPool`) | `crates/loremaster-project/src/check.rs:480`, `:424` | `check.rs` about 270 lines |
| Lines and auto-continue | `DialogueNode::all_lines`, `next`, `DialogCursor::follow`, `ADVANCE_LIMIT` | `talk.rs:167` (`enter`), `:243` (`show`) | `talk.rs` 339 |
| Dialogue token form | `dramaturge_syntax::tokens::parse_dialogue` with a `ParseHost` | `world!` dialogues (`crates/loremaster-macros/src/parse.rs:1876`) | about 260 lines in `loremaster-macros` |
| Dialogue lowering | `Lower` (from phase 3) | `crates/loremaster-lang/src/dialogue.rs` | 240 |

Three things are loremaster only, with no dramaturge part:

- **Agendas and talks.** An agenda step waits while the player talks with its actor (`crates/loremaster-runtime/src/game/agendas.rs:73`, `talks_with` at `:151`). It reads `State.talk`. dramaturge needs nothing for it, and that is a good fit.
- **Journal and quest text.** A quest's `journal:` line compiles with `Compiler::line` and does not go through the text core. `{ text: <pool> }` in a journal is a check error (`check.rs:199`, since the review of #166).
- **The ending checks and the explorer.** Text progress is not in the explorer's fingerprint (`crates/loremaster-runtime/src/explore.rs:92`), so the text core does not split states. The learned variables do, as facts.

## 3. The glue now and in phase 3

Lines per file, measured with `wc -l` or by function. "Phase 3" is `1020419`, "now" is `b48f52e`.

Code:

| Part | Place | Phase 3 | Now |
| --- | --- | --- | --- |
| Talk runtime | `crates/loremaster-runtime/src/game/talk.rs` | 268 | 339 |
| Talk helpers | `game/talk/turned.rs`, `view.rs`, `input.rs` | 356 | 356 |
| Dialogue lowering | `crates/loremaster-lang/src/dialogue.rs` | 220 | 240 |
| Text host types | `crates/loremaster-runtime/src/text/pool.rs` | — | 138 |
| Text choice, order, commit | `crates/loremaster-runtime/src/text.rs` | 262 (loremaster's own selection) | 323 |
| Pool lowering | `crates/loremaster-runtime/src/world.rs`: `fragments`, `pool`, `pools`, `group_texts` | 26 | 187 |
| Pool and text checks | `crates/loremaster-project/src/check.rs`: the pool functions, `line`, `fragments` | 49 | 273 |
| Dialogue checks | `check.rs`: `dialogues`, `not_played_yet` | 130 | 131 |
| `world!` dialogue host | `crates/loremaster-macros/src/`: `parse.rs` (57), `check.rs` (107), `emit.rs` (99) | — | 263 |
| Total | | 1311 | 2250 |

Tests:

| File | Phase 3 | Now |
| --- | --- | --- |
| `crates/loremaster-lang/src/dialogue/tests.rs` | 121 | 128 |
| `crates/loremaster-runtime/tests/talk_flow.rs` | — | 271 |
| `crates/loremaster-runtime/tests/text_pools.rs` | — | 66 |
| `crates/loremaster-project/tests/texts.rs` | — | 131 |
| `crates/loremaster-macros/tests/ui/fail/dialogue_checks.rs`, `dialogue_syntax.rs` (plus 94 lines of `.stderr`) | — | 155 |
| Total | 121 | 751 |

The first report gives 236 lines for `dialogue.rs` in phase 3. At `1020419` the file has 220. This report uses the measured numbers.

How to read the growth:

- The text core removed loremaster's own selection loop (the old `Pick` and `visible` at `text.rs:162` and `:169` in phase 3). `text.rs` lost 71 lines and gained 132. The modes `once`, `first_visit`, `cycle`, `shuffle` and `sticky` now run in dramaturge.
- Most new lines are not about choosing text. They build pools: `world.rs` lowers the YAML pool AST to `Pool<Lore>` by hand, and `check.rs` does the same again for `Pool<CheckTypes>`. dramaturge's `lower_pool` cannot do it (section 4.1).
- `talk.rs` grew by its own loop over `next` (section 4.2).
- The `world!` host is new: dialogues were not in `world!` in phase 3.

## 4. What fit well, and what did not

### What fit well

1. **The text core replaced loremaster's selection without a visible change.** No transcript changed. The explore output of cantina, zork1 and tower is the same. On lighthouse, one text has 4 variants instead of 3, because `shuffle` no longer repeats the last variant, as decided (#166, "Tests and output").
2. **The host traits are small.** `TextTypes`, `Judge`, `ProgressStore` and `TextRng` take 138 lines in `text/pool.rs`. `ProgressStore` lives on the save state: the keys are dramaturge's (`{pool}/{candidate}`), and old saves migrate from version 2 to 3 (`save.rs`, #166 thread). `TextRng` wraps loremaster's random draw, so a seed gives the same text as before.
3. **`DialogText::Pool` costs 4 lines.** The `Lower` host returns it for `say: { text: <id> }` (`dialogue.rs:173`).
4. **The token form made `world!` dialogues cheap.** The slice dialogue "compiles almost as drafted" (feel log, 2026-11-08, L4). The `ParseHost` is 57 lines. dramaturge's place paths map check errors to tokens.
5. **The validator knows `next`.** A loop of `next` without a choice is a check error from dramaturge's validator, and loremaster reuses `ADVANCE_LIMIT` (`talk.rs:172`).
6. **The core's review was thorough.** Two real bugs were fixed before the merge: a missing named pool had side effects, and a pool reference did not round-trip in YAML (#96, Copilot threads on `select.rs:897` and `shape.rs:968`).

### What did not fit

#### 4.1 `lower_pool` does not take the host's `TextTypes`

`lower_pool` returns `Pool<DialogTypes<T>>` (`dramaturge-syntax/src/lower.rs:589`). The leaf type is the host text, but the learn item, the actor and the topic are dramaturge's. loremaster's leaf also carries the layer, `replaces` and the fragment index, its learn item is a story variable, and its actor is a thing id (`text/pool.rs:19`). So loremaster lowers pools by hand, twice:

- `crates/loremaster-runtime/src/world.rs:425` (`pool`, 50 lines) for play.
- `crates/loremaster-project/src/check.rs:369` (`pool`, 50 lines) for the checks, with a second host type, `CheckTypes` (`check.rs:27`).

Both map `Pick` and `FragmentMode` by hand: 9 `match` sites (`world.rs:375`, `:385`, `:449`, `:463`, `:497`; `check.rs:302`, `:389`, `:402`, `:459`). There are three `Pick` enums: `loremaster_model::Pick`, `dramaturge_syntax::Pick` and `dramaturge_model::Pick`. Each `#[non_exhaustive]` enum needs a fallback arm, and the fallbacks guess (`_ => Pick::First`, `_ => DMode::Always`, `_ => continue`).

#### 4.2 `advance` does not fit a host that shows lines before outcomes

dramaturge's `advance` applies the outcomes of every node in the chain, then returns the list of nodes; the host shows the lines after that (`dramaturge-runtime/src/runtime.rs:67`). loremaster shows each node's lines before its outcomes. It also stops the chain when an outcome ends the story or takes the player away. So `talk.rs:167` has its own loop with `enter_node`, `DialogCursor::follow` and `ADVANCE_LIMIT` (about 45 lines).

The own loop had a bug: `next: back` entered the earlier node again, checked its `when` and applied its outcomes a second time. `go: back` does not do that. The review found it (#164, kilo thread on `talk.rs:186`, fixed in b77e985).

#### 4.3 A named pool inside a pool has no YAML key

In dramaturge's own YAML, a candidate says a named pool with `text: { text: <id> }`. That works only when the text type is dramaturge's `Text` or a host line with a pool form. In `texts/` pools, loremaster's text is a `Line`, which has `{ text: <id> }`, so it works (`world.rs:484`). In a `one_of` group of a description, the text is markup (`Candidate<String>`), and there is no way to name a pool. #96 says so: "Not added: a YAML key for a named pool inside a pool". A key that does not depend on the text type, such as `pool: <id>` on a candidate, would work for every host.

#### 4.4 The `dialogue!` token form cannot name a pool

dramaturge's token form has no pool reference (`dramaturge-syntax/src/tokens.rs` has none; #96: "Not added: … a `dialogue!` token form for pools"). The slice's `dsl/dialogue.rs` says `keeper_remark` in its first node, so it stays behind `cfg(any())` (`examples/lock-keeper-slice/dsl.rs:33`). `the_dsl_dialogue_equals_the_yaml_dialogue` holds an inline copy instead.

#### 4.5 `world!` has no `texts`, `one_of` or agendas

This is loremaster's grammar, not dramaturge's. `texts` and `agenda` are compile errors (`crates/loremaster-macros/src/parse.rs:119`, `:131`). `slice_equivalence_all_parts` stays pending for it (`PENDING.md`). The error for `texts` still says that it waits for the text core (R9, P4-01), which is now done. For `texts`, loremaster needs 4.3 and 4.4 from dramaturge first, or the `world!` form cannot say what the YAML form says.

#### 4.6 `seen_by` has two types and fails silently outside texts

- `Condition::SeenBy` holds a `CharacterId`. The query's perceiver is the host's `Actor` type. `LoreJudge` maps a loremaster id to an entity, and the entity to a `CharacterId`, to compare them (`text/pool.rs:59` to `:83`).
- A `ConditionEvaluator` has no perceiver, so `SeenBy` is `false` there (dramaturge CHANGELOG, alpha.5). A dialogue's `when: seen_by(player)` is always false, with no warning from dramaturge's validator. loremaster's check rejects `seen_by` in rules and other conditions (`check.rs:179`). The kilo review of #166 says that dialogue conditions and inline markup choices still pass the check (thread on `check.rs:179`, no reply yet).

#### 4.7 The trace cost was argued, not measured

`select` builds a `Trace` and a score for each candidate, also for `Pick::All`. The kilo review asked for a path without the trace. The answer was: not before a benchmark shows a cost (#96, thread on `select.rs:852`). Nobody measured it since. loremaster never reads the trace (no use of `Selection::trace` in `crates/`). It selects in play, and also in views: `text::visible` runs a full selection to list the fragments that show (`game.rs:721`, `:995`, `:1072`, `:1108`; `matrix.rs:365`). The explorer plays every state on a clone, so the demo's 60,275 states each render their texts. The cost is real or not; this report does not know.

#### 4.8 Smaller points

- Options and journals cannot say a pool. An option's label goes through `DialogText::as_host` (`talk.rs:115`, `:292`), so `{ text: <pool> }` there is a check error (`check.rs:199`). An option label must be stable before the player chooses, so this may be right; but nothing in dramaturge says so.
- `Pools::insert` returns the pool that it replaces. loremaster drops the result (`world.rs:500`, `check.rs:463`), so two `texts/` files with the same `id:` load without an error (kilo on #166, thread on `load.rs:332`, no reply yet). This is loremaster's bug; a duplicate check in `validate_pools` cannot see it, because the map already lost the pool.
- Who is the player: `talk.rs:260` compares `symbols.entity_key(speaker.0)` with `SymbolTable::PLAYER_KEY`. A `SymbolTable::is_player(entity)` would be one call.
- The inspector does not show a node's extra `lines` (`talk/view.rs` did not change; #164, "Left open").

#### 4.9 The explorer limits (loremaster, not dramaturge)

The design wants 5-minute turns to four o'clock. With them, the explorer did not finish the night in 600,000 states (feel log, 2026-11-08, P4-07). Its fingerprint keeps the rooms visited, the rules fired and the minutes to each agenda step (`explore.rs:92`). The demo takes 15-minute turns, the coroner comes at three, and Bert's trust is not a stored relation. Then the night has 60,275 states; the test sets `max_states: 70_000` and takes about 23 s in debug (#167). The cuts are loremaster's: dramaturge's state (talk, relations, memories) is in the fingerprint, but the relation was cut for the state count, not because dramaturge could not hold it. The feel log proposes time bands or a smaller fingerprint. The smaller fingerprint came with #168 (38,645 states, about 3 s in a release build); time bands were rejected (#239), because they are not exact, and the demo and its design stay at 15-minute turns with the coroner at three.

## 5. Recommendations for dramaturge 0.3.0 final

For dramaturge #77 (D03-REL). Ranked by the glue they remove and the risk they take away. Items 1 to 4 change public API, so they belong before 0.3.0; items 5 and 6 can come in 0.3.x or 0.4.

1. **`lower_pool` for a host `TextTypes`.** Let the `Lower` host (or a second trait) build the leaf, the learn item and the actor, and return `Pool<X>`. Also give `From` conversions between `dramaturge_syntax::{Pick, FragmentMode}` and the model's. Evidence: two hand lowerings of 50 lines each (`world.rs:425`, `check.rs:369`), a second host type for the checks (`check.rs:27`), 9 hand mappings (4.1).
2. **A pool reference that does not depend on the text type**, in YAML (for example `pool: <id>` on a candidate) and in the token form (`dialogue!` and its `ParseHost`). Evidence: 4.3 and 4.4; `dsl/dialogue.rs` behind `cfg(any())`; `slice_equivalence_all_parts` pending; #96 "Not added".
3. **A step-wise `advance`.** For example `advance_with(.., |node| Continue | Stop)`, called for each node after its conditions and before its outcomes, or an iterator of nodes. Then a host can show lines first and stop on its own effects, and `next: back` behaves as in dramaturge. Evidence: the own loop in `talk.rs:167`, and the `next: back` bug in #164 (4.2).
4. **`seen_by` with the host's actor, and a validator issue for `SeenBy` where nothing perceives.** Either `Condition::SeenBy` takes the actor type of `TextTypes`, or `Judge` gets a helper that compares a perceiver with a `CharacterId`. `validate_with_symbols` could report `SeenBy` in graph conditions as always false. Evidence: `text/pool.rs:59` to `:83`; the open kilo thread on #166 (4.6).
5. **Measure the trace before 0.3.0, then decide.** A benchmark of `resolve_text` and `select` with many `Pick::All` fragments, with and without the trace. If the cost shows, add a `select` without a trace for play. Evidence: #96 thread on `select.rs:852`; loremaster never reads the trace and selects in views and in every explored state (4.7).
6. **Small helpers.** `SymbolTable::is_player(entity)` (`talk.rs:260`). A note in the docs on where `DialogText::Pool` may stand (a node's lines, not an option label) (4.8).

On the loremaster side, not for dramaturge: the duplicate pool `id:` check, `seen_by` in dialogue conditions and markup, `texts`, `one_of` and agendas in `world!` (after items 1 and 2), the stale `texts` error message, the inspector's extra lines, and a story that uses `lines`, `next`, relations and a named pool, so that a fit test plays them and not only unit tests do.
