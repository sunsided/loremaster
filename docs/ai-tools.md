# AI tools

lantern has an MCP server. With it, an AI assistant can read, play and verify a project. Examples of assistants are Claude Code and Claude Desktop.

The tools do not change project files. The assistant can only suggest changes, as proposals. You accept a proposal, or you reject it.

## Start the server

There are two servers. Both serve the same tools.

| Command | Transport | Use it when |
| --- | --- | --- |
| `lantern mcp <project-folder>` | stdin and stdout | One assistant uses the project. The assistant starts the server. |
| `lantern serve <project-folder> [--port N]` | HTTP on `127.0.0.1`, port 4710 by default | The editor and one or more assistants use the project at the same time. You start the server. |

```sh
cargo build --release -p lantern-cli
target/release/lantern serve examples/lighthouse
```

## Connect Claude Code

1. Build the CLI (see above).
2. Add the server. Use absolute paths:

   ```sh
   claude mcp add lantern -- /path/to/lantern/target/release/lantern mcp /path/to/my-story
   ```

3. Start Claude Code and ask, for example: "Review the room shore."

If `lantern serve` runs, connect to it instead:

```sh
claude mcp add --transport http lantern http://127.0.0.1:4710/mcp
```

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
| Propose | `propose`, `list_proposals`, `get_proposal`, `withdraw_proposal` |
| Play | `session_start`, `input`, `act`, `author`, `scene`, `assembly`, `room_parts`, `inspect`, `vars`, `rules`, `verbs`, `map`, `save` |

The server also gives this guide as a resource: `lantern://guide/writing-stories`.

## Sessions

A play session is one game in the server. The tools use the session `main` if you do not name a session. The session `main` starts when a tool first uses it.

To start a different session, use `session_start` with a new name. `session_start` can also start from a state that `save` gave, or from a state that the editor gave.

## Changes to the files

Before each tool call, the server reads the project folder again. If a file changed, the server loads the project again. The sessions keep their state.

If the project has errors, the read tools and `check` still work. The play tools tell you to use `check`.

## Proposals

A proposal is a change that an assistant suggests. It contains whole files: new or changed files with their full text, and the paths of files to delete. It can only touch `story.yaml` and the files in `rooms/`, `things/` and `rules/`.

The server checks a proposal against the project as it is now. The result is a proposal view:

| Field | Contents |
| --- | --- |
| `id`, `summary` | The id, for example `p3`, and one line about the change. |
| `ok` | `true` when the project with the proposal has no errors. |
| `check` | All problems of the project with the proposal. |
| `new_problems` | The problems that the proposal adds. |
| `changes` | The changes, one item for each entity, field or fragment. See below. |
| `rooms` | The text of each new or changed room in all its states: `{ room, texts: [{ text, combinations, example, warnings }] }`. |
| `files`, `delete` | The paths that the proposal writes and deletes. |

Each item of `changes` has a `kind`, an `entity` (for example `room:shore`, `thing:lamp`, `rule:find_key` or `story`) and a `file`:

| `kind` | More fields |
| --- | --- |
| `added` | `after`: the new entity. |
| `removed` | `before`: the old entity. |
| `field` | `field`, `before`, `after`: a field that is not a fragment list, for example `exits`. |
| `fragment` | `field` (`description`, `presence` or `intro`), `fragment_id` if the fragment has an id, `before_index`, `after_index`, `before`, `after`. An added fragment has no `before`. A removed fragment has no `after`. |

The values are the YAML values as JSON.

The servers keep the proposals in `.lantern/proposals/<id>.json` in the project folder. Thus all servers on one folder see the same proposals. The servers write `.lantern/.gitignore` one time, so that git ignores the proposals. To track the proposals in git, remove the line `proposals/` from that file.

To play a proposal before you accept it, use `session_start` with `proposal`.

## HTTP endpoints

`lantern serve` has these endpoints:

| Method and path | What it does |
| --- | --- |
| `POST /mcp` | Sends one MCP message. The reply is JSON. |
| `GET /proposals` | Gives the views of all open proposals. |
| `GET /proposals/<id>` | Gives the view of one proposal. |
| `POST /proposals/<id>/accept` | Writes the proposal into the project files, then removes the proposal. A proposal with errors is refused (status 409). |
| `POST /proposals/<id>/reject` | Removes the proposal. The body `{ "note": "why" }` is optional. The server adds the note to `.lantern/rejected.jsonl`. |

The server accepts requests only from `127.0.0.1`. A request from a browser must come from a page on `localhost`, `127.0.0.1` or `[::1]`. A `POST` must have `Content-Type: application/json`.
