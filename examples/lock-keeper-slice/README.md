# The Lock-Keeper's Night

Two stories live in this folder:

- The golden slice of phase 0 (`yaml/`, `dsl/`, `mixed/`): three rooms, written before the engine could run them. Read `PENDING.md`.
- The full demo (`demo/`, P4-07 #84): the design "The Lock-Keeper's Night", steps 1 to 7, in YAML.

## The demo

Autumn 1923. Amos Reed, the lock-keeper, lies drowned in his own lock. You are the county constable, and the coroner comes at three. Prove means, opportunity and motive against the right person before he comes, or the death goes down as an accident.

Play it in the terminal:

```sh
cargo run -p loremaster --no-default-features -- play examples/lock-keeper-slice/demo
```

Or open the folder in the editor with `loremaster serve examples/lock-keeper-slice/demo`.

- `journal` shows the case notebook: one quest for each kind of proof.
- `hint` shows the hint ladder of the room you are in. The hints come from `demo/tests/best.yaml` and `demo/tests/ashes.yaml`.
- `accuse <person>` ends the game, with the case that you have then. At three the coroner reads your notebook.
- People keep their own hours (`demo/agendas/`): Marsh cleans his boots at eleven and goes to the lock at one, Bert goes to sleep at half past twelve, Hester slips out to the barge at a quarter past eleven.
- `show notes to marsh` shows Marsh your case notebook. With two kinds of proof in it he makes for the door, and the game ends ("He ran") unless your next command is `stop marsh`. Stopped, he is held in the parlour for the rest of the night.
- Hester lies about where she was. A kind word in the kitchen before eleven, or a wait on the barge at midnight, gives her alibi: it clears Crane, and proves the opportunity.

The walkthroughs are the three paths of the design, `best` (committed for trial), `ashes` (charged, case weak) and `wrong_man` (the wrong person), and four of #216: `ran` (he ran), `stopped` (stopped at the door, then charged), `trust` (Hester's alibi, told) and `follow` (Hester's alibi, seen on the barge). Run them with `cargo run -p loremaster --no-default-features -- test examples/lock-keeper-slice/demo`. `crates/loremaster-runtime/tests/demo.rs` also compares their transcripts with `expected/demo_*.transcript` (`BLESS=1` writes them) and runs the ending checks over the whole night.

| Part | Files | Design step |
| --- | --- | --- |
| Nine rooms, the clock, the people, the coroner | `rooms/`, `things/`, `story.yaml`, `agendas/coroner.yaml` | 1 |
| The tool chest and the windlass under the rope (concealed), the coat and the notebook | `things/chest.yaml`, `things/windlass.yaml`, `things/coat.yaml`, `things/notebook.yaml` | 2 |
| Talks with Bert, Crane, Hester and Marsh | `dialogues/` | 3 |
| Agendas: Marsh, Hester, Bert, Crane | `agendas/` | 4 |
| The lock machine, the drain timer, the pages and the stove | `machines/lock.yaml`, `timers/`, `things/pages.yaml`, `things/stove.yaml` | 5 |
| The accusation, six ranked endings, the case notebook (quests) | `rules/accuse.yaml`, `endings/`, `machines/means.yaml`, `machines/opportunity.yaml`, `machines/motive.yaml` | 6 |
| Walkthroughs, hints, the ending checks | `tests/`, `closes` in `agendas/` | 7 |
| "He ran", `show` and `stop`, Hester's trust and alibi (#216) | `verbs/show.yaml`, `verbs/stop.yaml`, `things/casebook.yaml`, `rules/marsh.yaml`, `timers/marsh_flight.yaml`, `endings/ran.yaml`, `dialogues/hester.yaml`, `rules/hester.yaml`, `agendas/hester.yaml` | 4, 6 |

What changed from the first draft of the design, so that the ending checks can walk every state of the night (72,148 with these choices, 39,047 before #216; with 5-minute turns and the coroner at three it takes 432,000 states and a minute, too slow for the tests):

- A turn is 15 minutes, not 5, and the coroner comes at three, not at four.
- The coroner reads the case at three; with no case, the verdict is "accidental death". That ending ranks above a wrong accusation and above "released at dawn".
- Bert tells of the two lanterns in the same talk in which you say that Amos was a good man; there is no stored trust. Hester's trust is one stored fact, `hester.trusts`, not a counter: "I am sorry about your father" in the kitchen before eleven sets it, and she tells the truth in the same line. Her agenda reads it, and she stays in the kitchen. The kind word comes only before eleven: a talk that stays open holds back her agenda, and at her quarter past eleven step that multiplies the states.
- `search chest` finds the windlass and proves the means; `examine windlass` shows the dent.
- Marsh's suspicion is one fact, `marsh_fled`, not a counter (#216). The clues of the demo do not travel (the boots, the windlass in the chest, Amos's notebook in the coat), so the player shows them through the case notebook: `show notes to marsh` shows every clue in it, and two kinds of proof (means, opportunity, motive) are the two clues of the design. Shown one at a time, each clue would need its own fact, and the night had 221,497 states.
- "He ran" ranks 3, between "charged" and "accidental death": his flight says what the case could not. Its text names what the notebook held and what it lacked.
- Marsh makes for the door and is gone on the next turn: `stop marsh` works only then, in the room where he read the notebook. A flight to the towpath, with time to follow him, gave more states. Stopped, he is held in the parlour, and his agenda is over (each step reads `marsh_fled`): put back on his agenda, the steps that fell due while he ran would be skipped, which gave 107,582 states, not fewer.
- Hester's alibi is part of the opportunity: it clears Crane, so only Marsh had the chance. Her trust gives it in the kitchen; without trust, a player on the barge at midnight sees her with Crane (`rules/hester.yaml`).
- Not in the demo: `ask … about` (the parser has it since #169; the demo's questions are dialogue options). The demo stays at 15-minute turns, and the design follows it (#239): time bands in the explorer were rejected, because they are not exact and the ending checks would miss the drops that come from being too late.
