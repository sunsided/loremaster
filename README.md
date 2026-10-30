# lantern

lantern is a world editor for text adventures. In lantern, each text in the game comes from the world state. The author can see which text shows, when it shows and why.

The core is in Rust. It uses [dramaturge](https://github.com/sunsided/dramaturge). The editor is a SvelteKit app. The player can play with typed commands or with a SCUMM-style verb bar. Both use the same runtime.

## Layout

| Path | Contents |
| --- | --- |
| `crates/lantern-model` | World types: rooms, exits, things, fragments, rules, clock and verbs. |
| `crates/lantern-lang` | The condition language and the inline markup. They compile to dramaturge conditions. |
| `crates/lantern-project` | The project folder format (one YAML file for each entity) and the checker. |
| `crates/lantern-runtime` | The runtime: state, text assembly, rules and parser. It has no input or output of its own. |
| `crates/lantern-wasm` | The WebAssembly bindings for the web app. Trunk builds them. |
| `crates/lantern-cli` | The terminal player: `lantern play` and `lantern check`. Also `lantern mcp`. |
| `crates/lantern-tools` | The operations that an AI assistant can do on a project: read, play and verify. It has no input or output of its own. |
| `crates/lantern-mcp` | The MCP server for one project: `lantern mcp <project-folder>`. |
| `examples/lighthouse` | A small story. The tests, the terminal player and the web app use it. |
| `examples/cantina` | A small story with a character whose name changes. |
| `examples/zork1` | Zork I, imported from its ZIL source. A large map for tests. Read `examples/zork1/README.md`. |
| `examples/tower` | A spiral tower on four levels, for the level map. |
| `crates/lantern-zil` | The importer for ZIL games: `lantern-zil <zil-folder> <project-folder>`. |
| `docs/writing-stories.md` | How to write a story: files, fragments, markup, conditions, frames, name stages, levels, and how to write in the game. |
| `docs/ai-tools.md` | How to connect an AI assistant to a project with MCP. |
| `web/` | The SvelteKit app and the design system with its Storybook. |

## Commands

Install [go-task](https://taskfile.dev), Trunk, Node.js and pnpm before you start.

| Command | What it does |
| --- | --- |
| `task play` | Plays the example in the terminal. |
| `task import:zork` | Imports Zork I again from its ZIL source into `examples/zork1`. |
| `task web:install` | Installs the web dependencies. Do this one time. |
| `task dev` | Builds the WebAssembly package, then runs `trunk watch` and the Vite dev server. |
| `task storybook` | Starts Storybook on port 6006. |
| `task web:check` | Type-checks the Svelte code and the stories. |
| `task verify` | Examines the format, runs clippy and runs the Rust tests. |

If you do not use go-task:

1. Play in the terminal with `cargo run -p lantern-cli -- play examples/lighthouse`.
2. For the web app, run `trunk watch` in `crates/lantern-wasm`.
3. At the same time, run `pnpm dev` in `web/`.

## How Trunk and SvelteKit work together

Trunk builds the Rust part. The file `crates/lantern-wasm/Trunk.toml` tells Trunk to build the crate with wasm-bindgen and TypeScript declarations. Trunk writes the output to `web/src/lib/engine/pkg/` without content hashes. `trunk watch` builds again when a crate changes.

SvelteKit and Vite make the user interface. They import the package as a normal module. Thus a change in Rust gets to the browser as a normal Vite reload.

## Design system

The web app uses design tokens and atomic design. Storybook shows and documents each element. Start with the "Introduction" page in Storybook.

These rules apply to all web work:

- Use only design tokens for colour, type, space, size and motion.
- Make each element accessible from the start. The target is WCAG 2.2 AA.
- Make each element work at all widths from 320px.
- Write a story with documentation for each element.

## Documentation

Write all documentation in ASD-STE100 Simplified Technical English. The "Foundations/Writing documentation" page in Storybook gives the rules.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
