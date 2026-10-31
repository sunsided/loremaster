# Import report: Zork I: The Great Underground Empire

loremaster-zil makes this file. Do not edit it. Run the importer again to make it new.

## Summary

| Item | Count |
| --- | --- |
| Rooms | 110 |
| Room descriptions that are not imported | 1 |
| Exits | 315 |
| Exits from hints | 7 |
| Message exits (only show a message) | 37 |
| Other exits that are not imported | 0 |
| Things | 131 |
| Things in a different place | 18 |
| Room and object routines (not ported) | 130 |
| Interrupt routines (not ported) | 16 |

## Room and object routines

These routines contain the behaviour of the game: puzzles, characters and special responses. The importer does not port them. Write rules for them by hand in `rules/`.

| Owner | Routine |
| --- | --- |
| `west_of_house` | WEST-HOUSE |
| `stone_barrow` | STONE-BARROW-FCN |
| `east_of_house` | EAST-HOUSE |
| `forest_1` | FOREST-ROOM |
| `forest_2` | FOREST-ROOM |
| `forest_3` | FOREST-ROOM |
| `path` | FOREST-ROOM |
| `up_a_tree` | TREE-ROOM |
| `grating_clearing` | CLEARING-FCN |
| `clearing` | FOREST-ROOM |
| `kitchen` | KITCHEN-FCN |
| `living_room` | LIVING-ROOM-FCN |
| `cellar` | CELLAR-FCN |
| `troll_room` | TROLL-ROOM-F |
| `grating_room` | MAZE-11-FCN |
| `cyclops_room` | CYCLOPS-ROOM-FCN |
| `treasure_room` | TREASURE-ROOM-FCN |
| `reservoir_south` | RESERVOIR-SOUTH-FCN |
| `reservoir` | RESERVOIR-FCN |
| `reservoir_north` | RESERVOIR-NORTH-FCN |
| `mirror_room_1` | MIRROR-ROOM |
| `mirror_room_2` | MIRROR-ROOM |
| `tiny_cave` | CAVE2-ROOM |
| `deep_canyon` | DEEP-CANYON-F |
| `loud_room` | LOUD-ROOM-FCN |
| `entrance_to_hades` | LLD-ROOM |
| `dome_room` | DOME-ROOM-FCN |
| `torch_room` | TORCH-ROOM-FCN |
| `south_temple` | SOUTH-TEMPLE-FCN |
| `dam_room` | DAM-ROOM-FCN |
| `white_cliffs_north` | WHITE-CLIFFS-FUNCTION |
| `white_cliffs_south` | WHITE-CLIFFS-FUNCTION |
| `river_4` | RIVR4-ROOM |
| `aragain_falls` | FALLS-ROOM |
| `canyon_view` | CANYON-VIEW-F |
| `bat_room` | BATS-ROOM |
| `gas_room` | BOOM-ROOM |
| `timber_room` | NO-OBJS |
| `lower_shaft` | NO-OBJS |
| `machine_room` | MACHINE-ROOM-FCN |
| `board` | BOARD-F |
| `teeth` | TEETH-F |
| `granite_wall` | GRANITE-WALL-F |
| `songbird` | SONGBIRD-F |
| `white_house` | WHITE-HOUSE-F |
| `forest` | FOREST-F |
| `mountain_range` | MOUNTAIN-RANGE-F |
| `global_water` | WATER-F |
| `water` | WATER-F |
| `kitchen_window` | KITCHEN-WINDOW-F |
| `chimney` | CHIMNEY-F |
| `ghosts` | GHOSTS-F |
| `lowered_basket` | BASKET-F |
| `raised_basket` | BASKET-F |
| `bat` | BAT-F |
| `bell` | BELL-F |
| `hot_bell` | HOT-BELL-F |
| `axe` | AXE-F |
| `bolt` | BOLT-F |
| `bubble` | BUBBLE-F |
| `book` | BLACK-BOOK |
| `sceptre` | SCEPTRE-FUNCTION |
| `slide` | SLIDE-FUNCTION |
| `sandwich_bag` | SANDWICH-BAG-FCN |
| `tool_chest` | TOOL-CHEST-FCN |
| `yellow_button` | BUTTON-F |
| `brown_button` | BUTTON-F |
| `red_button` | BUTTON-F |
| `blue_button` | BUTTON-F |
| `trophy_case` | TROPHY-CASE-FCN |
| `rug` | RUG-FCN |
| `chalice` | CHALICE-FCN |
| `garlic` | GARLIC-F |
| `cyclops` | CYCLOPS-FCN |
| `dam` | DAM-FUNCTION |
| `trap_door` | TRAP-DOOR-FCN |
| `boarded_window` | BOARDED-WINDOW-FCN |
| `front_door` | FRONT-DOOR-FCN |
| `barrow_door` | BARROW-DOOR-FCN |
| `barrow` | BARROW-FCN |
| `bottle` | BOTTLE-FUNCTION |
| `crack` | CRACK-FCN |
| `grate` | GRATE-FUNCTION |
| `knife` | KNIFE-F |
| `bones` | SKELETON |
| `bag_of_coins` | BAG-OF-COINS-F |
| `lamp` | LANTERN |
| `leak` | LEAK-FUNCTION |
| `machine` | MACHINE-F |
| `inflated_boat` | RBOAT-FUNCTION |
| `mailbox` | MAILBOX-F |
| `match` | MATCH-FUNCTION |
| `mirror_2` | MIRROR-MIRROR |
| `mirror_1` | MIRROR-MIRROR |
| `painting` | PAINTING-FCN |
| `candles` | CANDLES-FCN |
| `gunk` | GUNK-FUNCTION |
| `bodies` | BODY-FUNCTION |
| `leaves` | LEAF-PILE |
| `punctured_boat` | DBOAT-FUNCTION |
| `inflatable_boat` | IBOAT-FUNCTION |
| `rainbow` | RAINBOW-FCN |
| `river` | RIVER-FUNCTION |
| `buoy` | TREASURE-INSIDE |
| `rope` | ROPE-FUNCTION |
| `rusty_knife` | RUSTY-KNIFE-FCN |
| `sand` | SAND-FUNCTION |
| `large_bag` | LARGE-BAG-F |
| `stiletto` | STILETTO-FUNCTION |
| `machine_switch` | MSWITCH-FUNCTION |
| `wooden_door` | FRONT-DOOR-FCN |
| `sword` | SWORD-FCN |
| `thief` | ROBBER-FUNCTION |
| `pedestal` | DUMB-CONTAINER |
| `torch` | TORCH-OBJECT |
| `troll` | TROLL-FCN |
| `trunk` | TRUNK-F |
| `tube` | TUBE-FUNCTION |
| `putty` | PUTTY-FCN |
| `climbable_cliff` | CLIFF-OBJECT |
| `white_cliff` | WCLIF-OBJECT |
| `egg` | EGG-OBJECT |
| `canary` | CANARY-OBJECT |
| `broken_canary` | CANARY-OBJECT |
| `stairs` | STAIRS-F |
| `sailor` | SAILOR-FCN |
| `ground` | GROUND-FUNCTION |
| `grue` | GRUE-FUNCTION |
| `pathobj` | PATH-OBJECT |
| `zorkmid` | ZORKMID-FUNCTION |

