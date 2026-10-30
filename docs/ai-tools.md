# AI tools

lantern has an MCP server. With it, an AI assistant can read, play and verify a project. Examples of assistants are Claude Code and Claude Desktop.

The tools do not change project files. The assistant can only suggest changes, as proposals. You accept a proposal, or you reject it.

## Start the server

There are two servers. Both serve the same tools.

| Command | Transport | Use it when |
| --- | --- | --- |
| `lantern mcp <project-folder>` | stdin and stdout | One assistant uses the project. The assistant starts the server. |
| `lantern serve <project-folder> [--port N]` | HTTP on `127.0.0.1`, port 4710 by default | The editor and one or more assistants use the project at the same time. You start the server. |
| `lantern serve --bootstrap [--port N]` | The same | You start a new story in the editor and want the AI to draft it. The server uses a new empty folder in the temporary directory. |

If the folder does not exist, `lantern serve` makes it.

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
  --tools "ToolSearch,ListMcpResourcesTool,ReadMcpResourceTool" \
  --allowedTools "mcp__lantern__*,ToolSearch,ListMcpResourcesTool,ReadMcpResourceTool"
```

`--tools` turns off the built-in tools, for example Read, Grep and Bash. The three tools in the list stay on: Claude Code needs them to read the guide resource and to find deferred tools. Do not use `--tools ""`: then Claude Code cannot read the guide.

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

In `overview`, a message exit (an exit without `to`) has the value `null`. `neighbors` and `map` do not show message exits, because they go to no room.

## Task prompts

A task prompt is a request for one task. The editor and MCP clients use the same prompts. Thus a task works the same in the editor and in an assistant.

| Prompt | Arguments | What the assistant does |
| --- | --- | --- |
| `imagine_exit` | `room`, `direction`. Optional: `session`, `rejected`. | The player tried to go in a direction that has no exit. The assistant makes 3 proposals of different kinds, for example a new room, a hidden exit or a scenery thing. It replies with one line for each proposal: the id and a short pitch. |
| `seed_story` | `idea`. Optional: `rooms` (2 or 3, the default is 3), `rejected`. | The project is a new story. The assistant expands the idea into the first rooms, as one proposal: `story.yaml` with a title, an intro and the premise as a note, the rooms with exits in both directions and map positions, at most 2 things, one TODO in each room, and 1 or 2 open threads with their setup notes. It replies with the proposal id and one line for each room. |
| `review_room` | `room`. Optional: `session`. | The assistant reads the room in all its states and in its surroundings. It replies with a list of findings: the fragment, how serious the problem is, the problem and the fix. It makes one proposal for the clear fixes. |

`session` is the play session that has the state of the player. The default is `main`. The assistant does not change this session. `rejected` is a list of ideas that the author rejected before, one on each line. The assistant does not offer these ideas again.

An MCP client gets the prompts with `prompts/list`. It gets the text of one prompt with `prompts/get`, for example:

```json
{ "jsonrpc": "2.0", "id": 1, "method": "prompts/get",
  "params": { "name": "imagine_exit", "arguments": { "room": "shore", "direction": "down" } } }
