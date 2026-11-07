//! loremaster as a library. The terminal player is the binary of this crate.
//!
//! With the cargo feature `builder`, the crate gives the builder API of
//! [`loremaster-dsl`](https://docs.rs/loremaster-dsl) in [`prelude`]: write a
//! story in plain Rust, join it with YAML folders, and build the world.
//!
//! ```toml
//! loremaster = { version = "0.1", default-features = false, features = ["builder"] }
//! ```

/// The builder API: [`loremaster_dsl`].
#[cfg(feature = "builder")]
pub use loremaster_dsl as dsl;

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
#[cfg(feature = "builder")]
pub mod prelude {
    pub use loremaster_dsl::prelude::*;
}
