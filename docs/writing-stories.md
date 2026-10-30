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
- Place: `here == cellar`, `in(cellar)`, `cellar.visits`, `visited(cellar)`, `first_visit`, `level`.
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

## Exits

An exit is a way from one room to a different room. Write the exits of a room in its file, one on each line:

```yaml
# rooms/shore.yaml
exits:
  - { direction: north, to: cottage, door: cottage_door }
  - { direction: east, to: stairwell, door: lighthouse_door }
  - { direction: west, blocked: "Cold surf breaks over the rocks. There is only the sea that way." }
```

| Field | Contents |
| --- | --- |
| `direction` | `north`, `northeast`, `east`, `southeast`, `south`, `southwest`, `west`, `northwest`, `up`, `down`, `in`, `out`, or a name. |
| `to` | The room that the exit goes to. |
| `door` | A thing that must be open. The player opens a closed door first. A locked door stops the player. |
| `visible` | A condition. When it is false, the exit is hidden and the player cannot use it. |
| `passable` | A condition. When it is false, the player cannot go through, and the game shows `blocked`. |
| `blocked` | Markup. The game shows it when the player cannot go through. |
| `traversal` | Markup. The game shows it when the player goes through. |

### Message exits

An exit without `to` is a message exit. It goes to no room. When the player goes in its direction, the game shows `blocked`, and the player stays in the room. The move uses a turn, as a move that fails.

- A message exit must have `blocked`. The check gives an error for an exit without `to` and without `blocked`.
- `blocked` is markup, as in all exits. It can have conditions, for example `"{night: The surf is black. | Cold surf breaks over the rocks.}"`.
- `visible` works as in other exits. When it is false, the player gets the default response.
- A message exit does not use `door`, `passable` or `traversal`. The check gives a warning for them.
- The map, the scene, the verb bar and the room brief do not show message exits, because they go to no room.
- In write mode, a new exit in the direction of a message exit replaces the message exit.

## The Atlas

The Atlas shows the whole world at full width. Push Ctrl+M, or select the room name in the status line. Push Escape, or select "Back to …", to go back to the game at the same turn.

| View | What it shows |
| --- | --- |
| Map | The floors on the left. The map of a floor in the middle: drag or use the arrow keys to move, the wheel or + and − to zoom, F to see the whole floor. "/" finds a room or a thing. "List view" shows the rooms as a table. The room card on the right has "Go there", "Write here" and "Explore". |
| Rules | All rules, with "Here and now", "Fired" and "All". |
| State | All variables and properties, with their values now. A switch sets a value in the game. |
| Story | The turns so far. |
| Checks | The texts of the current room in all states, and the whole-game explorer. |

## Levels

A room can be on a level, for towers and dungeons. Up exits go one level higher, down exits one level lower.

```yaml
# rooms/north_2.yaml
name: "North Landing, Second Floor"
level: 2
position: [1, 0]
```

- A room without `level` gets its level from the `up` and `down` exits, counted from the start room (level 0).
- `position` is the place of the room on its level. Rooms on different levels can have the same position.
- The condition `level` is the level of the current room, for example `level == 3`.

With more than one level, the map has two views:

- **One level**: the level of the player in full. The levels above and below show faint behind it. ▲ and ▼ mark rooms with stairs. "Up" and "Down" show another level.
- **All levels, unfolded**: one row for each level, the highest at the top, the rooms of each level in ring order (north first, then clockwise). Each row moves sideways so that the top of a stair is right above its foot, and the stair is a straight line. A spiral reads as a staircase, so a stair to the wrong room is easy to see. A wide drawing scrolls sideways, and it starts at the current room.

The example `examples/tower` ("The Spiral") is a tower with three floors of four landings each. The stair on each west landing goes up to the north landing of the next floor.

## Examine one room in all conditions

The room matrix shows the text of one room in each combination of the values that the text depends on. It also shows combinations that the player can not reach. Use it to find a text that reads badly in one condition.

1. In the web app, go to the room.
2. In the Here panel, the section "Depends on" shows the values that the room text depends on. Change a value to preview the room text in place; the game does not move. "Make it so" sets the values in the game. "All texts ↗" opens Atlas › Checks with all the texts. "This room" is its default mode.

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