## Room descriptions that are not imported

The original prints these descriptions from code that the importer can not translate. The room has a placeholder text.

| Room | Reason |
| --- | --- |
| `up_a_tree` | TREE-ROOM: condition `<SET F <FIRST? ,PATH>>` |

## Message exits

In the original, these directions show a message and do not move the player. The importer makes them as message exits: exits with `blocked` and no `to`.

| Room | Direction and message |
| --- | --- |
| `west_of_house` | east: "The door is boarded and you can't remove the boards." |
| `north_of_house` | south: "The windows are all boarded." |
| `south_of_house` | north: "The windows are all boarded." |
| `forest_1` | up: "There is no tree here suitable for climbing." |
| `forest_1` | west: "You would need a machete to go further west." |
| `forest_2` | up: "There is no tree here suitable for climbing." |
| `forest_2` | north: "The forest becomes impenetrable to the north." |
| `mountains` | up: "The mountains are impassable." |
| `mountains` | east: "The mountains are impassable." |
| `forest_3` | up: "There is no tree here suitable for climbing." |
| `forest_3` | east: "The rank undergrowth prevents eastward movement." |
| `forest_3` | south: "Storm-tossed trees block your way." |
| `up_a_tree` | up: "You cannot climb any higher." |
| `grating_clearing` | north: "The forest becomes impenetrable to the north." |
| `clearing` | up: "There is no tree here suitable for climbing." |
| `cellar` | west: "You try to ascend the ramp, but it is impossible, and you slide back down." |
| `east_of_chasm` | down: "The chasm probably leads straight to the infernal regions." |
| `reservoir` | down: "The dam blocks your way." |
| `stream_view` | west: "The stream emerges from a spot too small for you to enter." |
| `in_stream` | up: "The channel is too narrow." |
| `in_stream` | west: "The channel is too narrow." |
| `damp_cave` | south: "It is too narrow for most insects." |
| `chasm_room` | down: "Are you out of your mind?" |
| `torch_room` | up: "You cannot reach the rope." |
| `river_1` | up: "You cannot go upstream due to strong currents." |
| `river_1` | east: "The White Cliffs prevent your landing here." |
| `river_2` | up: "You cannot go upstream due to strong currents." |
| `river_2` | land: "There is no safe landing spot here." |
| `river_2` | east: "The White Cliffs prevent your landing here." |
| `river_2` | west: "Just in time you steer away from the rocks." |
| `river_3` | up: "You cannot go upstream due to strong currents." |
| `river_4` | up: "You cannot go upstream due to strong currents." |
| `river_4` | land: "You can land either to the east or the west." |
| `river_5` | up: "You cannot go upstream due to strong currents." |
| `aragain_falls` | down: "It's a long way..." |
| `canyon_view` | south: "Storm-tossed trees block your way." |
| `shaft_room` | down: "You wouldn't fit and would die if you could." |

