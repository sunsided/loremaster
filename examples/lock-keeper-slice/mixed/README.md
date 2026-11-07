# The Lock-Keeper slice, mixed

The golden slice in two forms at once. The lock machine and the keeper's agenda are in `world!`. Everything else is YAML, copied from `../yaml/`: the rooms and the texts (room and thing descriptions, `texts/`), and also the things, rules, timer, verbs, endings, dialogue and walkthroughs. The ids are the same as in `../yaml/` and `../dsl/`, so all three forms must give one model (rule 1 of phase 0).

The `.rs` files are draft syntax, behind the cargo feature `dsl`. The `world!` macro does not exist yet. They do not compile until DSL step L2 (phase 1 and 2) for the packs, and steps L3 and L4 for the grammar. The YAML files load as in `../yaml/`; the loader reads only `*.yaml` files.

| File | Form | Part |
| --- | --- | --- |
| `story.yaml`, `rooms/`, `things/` (not the stove), `timers/`, `rules/lock.yaml`, `rules/accuse.yaml`, `verbs/` | YAML | Today |
| `machines/lock.rs` | `world!` | Today: the lock machine, in place of `machines/lock.yaml` |
| `endings/` | YAML | Endings |
| `dialogues/keeper.yaml` | YAML | Dialogue |
| `texts/keeper_remark.yaml` | YAML | Texts |
| `things/stove.yaml` | YAML | Stove |
| `rules/keeper.yaml` | YAML | Keeper rule |
| `agendas/keeper.rs` | `world!` | Agenda, in place of `agendas/keeper.yaml` |
| `tests/` | YAML | The walkthroughs |
| `mod.rs` | Rust | Joins the YAML folder and the two packs; `today()` and `world()` |

The two Rust files name YAML ids in an `extern { ... }` block (open mode). The macro cannot see the YAML, so the check finds these ids at load time.
