# loremaster

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate gives the `loremaster` command.

## Install

```sh
cargo install loremaster
```

The binary contains the web editor. `loremaster serve` serves it at `http://127.0.0.1:4710/`.

## Commands

| Command | What it does |
| --- | --- |
| `loremaster new <folder> [--title T] [--idea I]` | Makes a new story. With `--idea`, the AI drafts the story. |
| `loremaster play <folder>` | Plays the story in the terminal. |
| `loremaster check <folder> [--todos]` | Examines the story for problems, or lists the open TODOs. |
| `loremaster explore <folder>` | Walks all reachable states and shows the coverage. |
| `loremaster mcp <folder>` | Starts the MCP server on stdin and stdout. |
| `loremaster serve <folder \| --bootstrap> [--port N]` | Starts the editor and the MCP server on local HTTP. |

Read [How to write a story](https://github.com/sunsided/loremaster/blob/main/docs/writing-stories.md) and [AI tools](https://github.com/sunsided/loremaster/blob/main/docs/ai-tools.md).

## Build without the editor

In a git checkout, the editor is in the build only after `task release:ui`. To build without it, use `cargo build -p loremaster --no-default-features`. Then `loremaster serve` tells you to use the hosted editor at <https://sunsided.github.io/loremaster/>.

## Stories in Rust

With the cargo feature `dsl`, the crate is also a library: `loremaster::prelude` gives the builder API of `loremaster-dsl` and the `world!` macro of `loremaster-macros`. Write a story in Rust, join it with YAML folders, and build the world. The feature `builder` is the old name of `dsl`.

```toml
loremaster = { version = "0.1", default-features = false, features = ["dsl"] }
```

```rust
use loremaster::prelude::*;

world! {
    story "The Lighthouse" { start: shore }
    room shore "The Shore" {
        "Shingle crunches underfoot.";
    }
}

let world = Loremaster::new().pack(pack()).yaml("story/").build()?;
```

## License

EUPL-1.2. Read [LICENSE](LICENSE).
