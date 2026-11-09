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
- People keep their own hours (`demo/agendas/`): Marsh cleans his boots at eleven and goes to the lock at one, Bert goes to sleep at half past twelve.

The walkthroughs are the three paths of the design: `best` (committed for trial), `ashes` (charged, case weak) and `wrong_man` (the wrong person). Run them with `cargo run -p loremaster --no-default-features -- test examples/lock-keeper-slice/demo`. `crates/loremaster-runtime/tests/demo.rs` also compares their transcripts with `expected/demo_*.transcript` (`BLESS=1` writes them) and runs the ending checks over the whole night.

| Part | Files | Design step |
| --- | --- | --- |
| Nine rooms, the clock, the people, the coroner | `rooms/`, `things/`, `story.yaml`, `agendas/coroner.yaml` | 1 |
| The tool chest and the windlass under the rope (concealed), the coat and the notebook | `things/chest.yaml`, `things/windlass.yaml`, `things/coat.yaml`, `things/notebook.yaml` | 2 |
| Talks with Bert, Crane, Hester and Marsh | `dialogues/` | 3 |
| Agendas: Marsh, Hester, Bert, Crane | `agendas/` | 4 |
| The lock machine, the drain timer, the pages and the stove | `machines/lock.yaml`, `timers/`, `things/pages.yaml`, `things/stove.yaml` | 5 |
| The accusation, five ranked endings, the case notebook (quests) | `rules/accuse.yaml`, `endings/`, `machines/means.yaml`, `machines/opportunity.yaml`, `machines/motive.yaml` | 6 |
| Walkthroughs, hints, the ending checks | `tests/`, `closes` in `agendas/` | 7 |

What changed from the first draft of the design, so that the ending checks can walk every state of the night (39,047 with these choices; with 5-minute turns and the coroner at three it takes 432,000 states and a minute, too slow for the tests):

- A turn is 15 minutes, not 5, and the coroner comes at three, not at four.
- The coroner reads the case at three; with no case, the verdict is "accidental death". That ending ranks above a wrong accusation and above "released at dawn".
- Bert tells of the two lanterns in the same talk in which you say that Amos was a good man; there is no stored trust.
- `search chest` finds the windlass and proves the means; `examine windlass` shows the dent.
- Not in the demo: `ask … about` and `show … to` (the parser has them since #169; the demo's questions are dialogue options), Marsh's flight ("He ran"), Hester's alibi and her trust (#216). The demo stays at 15-minute turns, and the design follows it (#239): time bands in the explorer were rejected, because they are not exact and the ending checks would miss the drops that come from being too late.
