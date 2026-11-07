# The Lock-Keeper slice in world!

The whole golden slice in the `world!` DSL, with no YAML. It has the same ids, texts and rules as `../yaml/`, so it must give the same model (rule 1 of phase 0).

This is draft syntax, behind the cargo feature `dsl`. The `world!` macro does not exist yet. The files do not compile until DSL step L2 (phase 1 and 2) for the packs, and steps L3 and L4 for the grammar. Do not add them to a crate before then. The syntax follows the DSL doc (`planning/specs/rust-macro-dsl/01-rust-macro-dsl.md`) and the D2 side note of the roadmap. Each place that the doc does not define has a `// draft:` comment.

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
| `mod.rs` | | Joins the packs in a fixed order; `with(parts)` and `world()` |

The ids that a file uses from another pack are in an `extern { ... }` block. The check finds them at load time. A difference to the YAML form: `today.rs` names the endings in `end committed`, so it needs the endings pack to pass the check. Today's YAML gives `end:` a text and loads without `endings/`.