## Exits that are not imported

The importer can not convert these exits.

None.

## Exits from hints

A routine decides these exits in the original. The importer uses hints from `hints.rs` for them.

| Room | Exit |
| --- | --- |
| `grating_clearing` | down: PER GRATING-EXIT, to grating_room |
| `living_room` | down: PER TRAP-DOOR-EXIT, to cellar |
| `studio` | up: PER UP-CHIMNEY-FUNCTION, to kitchen |
| `maze_2` | down: PER MAZE-DIODES, to maze_4 |
| `maze_7` | down: PER MAZE-DIODES, to dead_end_1 |
| `maze_9` | down: PER MAZE-DIODES, to maze_11 |
| `maze_12` | down: PER MAZE-DIODES, to maze_5 |

## Things in a different place

These things are inside an object that is not a container or a surface, are global objects, or are not visible in the original.

| Thing | Note |
| --- | --- |
| `teeth` | global object in every room; placed nowhere |
| `wall` | global object in every room; placed nowhere |
| `granite_wall` | global object in every room; placed nowhere |
| `axe` | inside troll, which is not a container; placed in the room troll_room |
| `leak` | invisible at the start; placed nowhere until a rule moves it |
| `pot_of_gold` | invisible at the start; placed nowhere until a rule moves it |
| `scarab` | invisible at the start; placed nowhere until a rule moves it |
| `map` | invisible at the start; placed nowhere until a rule moves it |
| `thief` | invisible at the start; placed nowhere until a rule moves it |
| `trunk` | invisible at the start; placed nowhere until a rule moves it |
| `blessings` | global object in every room; placed nowhere |
| `sailor` | global object in every room; placed nowhere |
| `ground` | global object in every room; placed nowhere |
| `grue` | global object in every room; placed nowhere |
| `lungs` | global object in every room; placed nowhere |
| `pathobj` | global object in every room; placed nowhere |
| `zorkmid` | global object in every room; placed nowhere |
| `hands` | global object in every room; placed nowhere |

## Interrupt routines

These routines run on a timer or each turn: the lamp battery, the thief, combat and others. The importer does not port them.

- `I-CANDLES`
- `I-CURE`
- `I-CYCLOPS`
- `I-FIGHT`
- `I-FOREST-ROOM`
- `I-LANTERN`
- `I-MAINT-ROOM`
- `I-MATCH`
- `I-REMPTY`
- `I-RFILL`
- `I-RIVER`
- `I-SWORD`
- `I-THIEF`
- `I-XB`
- `I-XBH`
- `I-XC`
