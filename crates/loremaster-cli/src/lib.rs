//! loremaster as a library. The terminal player is the binary of this crate.
//!
//! With the cargo feature `dsl`, the crate gives the stories in Rust (D7):
//! the builder API of [`loremaster-dsl`](https://docs.rs/loremaster-dsl) and
//! the [`world!`] macro of [`loremaster-macros`](https://docs.rs/loremaster-macros),
//! both in [`prelude`]. Write a story in Rust, join it with YAML folders, and
//! build the world. The feature `builder` is the old name of `dsl`.
//!
//! ```toml
//! loremaster = { version = "0.1", default-features = false, features = ["dsl"] }
//! ```

/// The builder API: [`loremaster_dsl`]. The [`world!`] macro expands to
/// calls of it.
#[cfg(feature = "dsl")]
pub use loremaster_dsl as dsl;

#[cfg(feature = "dsl")]
pub use loremaster_macros::world;

/// Everything a story module needs: `use loremaster::prelude::*;`.
///
/// ```no_run
/// use loremaster::prelude::*;
///
/// let mut p = Pack::new("world");
/// p.story("The Lighthouse", "shore");
/// p.room("shore", "The Shore").text("Shingle crunches underfoot.");
/// let world = Loremaster::new().pack(p).yaml("story/").build();
/// ```
///
/// The same with the macro. `world!` makes the function `pack()`:
///
/// ```
/// use loremaster::prelude::*;
///
/// world! {
///     story "The Lighthouse" { start: shore }
///     room shore "The Shore" {
///         "Shingle crunches underfoot.";
///     }
/// }
///
/// let world = Loremaster::new().pack(pack()).build().unwrap();
/// assert_eq!(world.story.title, "The Lighthouse");
/// ```
#[cfg(feature = "dsl")]
pub mod prelude {
    pub use loremaster_dsl::prelude::*;
    pub use loremaster_macros::world;
}
