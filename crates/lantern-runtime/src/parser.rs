use lantern_model::{Direction, Id, VerbDef, VerbSlots};
use serde::{Deserialize, Serialize};

/// A resolved player action. The parser produces these from typed text; the
/// verb bar produces them from clicks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Command {
    Verb { verb: Id, objects: Vec<Id> },
    Go { direction: String },
}

/// What the parser can refer to right now.
pub(crate) struct ParseScope {
    /// Thing id and the nouns that name it.
    pub things: Vec<(Id, Vec<String>)>,
    /// Exit direction name and the name of the room it leads to.
    pub exits: Vec<(String, String)>,
}

const ARTICLES: [&str; 6] = ["the", "a", "an", "some", "my", "this"];
const PREPOSITIONS: [&str; 7] = ["with", "on", "to", "in", "into", "at", "using"];

pub(crate) fn parse(verbs: &[VerbDef], scope: &ParseScope, input: &str) -> Result<Command, String> {
    let cleaned = clean(input);
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        return Err("Say something.".into());
    }
    let phrase = words.join(" ");

    if let Some(dir) = direction(scope, &phrase) {
        return Ok(Command::Go { direction: dir });
    }

    let (verb, rest) = match_verb(verbs, &words)
        .ok_or_else(|| format!("I don't know how to \"{}\".", words[0]))?;

    if verb.id.as_str() == "go" {
        let target = strip_articles(&rest).join(" ");
        return direction(scope, &target)
            .map(|direction| Command::Go { direction })
            .ok_or_else(|| "You can't go that way.".to_string());
    }

    let (first, second) = split_objects(verb, &rest);
    let mut objects = Vec::new();
    match verb.slots {
        VerbSlots::None => {
            if !first.is_empty() {
                return Err(format!(
                    "\"{}\" doesn't need anything after it.",
                    verb.words[0]
                ));
            }
        }
        VerbSlots::One | VerbSlots::Two => {
            if first.is_empty() {
                return Err(format!("What do you want to {}?", verb.words[0]));
            }
            objects.push(resolve(scope, &first)?);
            if !second.is_empty() {
                if verb.slots == VerbSlots::One {
                    return Err(format!(
                        "I only understood you as far as \"{} {}\".",
                        verb.words[0],
                        first.join(" ")
                    ));
                }
                objects.push(resolve(scope, &second)?);
            }
        }
    }
    Ok(Command::Verb {
        verb: verb.id.clone(),
        objects,
    })
}

fn direction(scope: &ParseScope, phrase: &str) -> Option<String> {
    if phrase.is_empty() {
        return None;
    }
    let canonical = Direction::from_word(phrase).map(|d| d.name().to_string());
    let wanted = canonical.as_deref().unwrap_or(phrase);
    let by_room = |room: &String| {
        let room = room.to_lowercase();
        room == phrase || room.ends_with(&format!(" {phrase}"))
    };
    scope
        .exits
        .iter()
        .find(|(dir, _)| dir == wanted)
        .or_else(|| scope.exits.iter().find(|(_, room)| by_room(room)))
        .map(|(dir, _)| dir.clone())
        .or(canonical)
}

/// Longest verb phrase at the start of the input. When two verbs share the
/// phrase (`look` and `look` for examine), prefer the one whose slots fit:
/// a verb without objects when nothing follows, a verb with objects otherwise.
fn match_verb<'a>(verbs: &'a [VerbDef], words: &[&str]) -> Option<(&'a VerbDef, Vec<String>)> {
    match_verb_wanting(verbs, words, words.len() > 1)
}

fn match_verb_wanting<'a>(
    verbs: &'a [VerbDef],
    words: &[&str],
    wants_objects: bool,
) -> Option<(&'a VerbDef, Vec<String>)> {
    let mut best: Vec<(&VerbDef, usize)> = Vec::new();
    for verb in verbs {
        for phrase in &verb.words {
            let pw: Vec<&str> = phrase.split_whitespace().collect();
            if words.len() < pw.len() || words[..pw.len()] != pw[..] {
                continue;
            }
            match best.first() {
                Some((_, n)) if pw.len() < *n => {}
                Some((_, n)) if pw.len() == *n => best.push((verb, pw.len())),
                _ => best = vec![(verb, pw.len())],
            }
        }
    }
    let fits = |v: &VerbDef| (v.slots != VerbSlots::None) == wants_objects;
    let (verb, n) = best
        .iter()
        .find(|(v, _)| fits(v))
        .or(best.first())
        .copied()?;
    Some((verb, words[n..].iter().map(|w| w.to_string()).collect()))
}

