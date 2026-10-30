# Import report: Zork I: The Great Underground Empire

lantern-zil makes this file. Do not edit it. Run the importer again to make it new.

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
| Things in a different place | 54 |
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

A lantern thing is in one room, or nowhere. These things are in a container, in more than one room, or not visible in the original.

| Thing | Note |
| --- | --- |
| `board` | scenery of 3 rooms; placed in west_of_house only (also in north_of_house, south_of_house) |
| `teeth` | global object in every room; placed nowhere |
| `wall` | global object in every room; placed nowhere |
| `granite_wall` | global object in every room; placed nowhere |
| `songbird` | scenery of 6 rooms; placed in forest_1 only (also in forest_2, forest_3, path, up_a_tree, clearing) |
| `white_house` | scenery of 12 rooms; placed in west_of_house only (also in north_of_house, south_of_house, east_of_house, forest_1, forest_2, mountains, forest_3, path, up_a_tree, grating_clearing, clearing) |
| `forest` | scenery of 10 rooms; placed in west_of_house only (also in north_of_house, south_of_house, east_of_house, forest_1, forest_2, forest_3, path, up_a_tree, clearing) |
| `tree` | scenery of 7 rooms; placed in forest_1 only (also in forest_2, mountains, forest_3, path, up_a_tree, clearing) |
| `global_water` | scenery of 19 rooms; placed in reservoir_south only (also in reservoir, reservoir_north, stream_view, in_stream, dam_room, dam_base, river_1, river_2, river_3, white_cliffs_north, white_cliffs_south, river_4, river_5, shore, sandy_beach, aragain_falls, end_of_rainbow, canyon_bottom) |
| `water` | inside bottle in kitchen_table; placed in the room kitchen |
| `kitchen_window` | scenery of 2 rooms; placed in east_of_house only (also in kitchen) |
| `chimney` | scenery of 2 rooms; placed in kitchen only (also in studio) |
| `lunch` | inside sandwich_bag in kitchen_table; placed in the room kitchen |
| `axe` | inside troll; placed in the room troll_room |
| `book` | inside altar; placed in the room south_temple |
| `sceptre` | inside coffin; placed in the room egypt_room |
| `slide` | scenery of 2 rooms; placed in cellar only (also in slide_room) |
| `sandwich_bag` | inside kitchen_table; placed in the room kitchen |
| `garlic` | inside sandwich_bag in kitchen_table; placed in the room kitchen |
| `boarded_window` | scenery of 2 rooms; placed in north_of_house only (also in south_of_house) |
| `bottle` | inside kitchen_table; placed in the room kitchen |
| `crack` | scenery of 2 rooms; placed in damp_cave only (also in chasm_room) |
| `grate` | scenery of 2 rooms; placed in grating_clearing only (also in grating_room) |
| `knife` | inside attic_table; placed in the room attic |
| `emerald` | inside buoy; placed in the room river_4 |
| `advertisement` | inside mailbox; placed in the room west_of_house |
| `leak` | invisible at the start; placed nowhere until a rule moves it |
| `bodies` | scenery of 2 rooms; placed in entrance_to_hades only (also in land_of_living_dead) |
| `pot_of_gold` | invisible at the start; placed nowhere until a rule moves it |
| `rainbow` | scenery of 4 rooms; placed in aragain_falls only (also in on_rainbow, end_of_rainbow, canyon_view) |
| `river` | scenery of 15 rooms; placed in dam_base only (also in river_1, river_2, river_3, white_cliffs_north, white_cliffs_south, river_4, river_5, shore, sandy_beach, aragain_falls, end_of_rainbow, canyon_bottom, cliff_middle, canyon_view) |
| `ladder` | scenery of 2 rooms; placed in ladder_top only (also in ladder_bottom) |
| `scarab` | invisible at the start; placed nowhere until a rule moves it |
| `large_bag` | inside thief; placed in the room round_room |
| `stiletto` | inside thief; placed in the room round_room |
| `map` | inside trophy_case; placed in the room living_room |
| `map` | invisible at the start; placed nowhere until a rule moves it |
| `thief` | invisible at the start; placed nowhere until a rule moves it |
| `torch` | inside pedestal; placed in the room torch_room |
| `trunk` | invisible at the start; placed nowhere until a rule moves it |
| `putty` | inside tube; placed in the room maintenance_room |
| `climbable_cliff` | scenery of 3 rooms; placed in canyon_bottom only (also in cliff_middle, canyon_view) |
| `white_cliff` | scenery of 2 rooms; placed in white_cliffs_north only (also in white_cliffs_south) |
| `egg` | inside nest; placed in the room up_a_tree |
| `canary` | inside egg in nest; placed in the room up_a_tree |
| `blessings` | global object in every room; placed nowhere |
| `stairs` | scenery of 20 rooms; placed in kitchen only (also in attic, living_room, cellar, cyclops_room, treasure_room, reservoir_north, small_cave, tiny_cave, atlantis_room, ew_passage, deep_canyon, loud_room, chasm_room, egypt_room, torch_room, north_temple, smelly_room, gas_room, ladder_top) |
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
