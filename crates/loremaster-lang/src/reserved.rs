//! Names that the condition language keeps for itself.

/// The names that conditions read as built-ins before they look at the
/// story: keywords, values that the runtime derives, the starts of built-in
/// paths, and functions.
///
/// A variable, room or thing with a keyword's id cannot be named in a
/// condition; with a derived value's id, conditions read the built-in instead
/// of it (`resolve_path` in `lower.rs` resolves built-ins first). A function
/// name is a built-in only in call position (`carried(x)`), and a path start
/// only with a dot (`time.hour`); both are reserved for the built-ins and for
/// syntax to come. [`reserved`] tells the four kinds apart. The checker reports such an id at its declaration, and the ZIL
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
    // Built-in paths: `time.hour`, `action.count`, `timer.fuse`.
    "time",
    "action",
    "timer",
    // The rule events of a combat: `combat_hit.damage`.
    "combat_hit",
    "combat_round",
    "combat_end",
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
    // One source of truth with the kinds: the checker asks both.
    reserved(name).is_some()
}

/// Why a name is reserved, in the words of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reserved {
    /// `and`, `or`, `not`, `true`, `false`: a condition cannot name such an id at all.
    Keyword,
    /// `here`, `turns`, …: a bare name reads the built-in, not the story's.
    Derived,
    /// `time`, `action`, `timer`: built-in paths with a dot (`time.hour`). A
    /// bare name still reads the story's; the name is kept for the paths.
    Path,
    /// `carried`, `chance`, …: a built-in only in call position, `carried(x)`.
    /// A bare name still reads the story's; the name is kept for the function
    /// and for syntax to come.
    Function,
}

/// The kind of a reserved name, or None when the name is free.
pub fn reserved(name: &str) -> Option<Reserved> {
    // By the name itself, so a change of the list's order cannot change a kind.
    Some(match name {
        "and" | "or" | "not" | "true" | "false" => Reserved::Keyword,
        "here" | "turns" | "level" | "first_visit" => Reserved::Derived,
        "time" | "action" | "timer" | "combat_hit" | "combat_round" | "combat_end" => {
            Reserved::Path
        }
        "in" | "carried" | "present" | "visited" | "visits" | "chance" | "remembers"
        | "seen_by" => Reserved::Function,
        _ => return None,
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
        assert_eq!(reserved("first_visit"), Some(Reserved::Derived));
        assert_eq!(reserved("timer"), Some(Reserved::Path));
        assert_eq!(reserved("in"), Some(Reserved::Function));
        assert_eq!(reserved("seen_by"), Some(Reserved::Function));
        assert_eq!(reserved("lamp"), None);
    }

    #[test]
    fn the_list_and_the_kinds_agree() {
        for name in RESERVED_NAMES {
            assert!(reserved(name).is_some(), "{name} has no kind");
        }
        // A kind without a list entry: the list is what the ZIL importer and
        // the docs use, so it must name every reserved word.
        for probe in ["and", "here", "time", "seen_by", "level", "visits"] {
            assert_eq!(reserved(probe).is_some(), RESERVED_NAMES.contains(&probe));
        }
        let kinds = [
            "and",
            "or",
            "not",
            "true",
            "false",
            "here",
            "turns",
            "level",
            "first_visit",
            "time",
            "action",
            "timer",
            "combat_hit",
            "combat_round",
            "combat_end",
            "in",
            "carried",
            "present",
            "visited",
            "visits",
            "chance",
            "remembers",
            "seen_by",
        ];
        // The same names, in any order: the kinds no longer depend on it.
        let (mut kinds, mut names) = (kinds.to_vec(), RESERVED_NAMES.to_vec());
        kinds.sort_unstable();
        names.sort_unstable();
        assert_eq!(kinds, names);
    }

    #[test]
    fn the_list_has_no_duplicates() {
        let mut names = RESERVED_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), RESERVED_NAMES.len());
    }
}
