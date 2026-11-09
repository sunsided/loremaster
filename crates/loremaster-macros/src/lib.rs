//! The `world!` macro (DSL steps L3 and L4, DS-4): a story pack in Rust, in a
//! grammar close to the YAML files. Use it through the crate `loremaster`
//! with its feature `dsl`: `use loremaster::prelude::*;`.
//!
//! A `world!` block makes the function `pub fn pack() -> Pack`; a block
//! that starts with `pack <name>;` makes `pub fn <name>() -> Pack`
//! instead, so two blocks can share a module. The
//! function calls the builder API of loremaster-dsl, so the pack has the
//! same model values as the builder and the YAML loader make. Each entity
//! gets an `Origin::Rust` with the file, line and column of its id in the
//! block.
//!
//! At compile time, the macro builds the same pack and runs the schema
//! checks on it (`loremaster_project::check_packs`): unknown ids, reserved
//! names, conditions and markup. An error is a compile error at the tokens
//! it names. Ids of other packs are declared in `extern { .. }`; the checks
//! take them as given, and the checks of the joined packs find them at load
//! time.
//!
//! ```
//! use loremaster::prelude::*;
//!
//! world! {
//!     extern {                 // ids of other packs
//!         ending drowned;
//!         room kitchen;
//!         thing windlass;
//!     }
//!
//!     story "The Lock" {
//!         start: chamber,
//!         seed: 11,
//!         clock { start: "22:30", minutes_per_turn: 5 },
//!     }
//!
//!     var lock_open: bool = false;
//!
//!     room chamber "Lock Chamber" {
//!         at [1, 0];
//!         "Stone walls drop into the lock.";
//!         ambient cycle when night => ["Water knocks.", "A rope creaks."];
//!         exit west -> kitchen;
//!     }
//!
//!     thing paddles "lower paddles" in chamber {
//!         aliases [paddles, "lower paddles"];
//!         unlisted;
//!         prop open = false;
//!         examine "Two iron paddles in the lower gate.";
//!     }
//!
//!     rule jump on verb(jump) instead when in(chamber) and not lock_open {
//!         line "The water closes over you.";
//!         end drowned;
//!     }
//!
//!     machine lock starts full {
//!         state full "The chamber is full.";
//!         state empty;
//!         full -> empty on verb(open, paddles) instead when windlass.carried {
//!             line "Water roars out.";   // the first line is the step's `say`
//!             start drain;
//!         }
//!     }
//!
//!     timer drain turns 3;
//!
//!     ending free "Free" { kind win; rank 1; "You walk out at dawn."; }
//!
//!     verb jump "Jump" { words [jump, "jump in"]; slots none; no_bar; }
//!
//!     test drowning "Jump into the full lock" {
//!         pending phase 4, needs "P4-07 #84";
//!         wait * 3;
//!         jump sees "The water closes over you" expect ended;
//!         expect ending drowned, room == chamber, lock == full;
//!     }
//! }
//!
//! let p: Pack = pack();
//! assert_eq!(p.rooms[0].id.as_str(), "chamber");
//! ```
//!
//! A number variable takes limits, `var stamina = 10 min 0 max stamina_initial;`.
//! The effect `calc total = r + skill - 1;` sets a number from a sum. A test
//! takes `seed 42;`, and a step takes `rolls 4, 11` for the totals of its
//! rolls.
//!
//! `await` waits for the host: `await roll "2d6" into r against luck;`,
//! `await choice [left, right "the right door"] into path;`,
//! `await walk_to dock;` or
//! `await action pick_lock { difficulty = 3 } results [open, broken] into lock;`.
//! `combat [warden, hound] win yard loss fallen escape road together;` is a
//! fight (`escape` and `together` are optional). The story header takes the
//! adventure sheet, `stats { stamina "STAMINA" short "ST" max stamina_initial, gold "Gold" }`.
//!
//! ```
//! use loremaster::prelude::*;
//!
//! world! {
//!     story "Gate" { start: gate, stats { stamina "STAMINA" short "ST" max 12, luck "LUCK" } }
//!     var skill = 9;
//!     var stamina = 12 min 0 max 12;
//!     var luck = 9;
//!     var r = 0;
//!     room gate "Gate" {}
//!     room yard "Yard" {}
//!     room fallen "Fallen" {}
//!     thing warden "warden" in gate { prop skill = 7; prop stamina = 6; }
//!     rule fight on enter(gate) {
//!         await roll "2d6" into r against luck;
//!         combat [warden] win yard loss fallen;
//!     }
//! }
//! Loremaster::new().pack(pack()).build().unwrap();
//! ```
//!
//! A `line` is one string, or speech segments (P3-15, #111): bare strings
//! and `narrate "…"` are narration, `says "…"` and `keeper says "…"` are
//! speech, and `when <cond> then <segment>` shows the one segment after
//! `then` only while the condition holds (a bare string there is
//! narration). A bare string right after `says` reads like more speech, so
//! it is a compile error that suggests `narrate`; YAML loads it with a
//! warning.
//!
//! ```
//! use loremaster::prelude::*;
//!
//! world! {
//!     extern { thing keeper; room chamber; }
//!     rule accuse on turn {
//!         line "The lock-keeper shrugs." keeper says "Prove it," narrate "he says."
//!             when in chamber then "Water drips.";
//!     }
//! }
//! # let _ = pack();
//! ```
//!
//! A dialogue is dramaturge's token form of `dialogue!`
//! (`dramaturge_syntax::tokens`), with the conditions, lines and effects of
//! `world!` as its payloads. A node is `<key>: <line> { .. }`; its body has
//! `when <cond>;`, the outcomes and the options. An option is
//! `<line> [when <cond>] => <node | end | back>;`, or with a body
//! `{ id <key>; <outcomes> }`. The outcomes are dramaturge's `set`, `add`,
//! `relation` and `remember`, and `then <effect>, ..` for the effects of
//! `world!`. The speech of a node is the thing's, of an option the
//! player's. Without an id, the dialogue has the id of its thing. The
//! checks of dramaturge and of loremaster point at the tokens they name.
//!
//! ```
//! use loremaster::prelude::*;
//!
//! world! {
//!     pack cantina;
//!     story "Cantina" { start: deck, relations: [trust] }
//!     var knows_name: bool = false;
//!     room deck "Deck" {}
//!     thing blackbeard "Blackbeard" in deck {}
//!
//!     dialogue blackbeard_talk with blackbeard start [meet] {
//!         meet: "The figure looks up." says "Sit, if you like." {
//!             says "Who are you?" => names;
//!             says "Not tonight." => end { then goto deck; }
//!         }
//!         names: says "They call me Blackbeard." {
//!             set knows_name = true;
//!             relation trust += 1;
//!             says "Where did the *Marigold* go?" when blackbeard.trust >= 1 => back {
//!                 remember asked_marigold;
//!             }
//!         }
//!     }
//! }
//! # let _ = cantina();
//! ```
//!
//! An option can not start with a segment condition (`when .. then ..`):
//! there, `when` is the node's condition. More lines of a node are
//! `<speaker>: "…";` in its body (the speaker is a thing or `player`), and
//! `-> <node>` (or `-> end`, `-> back`) is where the talk goes on when the
//! node has no answer to choose (`next`).
//!
//! Also in the grammar:
//!
//! - `thread <id> "<name>" { status open; summary "…"; tracks <fact>; note "…"; }`
//!   and `behavior <id>;` (the code comes from elsewhere).
//! - In a room or a thing, `before <verb> <text>;`, `instead ..` and
//!   `after ..`: the frames around the response to a verb. In a thing,
//!   `name [when <cond> =>] "<text>" [aliases [..]] [proper];`: a stage of
//!   its name.
//! - In the story header, `relations: [trust]`, `score: <max>`,
//!   `score_var: <var>` and `ranks { 0 "Beginner", 26 "Amateur" }`.
//! - After the text of a fragment, `id <name>`, `replaces` and `note "…"`.
//! - `note "…"` and `todo "…"` in machines and timers.
//! - `replace` before a room, thing, machine, timer, ending, verb, dialogue
//!   or thread: it replaces the entity of an earlier pack.
//! - `extern { thing stove [hot: bool, fuel: int]; relation trust; }`: the
//!   properties of an extern thing (`bool`, `int` or `text`), so that
//!   conditions on them check, and a relation of another pack's header.
//!
//! Rust behaviors (DSL step L5). All three forms make a normal `Behavior`:
//!
//! - `behavior <expr>;` (anything but a lone lowercase id, for example
//!   `behavior Thief::zork();`): the pack keeps the instance, and
//!   `Loremaster::build` names and registers it. The macro does not know its
//!   id, so a `call` to it in the same block needs
//!   `extern { behavior <id>; }`.
//! - `#[behavior(on = turn)]` on a function: see [`macro@behavior`].
//! - `rust |cx| { .. }` as an effect in a rule or a machine step, one per
//!   rule or step: the macro makes the behavior `<rule>#rust` (`<machine>#<n>#rust`
//!   for a step) and puts `call: <rule>#rust` at the place of the block. The
//!   block's last expression gives a `Flow` or `()`; an early `return` gives
//!   a `Flow`.
//!
//! Rust code follows the determinism rules of the session: change the world
//! only through `Cx`, take random numbers only from `Cx::roll`, and use no
//! clock and no host state. The closure of a `rust` block can not capture
//! local variables; the macro can not check the rest.
//!
//! ```
//! use loremaster::prelude::*;
//!
//! world! {
//!     story "Scale" { start: hall }
//!     room hall "Hall" {}
//!     verb weigh "Weigh" { words [weigh]; slots none; }
//!     rule weigh on verb(weigh) instead {
//!         line "The scale tips.";
//!         rust |cx| { cx.say(format!("Turn {}.", cx.turns())); }
//!     }
//! }
//! let world = Loremaster::new().pack(pack()).build().unwrap();
//! assert!(world.missing_behaviors().is_empty());
//! ```
//!
//! Not in the grammar yet: text pools and `one_of` groups (R9), agendas
//! (D6), `extend`, a sealed mode `world!(yaml = "..")` that checks against
//! a YAML folder, and `uses <pack>;` for the ids of another pack. The macro
//! names the step or issue that brings each one it knows.

