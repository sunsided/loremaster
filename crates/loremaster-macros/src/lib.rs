//! The `world!` macro (DSL step L3, DS-4): a story pack in Rust, in a
//! grammar close to the YAML files. Use it through the crate `loremaster`
//! with its feature `dsl`: `use loremaster::prelude::*;`.
//!
//! A `world!` block makes the function `pub fn pack() -> Pack`. The
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
//! Not in the grammar yet: dialogues (they come with dramaturge's
//! `dialogue!`), text pools, `one_of` groups, agendas and speech segments.
//! The macro names the step or issue that brings each one.

mod ast;
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
