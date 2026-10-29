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
task dev         # trunk watch (Rust -> WASM) plus the Vite dev server
task verify      # fmt, clippy, tests
```

Without go-task: `cargo run -p lantern-cli -- play examples/lighthouse`. For the web app, run `trunk watch` in `crates/lantern-wasm` and `pnpm dev` in `web/` side by side.

### How Trunk and SvelteKit fit together

Trunk owns the Rust side: `crates/lantern-wasm/Trunk.toml` builds the crate with wasm-bindgen (`web` target, TypeScript declarations, wasm-opt in release) and writes `lantern-wasm.js`, `lantern-wasm_bg.wasm` and `.d.ts` into `web/src/lib/engine/pkg/` without content hashes. `trunk watch` rebuilds when any crate changes. SvelteKit and Vite own the UI and import that package like any module, so a Rust change reaches the browser as a normal Vite reload.

## License

EUPL-1.2, see [LICENSE](LICENSE).
