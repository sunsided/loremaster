# lantern

A world editor for text adventures, where every piece of text is a function of world state. Rust core (built on [dramaturge](https://github.com/sunsided/dramaturge)), SvelteKit front end, parser and SCUMM-style verb-bar play over one runtime.

## Layout

| Path | What |
| --- | --- |
| `crates/lantern-model` | World types: rooms, exits, things, fragments, rules, clock, verbs |
| `crates/lantern-lang` | Condition expressions and inline markup, lowered to dramaturge conditions |
| `crates/lantern-project` | Project folder format (one YAML file per entity) and checker |
| `crates/lantern-runtime` | Pure, deterministic runtime: state, text assembly, rules, parser |
| `crates/lantern-wasm` | WASM bindings used by the web app |
| `crates/lantern-cli` | `lantern play` / `lantern check` |
| `examples/lighthouse` | A small story used by tests, the CLI and the web app |
| `web/` | SvelteKit app: player with verb bar, assembly panel, themes and effects |

## Running

```sh
task play        # play the example in the terminal
task web:install # once
task dev         # build WASM and start the web dev server
task verify      # fmt, clippy, tests
```

Without go-task: `cargo run -p lantern-cli -- play examples/lighthouse`; for the web app run the `wasm-pack` command from `Taskfile.dist.yaml`, then `pnpm dev` in `web/`.
