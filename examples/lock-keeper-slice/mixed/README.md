# The Lock-Keeper slice, mixed

The golden slice in two forms at once. The lock machine and the keeper's agenda are in `world!`. Everything else is YAML, copied from `../yaml/`: the rooms and the texts (room and thing descriptions, `texts/`), and also the things, rules, timer, verbs, endings, dialogue and walkthroughs. The ids are the same as in `../yaml/` and `../dsl/`, so all three forms must give one model (rule 1 of phase 0).

The `.rs` files are behind the cargo feature `dsl`. `machines/lock.rs` compiles since the `world!` core (DSL step L3, P3-03 #66); `agendas/keeper.rs` waits for agendas (D6, P4-02 #79), and `../mixed.rs` keeps it out of the build until then. The YAML files load as in `../yaml/`; the loader reads only `*.yaml` files.

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
| `../mixed.rs` | Rust | Joins the YAML folder and the two packs; `today(files)` and `world()` |

The two Rust files name YAML ids in an `extern { ... }` block (open mode). The macro cannot see the YAML: it checks each block with stubs for these ids, and the check of the joined packs finds them at load time.