```

## Start a new story with AI

You can start a story from an idea. The AI writes the first 2 or 3 rooms as one proposal. You accept it or reject it.

| Where | How |
| --- | --- |
| Terminal | `lantern new <folder> --title "The Drowned Bell" --idea "A church bell sank in a storm. At low tide it still rings."` The command writes the new story, runs the job and tells you the proposal id. Then start `lantern serve <folder>` and accept the proposal in the editor. |
| Editor | Start `lantern serve --bootstrap`. In the editor, select "New story…", type the title and the idea, then select "Create and draft with AI". When the proposal comes, open Atlas › Proposals and select "Accept". The server also writes the files into its folder, and the game starts again from the intro. |
| MCP client | Call the tool `bootstrap_project` with `idea` (and `title`, `rooms` if you want). It writes the new story, then gives the `seed_story` request. The assistant follows the request. |

Bootstrap works only on a folder that has no story, or that has only a new story without changes. Else it refuses, and no file changes.

A job takes approximately 1 minute. At the `low` effort level, a test run cost approximately $0.26, as an estimate with a Claude plan login. "✦ Another take" in the editor starts one more job; each job has its own cost.

## Jobs

A job is one task prompt that `lantern serve` gives to Claude Code. The editor starts a job with `POST /jobs`. Then it reads the progress with `GET /jobs/<id>`.

The server starts `claude -p` with the login of the user, for example a Max plan. Claude Code connects to the server at `/mcp` and gets only the lantern tools, the MCP resources and the search for deferred tools. It cannot read files, search files or run commands on the computer. Thus it can only read, play and propose.

These environment variables change the agent:

| Variable | Contents |
| --- | --- |
| `LANTERN_CLAUDE` | The `claude` program. The default is `claude` on the `PATH`. |
| `LANTERN_CLAUDE_MODEL` | The model. The default is `sonnet`: it is fast for these short tasks and uses less than half the quota of a larger model. Set it to `opus`, for example, for more care. Set it to an empty value to use the default model of Claude Code. |
| `LANTERN_CLAUDE_EFFORT` | The effort level of Claude Code: `low`, `medium`, `high`, `xhigh` or `max`. The default is `low`, because the agent then writes its proposals sooner. Set it to an empty value to use the default of Claude Code. |

The body of `POST /jobs` has these fields:

| Field | Contents |
| --- | --- |
| `task` | The name of a task prompt, for example `imagine_exit`. |
| `args` | The arguments of the prompt, as an object. |
| `state` | Optional. A state that the editor saved. The server starts a new session `job-<id>` from this state, and the job uses that session. |
| `model` | Optional. The model for this job, for example `opus`. It replaces `LANTERN_CLAUDE_MODEL`. |

For `imagine_exit` and `seed_story`, if `args` has no `rejected`, the server adds the last 20 rejected proposals from `.lantern/rejected.jsonl`.

Before the server starts the agent, it gets the context of the task and puts it in the prompt: `story.yaml`, the room file, the neighbours (1 hop) and the scene of the session. For `imagine_exit` it also gives the variables. For `review_room` it also gives the room matrix. The writing guide is in the prompt too. Thus the agent does not use turns to get them. An MCP client that uses `prompts/get` does not get this context. The prompt then tells the assistant to get it with the tools.

The reply is `{ "id": "j1" }` with status 202. A job has these fields:

| Field | Contents |
| --- | --- |
| `id`, `task`, `args` | The id, the task prompt and its arguments. |
| `status` | `running`, `done`, `failed` or `cancelled`. |
| `events` | The steps of the job: `{ kind, text }`. `kind` is `tool` (the agent used a lantern tool; `text` is the tool name), `text` (the agent wrote text) or `error`. |
| `result` | The last reply of the agent, or `null`. |
| `proposals` | The ids of the proposals that the agent made and did not withdraw. |
| `agent_session` | The Claude Code session id. |
| `model` | The model that the job asked for, or `null` for the default model. |
| `cost_usd` | The cost that Claude Code gave for the job. With a Claude plan login, this is an estimate, not a charge. |

The server adds each proposal to `proposals` when the agent makes it. Thus the editor can show the first proposal before the job is done.

When you cancel a job, the proposals that it made stay.

## Sessions

A play session is one game in the server. The tools use the session `main` if you do not name a session. The session `main` starts when a tool first uses it.

To start a different session, use `session_start` with a new name. `session_start` can also start from a state that `save` gave, or from a state that the editor gave.

## Changes to the files

Before each tool call, the server reads the project folder again. If a file changed, the server loads the project again. The sessions keep their state.

If the project has errors, the read tools and `check` still work. The play tools tell you to use `check`.

## Proposals

A proposal is a change that an assistant suggests. It contains whole files: new or changed files with their full text, and the paths of files to delete. It can only touch `story.yaml` and the files in `rooms/`, `things/`, `rules/` and `threads/`.

`propose` takes the changes in two forms:

| Field | Contents |
| --- | --- |
| `files` | New files, or files that the assistant replaces as a whole: `{ path, text }`. |
| `edits` | Changes to files that exist: `{ path, old, new }`. The server replaces `old` with `new` in the text of the file. `old` must occur one time in the file. Edits of one file apply in order. A file cannot be in `files` and in `edits`. |
| `delete` | The paths of files to delete. |

The server makes whole files from the edits. Thus a proposal always contains whole files. Edits are much shorter than whole files, and the assistant writes them faster.

The server checks a proposal against the project as it is now. The result is a proposal view:

| Field | Contents |
| --- | --- |
| `id`, `summary` | The id, for example `p3`, and one line about the change. |
| `ok` | `true` when the project with the proposal has no errors. |
| `stale` | `true` when a file that the proposal touches changed after the proposal was made. |
| `check` | All problems of the project with the proposal. |
| `new_problems` | The problems that the proposal adds. |
| `changes` | The changes, one item for each entity, field or fragment. See below. |
| `rooms` | The text of each new or changed room in all its states: `{ room, texts: [{ text, combinations, example, warnings }] }`. |
| `files` | Each touched file: `{ path, base_sha, before, after }`. `base_sha` is the SHA-256 of the file when the proposal was made, or `null` for a new file. `before` is the text on disk now. `after` is the text after the proposal, or `null` when the proposal deletes the file. |

The `after` text is the text that the assistant gave, or the file with the edits. The server does not format it again.

Each item of `changes` has a `kind`, an `entity` (for example `room:shore`, `thing:lamp`, `rule:find_key` or `story`) and a `file`:

| `kind` | More fields |
| --- | --- |
| `added` | `after`: the new entity. |
| `removed` | `before`: the old entity. |
| `field` | `field`, `before`, `after`: a field that is not a fragment list or a frame, for example `exits`. |
| `fragment` | See below. |

A `fragment` change has these fields:

| Field | Contents |
| --- | --- |
| `owner` | The owner of the fragment list, as the runtime writes it in `Source.owner`. Examples: `room:shore`, `thing:lamp`, `thing:lamp/presence`, `story:intro`, `room:shore/frames/look/before`, `story/frames/look/before`. |
| `field` | The YAML path of the list in its entity, for example `["description"]` or `["frames", "look", "before"]`. |
| `fragment_id` | The `id` of the fragment, if it has one. |
| `before_index`, `after_index` | The index of the fragment in the list before and after. Together with `owner`, an index finds the runs in the transcript. |
| `before`, `after` | The fragment as YAML gives it: a string or a map. An added fragment has no `before`. A removed fragment has no `after`. |
| `before_fragment`, `after_fragment` | The same fragments in one form: `{ id, variants, when, mode, layer, replaces, notes }`. |

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
| `POST /proposals/<id>/accept` | Writes the proposal into the project files, then removes the proposal. A proposal with errors is refused (status 409). A stale proposal is refused too, unless the body is `{ "force": true }`. If the editor wrote the change itself, send `{ "applied": true }`: the server then only removes the proposal. |
| `POST /proposals/<id>/reject` | Removes the proposal. The body `{ "note": "why" }` is optional. The server adds the note to `.lantern/rejected.jsonl`. |
| `POST /jobs` | Starts a job. See [Jobs](#jobs). The reply has status 202. |
| `GET /jobs` | Gives all jobs. |
| `GET /jobs/<id>` | Gives one job, with its status, events, result and proposals. |
| `POST /jobs/<id>/cancel` | Stops a running job. |
| `POST /bootstrap` | Starts a new story from an idea. The body is `{ "title": "...", "idea": "...", "rooms": 3 }`; `title`, `rooms` and `model` are optional. The server writes the new story (the same files as `lantern new`), then starts a `seed_story` job. The reply is `{ "id": "j1" }` with status 202. When the folder has a story, the reply has status 409. |

The server accepts requests only from `127.0.0.1`. A request from a browser must come from a page on `localhost`, `127.0.0.1` or `[::1]`. A `POST` must have `Content-Type: application/json`.
