//! Names that the condition language keeps for itself.

/// The names that conditions read as built-ins before they look at the
/// story: keywords, values that the runtime derives, and functions.
///
/// A variable, room or thing with a keyword's id cannot be named in a
/// condition; with a derived value's id, conditions read the built-in instead
/// of it (`resolve_path` in `lower.rs` resolves built-ins first). A function
/// name is a built-in only in call position (`carried(x)`); it is reserved
/// for the function and for syntax to come. [`reserved`] tells the three
/// apart. The checker reports such an id at its declaration, and the ZIL
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

/// Why a name is reserved, in the words of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reserved {
    /// `and`, `or`, `not`, `true`, `false`: a condition cannot name such an id at all.
    Keyword,
    /// `here`, `turns`, …: a bare name reads the built-in, not the story's.
    Derived,
    /// `carried`, `chance`, …: a built-in only in call position, `carried(x)`.
    /// A bare name still reads the story's; the name is kept for the function
    /// and for syntax to come.
    Function,
}

/// The kind of a reserved name, or None when the name is free.
pub fn reserved(name: &str) -> Option<Reserved> {
    let at = RESERVED_NAMES.iter().position(|n| *n == name)?;
    Some(match at {
        0..5 => Reserved::Keyword,
        5..12 => Reserved::Derived,
        _ => Reserved::Function,
    })
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
    fn each_name_has_its_kind() {
        assert_eq!(reserved("not"), Some(Reserved::Keyword));
        assert_eq!(reserved("false"), Some(Reserved::Keyword));
        assert_eq!(reserved("here"), Some(Reserved::Derived));
        assert_eq!(reserved("timer"), Some(Reserved::Derived));
        assert_eq!(reserved("in"), Some(Reserved::Function));
        assert_eq!(reserved("seen_by"), Some(Reserved::Function));
        assert_eq!(reserved("lamp"), None);
    }

    #[test]
    fn the_list_has_no_duplicates() {
        let mut names = RESERVED_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), RESERVED_NAMES.len());
    }
}