fn split_objects(verb: &VerbDef, rest: &[String]) -> (Vec<String>, Vec<String>) {
    let is_prep = |w: &str| verb.preposition.as_deref() == Some(w) || PREPOSITIONS.contains(&w);
    match rest.iter().position(|w| is_prep(w)) {
        Some(i) if verb.slots == VerbSlots::Two => {
            (strip_articles(&rest[..i]), strip_articles(&rest[i + 1..]))
        }
        _ => (strip_articles(rest), Vec::new()),
    }
}

fn strip_articles(words: &[String]) -> Vec<String> {
    words
        .iter()
        .filter(|w| !ARTICLES.contains(&w.as_str()))
        .cloned()
        .collect()
}

/// The things that best match a noun phrase. With `prefix`, the last word may
/// be unfinished ("light" matches "lighthouse door").
fn candidates(scope: &ParseScope, words: &[String], prefix: bool) -> Vec<Id> {
    let phrase = words.join(" ");
    let word_ok =
        |i: usize, w: &str, n: &str| n == w || (prefix && i + 1 == words.len() && n.starts_with(w));
    let score = |noun: &str| -> u8 {
        if noun == phrase {
            3
        } else if noun.ends_with(&format!(" {phrase}")) {
            2
        } else if words
            .iter()
            .enumerate()
            .all(|(i, w)| noun.split_whitespace().any(|n| word_ok(i, w, n)))
        {
            1
        } else {
            0
        }
    };
    let mut best = 0;
    let mut matches: Vec<Id> = Vec::new();
    for (id, nouns) in &scope.things {
        let s = nouns.iter().map(|n| score(n)).max().unwrap_or(0);
        if s == 0 || s < best {
            continue;
        }
        if s > best {
            best = s;
            matches.clear();
        }
        if !matches.contains(id) {
            matches.push(id.clone());
        }
    }
    matches
}

fn resolve(scope: &ParseScope, words: &[String]) -> Result<Id, String> {
    let phrase = words.join(" ");
    let matches = candidates(scope, words, false);
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("You see no {phrase} here.")),
        many => {
            let names: Vec<String> = many
                .iter()
                .filter_map(|id| scope.things.iter().find(|(t, _)| t == id))
                .map(|(_, nouns)| format!("the {}", nouns[0]))
                .collect();
            Err(format!(
                "Which do you mean, {}?",
                crate::text::list(&names, "or")
            ))
        }
    }
}

/// What a command that is still being typed refers to so far.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    /// The verb, when the words so far name exactly one.
    pub verb: Option<Id>,
    /// Things the command names without doubt, in order.
    pub objects: Vec<Id>,
    /// Things the last, unfinished noun phrase could still mean.
    pub candidates: Vec<Id>,
    /// The exit, for a movement command.
    pub direction: Option<String>,
    /// True when the command can run as it is.
    pub complete: bool,
}

