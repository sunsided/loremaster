# AI tools

lantern has an MCP server. With it, an AI assistant can read, play and verify a project. Examples of assistants are Claude Code and Claude Desktop.

The tools do not change project files. The assistant can only suggest changes. You make the changes.

## Start the server

The server uses stdin and stdout. The assistant starts it. You do not start it yourself.

```sh
cargo build --release -p lantern-cli
target/release/lantern mcp examples/lighthouse
```

## Connect Claude Code

1. Build the CLI (see above).
2. Add the server. Use absolute paths:

   ```sh
   claude mcp add lantern -- /path/to/lantern/target/release/lantern mcp /path/to/my-story
   ```

3. Start Claude Code and ask, for example: "Review the room shore."

To let Claude Code use only the lantern tools, start it like this:

```sh
claude -p "Review the room shore." \
  --mcp-config lantern-mcp.json --strict-mcp-config \
  --allowedTools "mcp__lantern__*" --disallowedTools Bash Edit Write
```

The file `lantern-mcp.json` contains:

```json
{
  "mcpServers": {
    "lantern": {
      "command": "/path/to/lantern/target/release/lantern",
      "args": ["mcp", "/path/to/my-story"]
    }
  }
}
```

## The tools

| Group | Tools |
| --- | --- |
| Read the world | `overview`, `list_files`, `read_file`, `get_entity`, `neighbors` |
| Verify | `check`, `explore`, `room_matrix`, `text_matrix`, `try_markup`, `try_condition`, `symbols` |
| Play | `session_start`, `input`, `act`, `author`, `scene`, `assembly`, `room_parts`, `inspect`, `vars`, `rules`, `verbs`, `map`, `save` |

The server also gives this guide as a resource: `lantern://guide/writing-stories`.

## Sessions

A play session is one game in the server. The tools use the session `main` if you do not name a session. The session `main` starts when a tool first uses it.

To start a different session, use `session_start` with a new name. `session_start` can also start from a state that `save` gave, or from a state that the editor gave.

## Changes to the files

Before each tool call, the server reads the project folder again. If a file changed, the server loads the project again. The sessions keep their state.

If the project has errors, the read tools and `check` still work. The play tools tell you to use `check`.
