# The Lock-Keeper slice in world!

The whole golden slice in the `world!` DSL, with no YAML. It has the same ids, texts and rules as `../yaml/`, so it must give the same model (rule 1 of phase 0).

The files are behind the cargo feature `dsl` of the crate `loremaster`; the test target `crates/loremaster-cli/tests/slice_dsl.rs` builds them. `today.rs`, `endings.rs` and `tests.rs` compile since the `world!` core (DSL step L3, P3-03 #66), and `slice_equivalence` proves that they give the model of `../yaml/`. The other files use syntax that `world!` does not have yet (dialogues, text pools, `one_of`, agendas); `../dsl.rs` keeps them out of the build with `#[cfg(any())]` until it does. The syntax follows the DSL doc (`planning/specs/rust-macro-dsl/01-rust-macro-dsl.md`) and the D2 side note of the roadmap. Each place that the doc does not define has a `// draft:` comment.

Each part of the parts table in `../PENDING.md` is one file with one `world!` block. Each block makes one Pack. So a test can load today's pack plus only the parts that it needs.

| File | Part | Holds |
| --- | --- | --- |
| `today.rs` | Today | Story header, vars, rooms, things (not the stove), lock machine, timer, lock and accuse rules, verbs |
| `endings.rs` | Endings | The endings `committed`, `ashes` and `drowned` |
| `texts.rs` | Texts | The text pool `keeper_remark` |
| `dialogue.rs` | Dialogue | The keeper's dialogue |
| `stove.rs` | Stove | The stove and its `one_of` text |
| `keeper_rule.rs` | Keeper rule | The rule `keeper_comments` |
| `agenda.rs` | Agenda | The keeper's agenda step at midnight |
| `tests.rs` | | The walkthroughs of `../yaml/tests/`, as test blocks |
| `../dsl.rs` | | Joins the packs in a fixed order; `with(parts)` and `world()` |

The ids that a file uses from another pack are in an `extern { ... }` block. The macro checks each block at compile time with stubs for these ids, and the check of the joined packs finds them at load time. `today.rs` names the endings in `end committed`, so the joined packs need the endings pack to pass the check, as the YAML form needs `endings/`.
