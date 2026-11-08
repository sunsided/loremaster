//! The checks of the host contract (IF-2): the actions and host events that
//! the story uses against its `interface:` section, and the arguments that
//! walkthroughs give (IF-6). A story without the section skips them.

use super::Checker;
use crate::Severity;
use crate::bindings::{pascal, type_names};
use loremaster_model::{ArgType, AwaitSpec, EventPattern, Id, Interface, Walkthrough, check_args};
use std::collections::{BTreeMap, BTreeSet};

/// The properties that every thing has: an event argument with one of these
/// names would read the property, not the argument.
const THING_PROPERTIES: &[&str] = &[
    "name",
    "the",
    "The",
    "a",
    "A",
    "carried",
    "present",
    "location",
    "interactions",
    "talked",
];

/// What the other checks saw on their way: `(file, at, ...)`.
#[derive(Debug, Default)]
pub(super) struct Seen {
    pub awaits: Vec<(String, String, AwaitSpec)>,
    pub emits: Vec<(String, String, Id)>,
    /// Every condition and markup source, to look for a branch.
    pub tests: Vec<String>,
}

impl Checker<'_> {
    pub(super) fn contract(&mut self) {
        let Some(interface) = self.story().interface.clone() else {
            return;
        };
        self.declarations(&interface);
        self.calls(&interface);
        self.listeners(&interface);
    }

    fn declarations(&mut self, interface: &Interface) {
        let header = self.file_of("story");
        let actions = interface
            .actions
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>();
        self.variants(&header, "interface.actions", "actions", &actions);
        let events = interface
            .events
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>();
        self.variants(&header, "interface.events", "events", &events);
        for (i, a) in interface.actions.iter().enumerate() {
            let at = format!("interface.actions[{i}]");
            let results = a.results.iter().map(Id::as_str).collect::<Vec<_>>();
            self.variants(&header, &at, "results", &results);
            self.args(&header, &at, &a.args);
        }
        let mut types: BTreeMap<String, String> = BTreeMap::new();
        for (name, what) in type_names(interface) {
            if let Some(first) = types.get(&name) {
                self.error(
                    &header,
                    "interface",
                    format!("{first} and {what} are both the Rust type `{name}`; rename one"),
                );
            } else {
                types.insert(name, what);
            }
        }
        for (i, e) in interface.events.iter().enumerate() {
            let at = format!("interface.events[{i}]");
            self.args(&header, &at, &e.args);
            for arg in e.args.keys() {
                if THING_PROPERTIES.contains(&arg.as_str())
                    || self.story().relations.iter().any(|r| r.as_str() == arg)
                {
                    self.error(&header, &format!("{at}.args.{arg}"), format!("`{}.{arg}` would read the property `{arg}`, not the argument; rename it", e.id));
                }
            }
            // `event.arg` reads like `thing.prop`: an id that conditions
            // already read another way would hide the arguments.
            if self.story().thing(e.id.as_str()).is_some()
                || self.story().var(e.id.as_str()).is_some()
                || self.story().room(e.id.as_str()).is_some()
                || loremaster_lang::is_reserved(e.id.as_str())
            {
                self.error(&header, &at, format!("the event `{}` has the id of a thing, a variable, a room or a built-in name; its arguments would clash", e.id));
            }
            if let Some(when) = &e.when {
                self.condition(&header, &format!("{at}.when"), when);
            }
        }
    }

    /// The ids of one Rust enum: plain names, each its own variant.
    fn variants(&mut self, file: &str, at: &str, what: &str, ids: &[&str]) {
        let mut seen: BTreeMap<String, &str> = BTreeMap::new();
        for id in ids {
            if !plain(id) {
                self.error(file, at, not_plain(id));
            }
            match seen.insert(pascal(id), id) {
                Some(first) if first == *id => {
                    self.error(file, at, format!("two {what} have the id `{id}`"))
                }
                Some(first) => self.error(
                    file,
                    at,
                    format!(
                        "`{first}` and `{id}` are both the Rust variant `{}`; rename one",
                        pascal(id)
                    ),
                ),
                None => {}
            }
        }
    }

    /// The arguments of an action or an event: plain names, and plain enum values.
    fn args(&mut self, file: &str, at: &str, args: &BTreeMap<String, ArgType>) {
        for (arg, ty) in args {
            let at = format!("{at}.args.{arg}");
            if !plain(arg) {
                self.error(file, &at, not_plain(arg));
            }
            if let ArgType::Enum(values) = ty {
                if values.is_empty() {
                    self.error(file, &at, "an enum needs at least one value");
                }
                let values: Vec<&str> = values.iter().map(String::as_str).collect();
                self.variants(file, &at, "values", &values);
            }
        }
    }

    /// Each `await` of an action: declared, with fitting arguments, the
    /// declared results, and a branch for each result.
    fn calls(&mut self, interface: &Interface) {
        let awaits = std::mem::take(&mut self.contract.awaits);
        for (file, at, spec) in &awaits {
            let Some(id) = &spec.action else { continue };
            let Some(action) = interface.action(id.as_str()) else {
                self.error(
                    file,
                    at,
                    format!("the action `{id}` is not in `interface:`"),
                );
                continue;
            };
            if let Err(why) = check_args(&format!("the action `{id}`"), &action.args, &spec.args) {
                self.error(file, at, why);
            }
            let given: BTreeSet<_> = spec.results.iter().collect();
            let declared: BTreeSet<_> = action.results.iter().collect();
            if given != declared {
                self.error(
                    file,
                    at,
                    format!("the `results` of the action `{id}` are not the declared ones"),
                );
                continue;
            }
            if action.results.len() < 2 {
                continue;
            }
            let Some(into) = &spec.into else {
                self.error(
                    file,
                    at,
                    format!(
                        "the action `{id}` has several results; give `into` and branch on them"
                    ),
                );
                continue;
            };
            for result in &action.results {
                if !self
                    .contract
                    .tests
                    .iter()
                    .any(|t| names_both(t, into, result.as_str()))
                {
                    self.error(file, at, format!("no condition handles the result `{result}` of `{id}` (for example `{into} == {result}`)"));
                }
            }
        }
    }

    /// An event that the story both sends and hears is its own. An event
    /// with only one side must be a declared host event: that catches a typo
    /// on either side.
    fn listeners(&mut self, interface: &Interface) {
        let emits = std::mem::take(&mut self.contract.emits);
        let listens: Vec<_> = (self.story().rules.iter())
            .filter_map(|r| match &r.on {
                EventPattern::Event(e) => Some((r.id.clone(), e.clone())),
                _ => None,
            })
            .collect();
        let sent: BTreeSet<_> = emits.iter().map(|(_, _, e)| e.clone()).collect();
        let heard: BTreeSet<_> = listens.iter().map(|(_, e)| e.clone()).collect();
        for (file, at, e) in &emits {
            match interface.event(e.as_str()) {
                None if !heard.contains(e) => self.error(
                    file,
                    at,
                    format!("the event `{e}` is not in `interface:` and no rule listens for it"),
                ),
                Some(decl) if !decl.args.is_empty() => self.error(
                    file,
                    at,
                    format!("`emit` gives no arguments, and the event `{e}` needs some"),
                ),
                _ => {}
            }
        }
        for (rule, e) in listens {
            if interface.event(e.as_str()).is_none() && !sent.contains(&e) {
                let file = self.file_of(&format!("rule:{rule}"));
                self.error(
                    &file,
                    "on",
                    format!(
                        "the rule listens for `{e}`, which is not in `interface:` and nothing emits"
                    ),
                );
            }
        }
    }

    /// A walkthrough step that reports a host event (IF-6).
    pub(super) fn event_steps(&mut self, key: &str, walk: &Walkthrough) {
        for (i, step) in walk.steps.iter().enumerate() {
            let Some(plan) = step.plan() else { continue };
            let Some(event) = &plan.event else {
                if step.command().is_empty() {
                    self.at_step(key, i, Severity::Error, "a step needs `do` or `event`");
                }
                if !plan.args.is_empty() {
                    self.at_step(key, i, Severity::Error, "`args` belong to an `event`");
                }
                continue;
            };
            // Without `do`, the command is the text `event <id>`.
            if step.command() != format!("event {event}") {
                self.at_step(
                    key,
                    i,
                    Severity::Error,
                    "a step has `do` or `event`, not both",
                );
            }
            let found = self
                .story()
                .interface
                .as_ref()
                .and_then(|c| c.event(event.as_str()))
                .cloned();
            match found {
                None => self.at_step(
                    key,
                    i,
                    Severity::Error,
                    format!("the event `{event}` is not in `interface:`"),
                ),
                Some(decl) => {
                    if let Err(why) =
                        check_args(&format!("the event `{event}`"), &decl.args, &plan.args)
                    {
                        self.at_step(key, i, Severity::Error, why);
                    }
                }
            }
        }
    }
}

/// A name of the host contract becomes a Rust, TypeScript and JSON name:
/// lowercase letters, digits and single `_`, from a letter on.
fn plain(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !name.contains("__")
        && !name.ends_with('_')
}

fn not_plain(name: &str) -> String {
    format!(
        "`{name}` is no plain name: use lowercase letters, digits and single `_`, from a letter on"
    )
}

// shortcut: looks for the two words anywhere in one condition; a real branch
// analysis needs the compiled conditions, upgrade if false passes appear.
fn names_both(source: &str, into: &str, result: &str) -> bool {
    let mut words = source.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'));
    let (mut a, mut b) = (false, false);
    for w in &mut words {
        a |= w == into;
        b |= w == result;
    }
    a && b
}
