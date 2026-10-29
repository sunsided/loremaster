# Writing stories

This guide tells you how to write a story for lantern. The examples are in `examples/lighthouse` and `examples/cantina`.

## Project folder

A story is a folder of YAML files. Each entity has its own file.

| Path | Contents |
| --- | --- |
| `story.yaml` | Title, start room, intro, clock, variables, custom verbs and story frames. |
| `rooms/*.yaml` | One room in each file. |
| `things/*.yaml` | One thing or character in each file. |
| `rules/*.yaml` | One rule, or a list of rules, in each file. |

The id of an entity is its file name. You can set `id:` to use a different id.

To examine a story for problems, run `task check:story` or `cargo run -p lantern-cli -- check <folder>`.

## Fragments

A text is a list of fragments. A fragment is a plain string, or a map with these fields:

| Field | Meaning |
| --- | --- |
| `text` | The text, with inline markup. |
| `variants` | More texts for the modes `cycle`, `shuffle` and `sticky`. |
| `when` | A condition. The fragment shows only when the condition is true. |
| `mode` | `always`, `first_visit`, `once`, `cycle`, `shuffle` or `sticky`. |
| `layer` | `base`, `ambient`, `event`, `objects`, `characters` or `exits`. |
| `replaces` | `true` hides the fragments before it in the same layer. |

## Inline markup

| Markup | Result |
| --- | --- |
| `{night: moonlit \| sunlit}` | "moonlit" at night, else "sunlit". |
| `{first: , new to you,}` | The text only on the first visit. |
| `{lamp.lit ? glowing : dark}` | Any condition, with two texts. |
| `{weather}` | The value of a variable. |
| `{lamp.name}`, `{lamp.the}`, `{lamp.The}`, `{lamp.a}`, `{lamp.A}` | The current name of a thing, with an article. A proper name gets no article. |

## Conditions

Conditions use variables, thing properties and values that the runtime calculates:

- Variables and properties: `flooded`, `lamp.lit`, `sanity >= 3`, `weather == fog`.
- Time: `night`, `time.phase == dusk`, `time.hour`, `time.day`, `turns`.
- Place: `here == cellar`, `in(cellar)`, `cellar.visits`, `visited(cellar)`, `first_visit`.
- Things: `carried(lamp)`, `present(lamp)`, `lamp.location`, `lamp.interactions`, `lamp.talked`.
- The current action: `action.repeat`, `action.count`, `action.total`, `action.verb`.

Use `and`, `or`, `not` and parentheses to join conditions.

## Action frames

An action frame adds author text around the response to a verb. Frames are optional. A story without frames plays as before.

```yaml
# story.yaml, a room file or a thing file
frames:
  look:
    before:
      - when: action.repeat == 1
        text: "You look around."
      - when: action.repeat == 2
        text: "You look around, again."
      - when: action.repeat >= 3
        text: "You keep looking around, and nothing seems to change."
```

A frame has three parts. All parts are lists of fragments:

- `before` shows before the response.
- `instead` replaces the default response when one of its fragments applies. Rules still fire.
- `after` shows after the response.

The runtime uses the most specific frame for the verb: the frame on the first thing of the command, then the frame on the current room, then the frame on the story.

The counters include the current action:

- `action.repeat` is the number of times in a row that the player gave the same command.
- `action.count` is the number of times that the player used the verb in this room.
- `action.total` is the number of times that the player used the verb in the game.

The room description that shows when the player enters a room is not an action. Frames do not apply to it.

## Name stages

A thing or a character can have names that change with the state. The last stage whose condition is true is the current name.

```yaml
# things/blackbeard.yaml
name: shadow
names:
  - { text: shadow, aliases: [figure, shape] }
  - { when: blackbeard.talked > 0, text: grim-looking man, aliases: [man] }
  - { when: knows_blackbeard_name, text: Blackbeard, aliases: [captain], proper: true }
```

- The text, the verb bar, the room brief and the default responses use the current name.
- The parser accepts the names of the current stage and of all stages before it. It does not accept a name that the player has not learned yet.
- Set `proper: true` for a proper name. A proper name gets no article.
- The author tooltip shows all stages and the current stage.

## Examine one room in all conditions

The room matrix shows the text of one room in each combination of the values that the text depends on. It also shows combinations that the player can not reach. Use it to find a text that reads badly in one condition.

1. In the web app, go to the room.
2. Open the "Explore" tab of the inspector. "This room" is the default mode.

The runtime finds the values in these locations:

- The conditions of the room fragments.
- The inline markup of the room fragments.
- The presence texts and the name stages of the things in the room.

For a number, the runtime tries the values next to each limit in a condition. For example, `talked > 0` gives 0 and 1. The matrix renders not more than 256 combinations.

The view shows each different text one time, with the combinations that give it. It marks these possible problems:

- No room text.
- Two spaces in a row, a space before punctuation, or a brace in the text.
- Two replacing fragments of one layer that apply at the same time.
- Two or more presence texts of one thing that show at the same time.

After a "Whole game" run, each text has a mark: "seen in the whole-game run" or "not seen in the whole-game run". A text that the run did not see can be unreachable, or it can be outside the budget of the run.

"Try this" puts the game into one combination. It moves the player to the room and sets each value. It does not play a turn.

## Examine all states

The state explorer plays all ways through the story from the start. It stops at 2000 states or 40 commands.

1. In the web app, open the "Explore" tab of the inspector, select "Whole game", then select "Explore all states".
2. Or, run `task explore` or `cargo run -p lantern-cli -- explore <folder>` in a terminal.

The explorer shows:

- Each way the text of a room reads, with the shortest command list to it.
- Fragments that never show and rules that never fire.
- Endings, with the shortest command list to each.
- Stuck states: states from which the player can not reach an ending.

The explorer tries the actions that move the story forward: exits, verbs that rules listen to, take, open, turn on, unlock with a key in reach, and wait when the story reacts to time. It does not try drop, close, turn off or lock.

In the web app, "Replay this path" starts the game again and plays the command list. You can then continue from that state.

## Write in the game

You can change the story while you play it. The game keeps its state after each change.

1. Select "Write" in the header, or push W.
2. In the newest room text, select a fragment. The writer opens in place of the fragment.
3. Change the text, the condition ("When"), the layer or the mode. "Now it reads" shows the text as the player reads it now.
4. Push Enter to save. Push Shift+Enter for a line break. Push Escape to cancel.

More actions in write mode:

- "+ Add room text" adds a fragment to the current room.
- In the "Assembly" tab, "Edit" opens the writer for a fragment that does not show now.
- "Remove this fragment" removes the fragment from the file.

The editor examines the text while you type. A problem has a wavy line and a message, and you cannot save until you correct it. When a change does not build, the writer stays open with the message, and the game keeps the last good version.

### Where the changes go

A change goes into the story file. Only the changed lines in the file change, so comments and formatting stay.

- The browser keeps a working copy of each story (in the Origin Private File System). The header shows "edited, saved in browser".
- In Chrome, Edge and other Chromium browsers, use the command palette (Ctrl+K), "Open a story folder on disk". Then each change goes to the folder too. After a reload of the page, the browser asks for access to the folder again: use "Give access to the folder again" in the command palette.
- In all browsers, "Export the story as a zip file" and "Import a story from a zip file" move a story in and out.
- "Go back to the bundled example" removes the working copy.
