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
    let cleaned: String = input
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();
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

/// Longest verb phrase at the start of the input.
fn match_verb<'a>(verbs: &'a [VerbDef], words: &[&str]) -> Option<(&'a VerbDef, Vec<String>)> {
    let mut best: Option<(&VerbDef, usize)> = None;
    for verb in verbs {
        for phrase in &verb.words {
            let pw: Vec<&str> = phrase.split_whitespace().collect();
            if words.len() >= pw.len()
                && words[..pw.len()] == pw[..]
                && best.is_none_or(|(_, n)| pw.len() > n)
            {
                best = Some((verb, pw.len()));
            }
        }
    }
    best.map(|(v, n)| (v, words[n..].iter().map(|w| w.to_string()).collect()))
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

fn resolve(scope: &ParseScope, words: &[String]) -> Result<Id, String> {
    let phrase = words.join(" ");
    let score = |noun: &str| -> u8 {
        if noun == phrase {
            3
        } else if noun.ends_with(&format!(" {phrase}")) {
            2
        } else if words
            .iter()
            .all(|w| noun.split_whitespace().any(|n| n == w))
        {
            1
        } else {
            0
        }
    };
    let mut best = 0;
    let mut matches: Vec<&Id> = Vec::new();
    for (id, nouns) in &scope.things {
        let s = nouns.iter().map(|n| score(n)).max().unwrap_or(0);
        if s == 0 || s < best {
            continue;
        }
        if s > best {
            best = s;
            matches.clear();
        }
        if !matches.contains(&id) {
            matches.push(id);
        }
    }
    match matches.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!("You see no {phrase} here.")),
        many => {
            let names: Vec<String> = many
                .iter()
                .filter_map(|id| scope.things.iter().find(|(t, _)| t == *id))
                .map(|(_, nouns)| format!("the {}", nouns[0]))
                .collect();
            Err(format!(
                "Which do you mean, {}?",
                crate::text::list(&names, "or")
            ))
        }
    }
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