/// Read a partial command. A space at the end means the last word is finished.
pub(crate) fn preview(verbs: &[VerbDef], scope: &ParseScope, input: &str) -> Preview {
    let finished = input.ends_with(' ');
    let cleaned = clean(input);
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let mut out = Preview::default();
    if words.is_empty() {
        return out;
    }
    let phrase = words.join(" ");
    if let Some(dir) = direction(scope, &phrase).filter(|_| Direction::from_word(&phrase).is_some())
    {
        out.verb = Some(Id::new("go"));
        out.direction = Some(dir);
        out.complete = true;
        return out;
    }

    // "look " with a space: the player is about to name a thing, so prefer "look at".
    let wants_objects = words.len() > 1 || finished;
    let Some((verb, rest)) = match_verb_wanting(verbs, &words, wants_objects) else {
        // An unfinished first word: name the verb only if exactly one verb starts with it.
        if words.len() == 1 && !finished {
            let mut found: Vec<&VerbDef> = verbs
                .iter()
                .filter(|v| v.words.iter().any(|w| w.starts_with(words[0])))
                .collect();
            found.dedup_by(|a, b| a.id == b.id);
            if let [one] = found.as_slice() {
                out.verb = Some(one.id.clone());
            }
        }
        return out;
    };
    out.verb = Some(verb.id.clone());

    if verb.id.as_str() == "go" {
        let target = strip_articles(&rest).join(" ");
        out.direction = direction(scope, &target);
        out.complete = out.direction.is_some();
        return out;
    }

    let (first, second) = split_objects(verb, &rest);
    let has_second_slot = verb.slots == VerbSlots::Two && rest.len() > first.len();
    for (i, words) in [first, second].iter().enumerate() {
        if words.is_empty() {
            continue;
        }
        let last = i == 1 || !has_second_slot;
        let exact = candidates(scope, words, false);
        let found = if exact.is_empty() && last && !finished {
            candidates(scope, words, true)
        } else {
            exact
        };
        match found.as_slice() {
            [one] => out.objects.push(one.clone()),
            _ => out.candidates.extend(found),
        }
    }
    let wanted = match verb.slots {
        VerbSlots::None => 0,
        VerbSlots::One => 1,
        VerbSlots::Two => 1,
    };
    out.complete = out.candidates.is_empty()
        && out.objects.len() >= wanted
        && !(has_second_slot && out.objects.len() < 2);
    out
}

fn clean(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use lantern_model::standard_verbs;

    fn scope() -> ParseScope {
        ParseScope {
            things: vec![
                ("brass_key".into(), vec!["brass key".into(), "key".into()]),
                ("iron_key".into(), vec!["iron key".into(), "key".into()]),
                ("door".into(), vec!["oak door".into(), "door".into()]),
            ],
            exits: vec![("north".into(), "Keeper's Cottage".into())],
        }
    }

    fn verb(id: &str, objects: &[&str]) -> Command {
        Command::Verb {
            verb: id.into(),
            objects: objects.iter().map(|o| Id::new(*o)).collect(),
        }
    }

    #[test]
    fn commands() {
        let v = standard_verbs();
        let s = scope();
        check!(
            parse(&v, &s, "unlock the door with the brass key")
                == Ok(verb("unlock", &["door", "brass_key"]))
        );
        check!(parse(&v, &s, "pick up iron key") == Ok(verb("take", &["iron_key"])));
        check!(
            parse(&v, &s, "n")
                == Ok(Command::Go {
                    direction: "north".into()
                })
        );
        check!(
            parse(&v, &s, "go to the cottage")
                == Ok(Command::Go {
                    direction: "north".into()
                })
        );
        check!(parse(&v, &s, "i") == Ok(verb("inventory", &[])));
    }

    #[test]
    fn previews_follow_typing() {
        let v = standard_verbs();
        let s = scope();
        let p = preview(&v, &s, "look ");
        check!(p.verb == Some(Id::new("examine")));
        check!(!p.complete);
        let p = preview(&v, &s, "look oak");
        check!(p.objects == vec![Id::new("door")]);
        check!(p.complete);
        let p = preview(&v, &s, "take ke");
        check!(p.candidates.len() == 2);
        let p = preview(&v, &s, "unlock door with bra");
        check!(p.objects == vec![Id::new("door"), Id::new("brass_key")]);
        check!(p.complete);
        let p = preview(&v, &s, "unl");
        check!(p.verb == Some(Id::new("unlock")));
        check!(preview(&v, &s, "n").direction.as_deref() == Some("north"));
    }

    #[test]
    fn look_with_a_thing_examines_it() {
        let v = standard_verbs();
        check!(parse(&v, &scope(), "look oak door") == Ok(verb("examine", &["door"])));
        check!(parse(&v, &scope(), "look") == Ok(verb("look", &[])));
    }

    #[test]
    fn helpful_failures() {
        let v = standard_verbs();
        let s = scope();
        check!(
            parse(&v, &s, "take key")
                == Err("Which do you mean, the brass key or the iron key?".to_string())
        );
        check!(parse(&v, &s, "take lamp") == Err("You see no lamp here.".to_string()));
        check!(parse(&v, &s, "dance") == Err("I don't know how to \"dance\".".to_string()));
        check!(parse(&v, &s, "open") == Err("What do you want to open?".to_string()));
    }
}
