# The Lock-Keeper slice: pending tests

The golden slice of the roadmap (phase 0). It is a small part of the demo "The Lock-Keeper's Night", written before the engine can run all of it. Each later phase turns some of its tests green. The design is in the roadmap and spec doc, section "Phase 0: golden slice".

## Contents

| Folder | Contents | Status |
| --- | --- | --- |
| `yaml/` | The whole slice in YAML | This folder |
| `dsl/` | The same slice in `world!`, behind the cargo feature `dsl` | P0-02 (#34) |
| `mixed/` | Rooms and texts in YAML, the agenda and the lock machine in `world!` | P0-03 (#35) |
| `expected/` | `model.hash` and `slice_best.transcript` | When `slice_equivalence` (P1-06) and `slice_best` (P4-07) pass |

The slice: three rooms (kitchen, keeper's room, lock chamber), the lock machine with its timer, the stove with the text of the D2 side note, the keeper with one dialogue and one agenda step (he burns the pages at midnight), and three endings (a win, a loss and a death).

## Which file needs which phase

Files in today's syntax load now. The other files use syntax that the model does not know yet. Today's loader rejects them (`things/stove.yaml`, `rules/keeper.yaml`, `dialogues/`) or ignores their folder (`texts/`, `agendas/`, `endings/`).

| Part | Files | New syntax | Phase |
| --- | --- | --- | --- |
| Today | `story.yaml`, `rooms/`, `things/` (not the stove), `machines/`, `timers/`, `rules/lock.yaml`, `rules/accuse.yaml`, `verbs/` | None | Now |
| Endings | `endings/` | Ending entities with `kind`, `rank`, `title`; `end: <ending id>` | 1 (P1-03) |
| Dialogue | `dialogues/keeper.yaml` | A node says a text pool: `say: { text: keeper_remark }` | 3 (D3) |
| Texts | `texts/keeper_remark.yaml` | Named pool with `one_of`, `pick: best`, `mode: sticky` | 4 (R9) |
| Stove | `things/stove.yaml` | `one_of`, `pick: best`, `seen_by(player)`, `learn` | 4 (R9) |
| Keeper rule | `rules/keeper.yaml` | `on: { verb: examine, direct: stove }` (Command record), `say: { text: … }` | 4 (D6, R9) |
| Agenda | `agendas/keeper.yaml` | `actor`, `steps` with `at`, `place`, `when`, `then`, `allow_unwinnable` | 4 (D6, CK) |

## Pending tests

The Rust tests are in `crates/loremaster-runtime/tests/slice.rs`. Each one loads today's files plus only the parts that it needs. So a feature turns its own test green, even while other parts do not load yet. The walkthroughs are in `yaml/tests/` and have the same `pending:` mark.

| Test | It proves | Loads | Passes after | Issue |
| --- | --- | --- | --- | --- |
| `slice_baseline` | Rooms, things and the lock machine load and play in today's syntax | Today | Phase 0 (passes now) | #37 |
| `slice_endings` | Endings are entities with id, kind and rank | + Endings | Phase 1 | P1-03 #44 |
| `slice_save_envelope` | A save from the middle of `slice_best` has an envelope and restores | + Endings | Phase 1 | P1-04 #45 |
| `slice_equivalence` | YAML, DSL and mixed give one model | Today, per form | Phase 1 and DSL step L2 | P1-06 #47 |
| `slice_undo_after_death` | The death prompt, and undo after it | + Endings | Phase 2 | P2-03 #54 |
| `slice_keeper_dialogue` | One dialogue format with a text pool | + Dialogue, Texts | Phase 3 (and R9, see the log) | P3-01 #64 |
| `slice_stove_text` | `one_of`, `pick: best`, `seen_by` and `learn` | + Stove, Agenda | Phase 4 | P4-01 #78 |
| `slice_keeper_agenda` | The keeper feeds the stove at midnight | + Agenda | Phase 4 | P4-02 #79 |
| `slice_talk_pauses_agenda` | A talk at midnight delays the agenda step | + Agenda, Dialogue, Texts | Phase 4 | P4-02 #79 |
| `slice_winnable` | The agenda step is an unwinnable edge, and `allow_unwinnable` accepts it | All | Phase 4 | P4-04 #81 |
| `slice_best`, `slice_ashes` | The two full walkthroughs | All | Phase 4 | P4-07 #84 |

Run the pending tests with `task test:pending`. When a test passes, remove its `#[ignore]` mark and the `pending:` line of its walkthrough in the same pull request.

## Feel log

What felt bad to write, and what to change. Rule 5 of phase 0: change the slice first, the engine after.

| Date | Form | What felt bad | Proposal |
| --- | --- | --- | --- |
| 2026-11-07 | YAML | No plural names: "a ledger pages". The thing is now an "oilcloth packet" with the alias `pages`. | A `plural: true` field (articles "some", "the"). |
| 2026-11-07 | YAML | Today's loader ignores unknown folders. A typo such as `ending/` loads without an error. | Reject unknown folders, as unknown fields are rejected. |
| 2026-11-07 | YAML | The stove text of D2 sits in the kitchen description. It moved to the stove, so that the kitchen loads today. | None needed; both places should work after R9. |
| 2026-11-07 | YAML | Walkthroughs to midnight need 17 lines of `wait`. | A step form `{ wait: 17 }` or `{ until: "00:00" }`. |
| 2026-11-07 | YAML | `end: committed` reuses the field that takes a text today. | P1-03: `end:` takes an ending id; an ending without a file is an error. |
| 2026-11-07 | YAML | `expect:` ignores unknown keys, so `expect: { ending: … }` would pass silently. The walkthroughs use `facts: { ending: …, ending.kind: …, ending.rank: … }` instead. | Reject unknown keys in `expect:`; P1-03 decides how a walkthrough names the ending. |
| 2026-11-07 | YAML | `seen_by(inspector)` in D2 needs a name for the player. The slice uses `seen_by(player)`. | Decide the perceiver id with D6. |
| 2026-11-07 | YAML | `slice_keeper_dialogue` is phase 3 in the spec, but its dialogue says a text pool, which is R9 (phase 4). | Either an inline `say` in phase 3, or the test moves to phase 4. |
| 2026-11-07 | YAML | The roadmap wants Markus to write one part in each form and note how it felt. | Open: Markus. |
