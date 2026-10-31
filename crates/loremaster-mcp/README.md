# loremaster-mcp

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate has the MCP server for one project:

- `loremaster mcp <folder>` on stdin and stdout.
- `loremaster serve <folder>` on local HTTP (`127.0.0.1:4710`), with the proposal and job endpoints for the editor.

The server gives the tools of `loremaster-tools` and the writing guide as the resource `loremaster://guide/writing-stories`.

## Feature `embed-ui`

With the feature `embed-ui`, `loremaster serve` also serves the web editor. The editor comes from `ui/` in this crate. The crates.io package contains `ui/`. In a git checkout, run `task release:ui` to make it.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
