//! Names that the condition language keeps for itself.

/// The names that conditions read as built-ins before they look at the
/// story: keywords, values that the runtime derives, and functions.
///
/// A variable, room or thing with such an id loads, but conditions read the
/// built-in instead of it (`resolve_path` in `lower.rs` resolves built-ins
/// first). The checker reports such an id at its declaration, and the ZIL
/// importer renames such names. The list also holds names of planned
/// built-ins (`seen_by`), so that no story takes them now.
pub const RESERVED_NAMES: &[&str] = &[
    // Keywords and literals.
    "and",
    "or",
    "not",
    "true",
    "false",
    // Values that the runtime derives.
    "here",
    "turns",
    "level",
    "first_visit",
    "time",
    "action",
    "timer",
    // Functions.
    "in",
    "carried",
    "present",
    "visited",
    "visits",
    "chance",
    "remembers",
    "seen_by",
];

/// True for a name in [`RESERVED_NAMES`].
pub fn is_reserved(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scope, Ty, compile_condition};

    #[test]
    fn a_story_variable_with_a_reserved_name_is_not_what_conditions_read() {
        // Why the names are reserved: the built-in wins over the variable. A
        // flag named `turns` reads the number of turns, which is no flag.
        for name in ["here", "turns", "level"] {
            let mut scope = Scope::default();
            scope.vars.insert(name.into(), Ty::Bool);
            assert!(compile_condition(name, &scope).is_err(), "{name}");
        }
    }

    #[test]
    fn the_list_has_no_duplicates() {
        let mut names = RESERVED_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), RESERVED_NAMES.len());
    }
}