1. In the web app, open Atlas › Checks (Ctrl+M, then "Checks"), select "Whole game", then select "Explore all states".
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

1. Select "Write" in the header, or push Ctrl+E (Cmd+E on a Mac). The shortcut also works while you type.
2. In the newest room text, select a fragment. It opens for editing in place, and it still reads as the player reads it.
3. Type in the game text. Markup shows as a chip with the text that it gives now; select a chip to see and change its markup.
4. The fragment panel has the properties: the mode ("Shows"), the condition ("When"), the layer and the variants. "Reads in other states" shows how the text reads in other values. "</> Show markup" (Ctrl+/) shows the text with all its markup; you can type in both places. On a screen from 900px wide, the panel is next to the game text, with the room pinned on top: change a value there to see the fragment in the game text in that value. On a narrow screen, the panel is under the paragraph.
5. Push Enter to save. Push Shift+Enter for a line break. Push Escape to cancel.

### Type first, tune later

Select words in the game text or in the writer. A small toolbar shows (or push Ctrl+.):

| Action | Result |
| --- | --- |
| Use the name | The selected name of a thing, for example "the oil lamp", becomes `{oil_lamp.the}`. Then the name changes with the thing. |
| Make it a thing | The selected words become a new thing in the room. The text does not change. |
| Only the first time | `{first: …}` goes around the selection. |
| Only when… | `{ ? …}` goes around the selection. Write the condition before the `?`. |
| Make variants | Write alternatives with " / " between them, for example `The wind howls / drops.`, and select them. Each alternative becomes a variant of the fragment. |

More actions in write mode:

- "+ Add room text" adds a fragment to the current room.
- In write mode, each fragment of the room text is on its own line, with a note in the margin: its layer, its mode and its condition. "+ N hidden here" shows the fragments that do not show now, struck through, with the reason in the margin. Select one to edit it.
- "Remove this fragment" removes the fragment from the file.

Help while you type:

- Completion: in markup (inside `{…}`) and in the condition, the editor suggests names with their values now. After `==` it suggests the values, for example the phases of the day. Push Ctrl+Space to open the list.
- Hover: put the pointer on a name to see what it is and its value now.
- Quick fix: for an unknown name, "Use …" puts in the name that the editor finds, for example `oil_lamp.lit` for `lamp.lit`.
- "Other values" shows how the text reads in each combination of the values that it depends on, and in which combinations it does not show.

The editor examines the text while you type. A problem has a wavy line and a message, and you cannot save until you correct it. When a change does not build, the writer stays open with the message, and the game keeps the last good version.

### Make new rooms, things and responses

In write mode, the game helps you make what is not there yet:

| You do | The game shows | It makes |
| --- | --- | --- |
| Walk in a direction that has no exit, for example `s`. | A form for a new exit. | The exit, a new room or a link to an existing room, and the way back. You walk into the new room, and the writer opens for its text. |
| Write a name that the story does not know, for example `{brass_key.The}`. | "Create thing brass_key" under the problem. | A thing in the current room. The writer stays open. |
| Give a command that gets the default response, for example `push brass key`. | "+ Write a response for “push brass key”". | A rule in `rules/<room>.yaml` that answers the command with your text. |
| Ctrl+K, "Create a thing", "Create a character" or "Create an exit". | The same form. | A thing or a character in the current room (the writer then opens for its presence text), or an exit. |

The editor makes the ids from the names. For example, "The Beach" becomes `the_beach`.

### Where the changes go

A change goes into the story file. Only the changed lines in the file change, so comments and formatting stay.

- The browser keeps a working copy of each story (in the Origin Private File System). The header shows "edited, saved in browser".
- In Chrome, Edge and other Chromium browsers, use the command palette (Ctrl+K), "Open a story folder on disk". Then each change goes to the folder too. After a reload of the page, the browser asks for access to the folder again: use "Give access to the folder again" in the command palette.
- In all browsers, "Export the story as a zip file" and "Import a story from a zip file" move a story in and out.
- "Go back to the bundled example" removes the working copy.