mod ast;
mod behavior;
mod check;
mod cond;
mod emit;
mod parse;

use proc_macro::TokenStream;

/// A story pack in Rust: `world! { room .. thing .. rule .. }` makes
/// `pub fn pack() -> Pack`. Read the crate docs for the grammar.
#[proc_macro]
pub fn world(input: TokenStream) -> TokenStream {
    let world = syn::parse_macro_input!(input as ast::World);
    if let Err(e) = check::validate(&world).and_then(|()| check::check(&world)) {
        return e.to_compile_error().into();
    }
    emit::expand(&world).into()
}

/// A behavior from a function: the macro keeps the function and adds a unit
/// struct (`parrot` makes `Parrot`) with a `Behavior` impl. The id is the
/// function name, `description()` is the doc comment, and `on` is the event
/// filter of the manifest: `on = turn`, `on = [turn, enter]`, or nothing for
/// every event. A `call` effect runs it whatever the filter says. It takes
/// `cx: &mut Cx<'_>` and returns a `Flow` or `()`. The attribute registers
/// nothing: name the struct in a block with `behavior Parrot;`.
///
/// ```
/// use loremaster::prelude::*;
///
/// /// The parrot squawks now and then.
/// #[behavior(on = turn)]
/// fn parrot(cx: &mut Cx<'_>) {
///     if cx.roll(4) == 0 {
///         cx.say("\"Pieces of eight!\" squawks the parrot.");
///     }
/// }
///
/// assert_eq!(Parrot.id(), "parrot");
/// assert_eq!(Parrot.description(), "The parrot squawks now and then.");
/// ```
#[proc_macro_attribute]
pub fn behavior(attr: TokenStream, item: TokenStream) -> TokenStream {
    behavior::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
