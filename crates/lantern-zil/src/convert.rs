//! Conversion of ZIL rooms and objects to lantern rooms and things.

use crate::hints;
use crate::out::{ExitOut, Frag, Initial, RoomOut, StoryOut, ThingOut, VarOut};
use crate::reader::Node;
use crate::source::{Entity, Source};
use crate::translate::{Translator, id, show, text};
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Something that the importer did not convert, or converted only in part.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// The lantern id of the room or thing.
    pub owner: String,
    pub detail: String,
}

/// What the importer did not convert, by kind.
#[derive(Debug, Default)]
pub struct Report {
    /// Room and object routines (`ACTION`). Their behaviour is not ported.
    pub routines: Vec<Note>,
    /// Room descriptions in routines that the importer could not translate.
    pub descriptions: Vec<Note>,
    /// Exits that only print a message, such as "The door is boarded".
    pub message_exits: Vec<Note>,
    /// Exits that the importer could not convert.
    pub exits: Vec<Note>,
    /// Exits that the importer converted from a hint, not from the source.
    pub hinted_exits: Vec<Note>,
    /// Things that are not in the same place as in the original.
    pub placement: Vec<Note>,
    /// Interrupt routines (`I-...`): timers and characters that act on their own.
    pub daemons: Vec<String>,
    /// Source files that the reader could not read.
    pub errors: Vec<Note>,
}

/// The converted world.
pub struct World {
    pub story: StoryOut,
    pub rooms: Vec<RoomOut>,
    pub things: Vec<ThingOut>,
    pub report: Report,
}

/// Convert a game. `rules` is the text of hand-written rule files: the ZIL flags that it names
/// become story variables too.
pub fn convert(src: &Source, rules: &str) -> World {
    let mut t = Translator::new(src);
    for name in src.globals.keys() {
        if t.is_flag(name) && mentions(rules, &id(name)) {
            t.flag(name);
        }
    }
    let mut report = Report {
        errors: src
            .errors
            .iter()
            .map(|(file, e)| Note {
                owner: file.clone(),
                detail: e.clone(),
            })
            .collect(),
        ..Default::default()
    };

    let mut rooms: Vec<RoomOut> = src
        .rooms
        .iter()
        .map(|r| room(r, &mut t, &mut report))
        .collect();

    let doors: HashSet<String> = rooms
        .iter()
        .flat_map(|r| r.exits.iter().filter_map(|e| e.door.clone()))
        .collect();
    let things: Vec<ThingOut> = src
        .objects
        .iter()
        .filter(|o| !hints::PSEUDO_OBJECTS.contains(&o.name.as_str()))
        .filter_map(|o| thing(o, src, &doors, &mut report))
        .collect();

    let start = src
        .object("ADVENTURER")
        .and_then(|a| a.atom("IN"))
        .filter(|r| src.room(r).is_some())
        .or_else(|| src.rooms.first().map(|r| r.name.as_str()))
        .unwrap_or_default();
    let positions = layout(&rooms, &id(start));
    for room in &mut rooms {
        room.position = positions.get(&room.id).copied().unwrap_or_default();
    }

    report.daemons = src
        .routines
        .keys()
        .filter(|n| n.starts_with("I-"))
        .cloned()
        .collect();

    let mut vars: Vec<VarOut> = t
        .flags
        .iter()
        .map(|name| VarOut {
            id: id(name),
            initial: Initial::Bool(
                matches!(src.globals.get(name), Some(Node::Atom(a)) if a == "T"),
            ),
        })
        .collect();
    vars.extend(t.numbers.iter().map(|name| VarOut {
        id: id(name),
        initial: Initial::Int(match src.globals.get(name) {
            Some(Node::Int(i)) => *i,
            _ => 0,
        }),
    }));
    vars.sort_by(|a, b| a.id.cmp(&b.id));
    let story = StoryOut {
        title: hints::title(start)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Imported ZIL game ({})", start.to_lowercase())),
        author: "Tim Anderson, Marc Blank, Bruce Daniels and Dave Lebling (Infocom). Imported from ZIL with lantern-zil.".into(),
        start: id(start),
        intro: Vec::new(),
        vars,
    };
    World {
        story,
        rooms,
        things,
        report,
    }
}

/// Does `text` contain `word` as a whole identifier?
fn mentions(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

fn origin(e: &Entity) -> String {
    format!("{} ({})", e.name, e.file)
}

fn room(r: &Entity, t: &mut Translator, report: &mut Report) -> RoomOut {
    let rid = id(&r.name);
    let name = r.text("DESC").unwrap_or(&r.name).to_string();
    let mut tags = Vec::new();
    if !r.has_flag("ONBIT") {
        tags.push("dark".to_string());
    }

    let action = r.atom("ACTION");
    let description = if let Some(ldesc) = r.text("LDESC") {
        vec![Frag::plain(text(ldesc).trim())]
    } else {
        let looked = match action.and_then(|a| t.src.routines.get(a)) {
            Some(routine) => t.look(routine),
            None => Err("the room has no description".into()),
        };
        looked.unwrap_or_else(|reason| {
            report.descriptions.push(Note {
                owner: rid.clone(),
                detail: format!("{}: {reason}", action.unwrap_or("no routine")),
            });
            vec![Frag::plain(format!(
                "[{name}: the original prints this description from the routine {}. It is not imported yet.]",
                action.unwrap_or("(none)")
            ))]
        })
    };
    if let Some(action) = action {
        report.routines.push(Note {
            owner: rid.clone(),
            detail: action.to_string(),
        });
    }

    let exits = r
        .props
        .iter()
        .filter(|(p, _)| t.src.is_direction(p))
        // `(IN ROOMS)` is where the room is, not an exit.
        .filter(|(_, args)| !matches!(args.as_slice(), [Node::Atom(_)]))
        .filter_map(|(dir, args)| exit(&rid, &r.name, dir, args, t, report))
        .collect();

    RoomOut {
        id: rid,
        origin: origin(r),
        name,
        tags,
        position: (0, 0),
        description,
        exits,
    }
}

fn direction(zil: &str) -> String {
    match zil {
        "NE" => "northeast".into(),
        "NW" => "northwest".into(),
        "SE" => "southeast".into(),
        "SW" => "southwest".into(),
        other => other.to_lowercase(),
    }
}

/// Convert one `(DIRECTION ...)` property of a room.
fn exit(
    rid: &str,
    room: &str,
    dir: &str,
    args: &[Node],
    t: &mut Translator,
    report: &mut Report,
) -> Option<ExitOut> {
    let direction = direction(dir);
    let note = |detail: String| Note {
        owner: rid.to_string(),
        detail: format!("{direction}: {detail}"),
    };
    let words: Vec<Option<&str>> = args.iter().map(Node::atom).collect();
    match (words.as_slice(), args) {
        // (EAST "The door is boarded.") and (EAST SORRY "...")
        (_, [Node::Str(msg)]) | ([Some("SORRY"), _], [_, Node::Str(msg)]) => {
            let msg = text(msg).trim().to_string();
            report.message_exits.push(note(format!("\"{msg}\"")));
            Some(ExitOut {
                direction: direction.clone(),
                blocked: Some(msg),
                ..Default::default()
            })
        }
        ([Some("PER"), Some(routine)], _) => match hints::per_exit(room, routine) {
            Some(mut e) => {
                for flag in hints::per_exit_flags(routine) {
                    t.flag(flag);
                }
                e.direction = direction.clone();
                report.hinted_exits.push(note(format!(
                    "PER {routine}, to {}",
                    e.to.as_deref().unwrap_or("-")
                )));
                Some(e)
            }
            None => {
                report.exits.push(note(format!("PER {routine}")));
                None
            }
        },
        ([Some("TO"), Some(to), rest @ ..], _) if t.src.room(to).is_some() => {
            let mut e = ExitOut {
                direction: direction.clone(),
                to: Some(id(to)),
                ..Default::default()
            };
            let cond = &args[2..];
            // `ELSE "message"` at the end.
            let (cond, blocked) = match cond {
                [head @ .., els, Node::Str(msg)] if els.is_atom("ELSE") => {
                    (head, Some(text(msg).trim().to_string()))
                }
                _ => (cond, None),
            };
            match (rest.len(), cond) {
                (0, []) => {}
                (_, [iff, obj, is, open])
                    if iff.is_atom("IF") && is.is_atom("IS") && open.is_atom("OPEN") =>
                {
                    let door = obj.atom().filter(|o| t.src.object(o).is_some());
                    match door {
                        Some(door) => e.door = Some(id(door)),
                        None => {
                            report.exits.push(note(format!("door {}", show(obj))));
                            return None;
                        }
                    }
                }
                (_, [iff, flag]) if iff.is_atom("IF") => {
                    match flag.atom().and_then(|f| t.flag(f)) {
                        // With a message the exit is known but blocked; without one it is hidden.
                        Some(var) if blocked.is_some() => e.passable = Some(var),
                        Some(var) => e.visible = Some(var),
                        None => {
                            report.exits.push(note(format!("condition {}", show(flag))));
                            return None;
                        }
                    }
                }
                _ => {
                    report.exits.push(note(format!(
                        "condition {}",
                        cond.iter().map(show).collect::<Vec<_>>().join(" ")
                    )));
                    return None;
                }
            }
            if e.passable.is_some() {
                e.blocked = blocked;
            }
            Some(e)
        }
        _ => {
            let shown: Vec<String> = args.iter().map(show).collect();
            report.exits.push(note(shown.join(" ")));
            None
        }
    }
}

/// Where a ZIL object is, as a lantern room, or `None` for nowhere.
fn place(o: &Entity, src: &Source, report: &mut Report) -> Option<String> {
    let oid = id(&o.name);
    let mut note = |detail: String| {
        report.placement.push(Note {
            owner: oid.clone(),
            detail,
        })
    };
    let mut chain: Vec<String> = Vec::new();
    let mut here = o.atom("IN")?;
    for _ in 0..8 {
        if let Some(room) = src.room(here) {
            if !chain.is_empty() {
                note(format!(
                    "inside {}; placed in the room {}",
                    chain.join(" in "),
                    id(&room.name)
                ));
            }
            return Some(id(&room.name));
        }
        match here {
            "LOCAL-GLOBALS" => {
                let rooms: Vec<&Entity> = src
                    .rooms
                    .iter()
                    .filter(|r| r.atoms("GLOBAL").contains(&o.name.as_str()))
                    .collect();
                let first = rooms.first()?;
                if rooms.len() > 1 {
                    let others: Vec<String> = rooms[1..].iter().map(|r| id(&r.name)).collect();
                    note(format!(
                        "scenery of {} rooms; placed in {} only (also in {})",
                        rooms.len(),
                        id(&first.name),
                        others.join(", ")
                    ));
                }
                return Some(id(&first.name));
            }
            "GLOBAL-OBJECTS" => {
                note("global object in every room; placed nowhere".into());
                return None;
            }
            _ => {}
        }
        let Some(holder) = src.object(here) else {
            note(format!("in {here}, which is not a room or object"));
            return None;
        };
        chain.push(id(&holder.name));
        here = holder.atom("IN")?;
    }
    note(format!(
        "inside {}: too deep, or a loop; placed nowhere",
        chain.join(" in ")
    ));
    None
}

fn thing(
    o: &Entity,
    src: &Source,
    doors: &HashSet<String>,
    report: &mut Report,
) -> Option<ThingOut> {
    let oid = id(&o.name);
    let name = o.text("DESC")?.to_string();
    let door = doors.contains(&oid) || o.has_flag("DOORBIT");

    let mut room = place(o, src, report);
    if o.has_flag("INVISIBLE") && !door {
        report.placement.push(Note {
            owner: oid.clone(),
            detail: "invisible at the start; placed nowhere until a rule moves it".into(),
        });
        room = None;
    }

    let mut aliases: Vec<String> = Vec::new();
    let lower = name.to_lowercase();
    for word in o.atoms("SYNONYM") {
        let word = word.to_lowercase();
        if word != lower && !aliases.contains(&word) {
            aliases.push(word);
        }
    }

    let mut props = Vec::new();
    let mut key = None;
    if o.has_flag("CONTBIT") || door {
        props.push(("open".to_string(), o.has_flag("OPENBIT")));
    }
    if let Some((_, locked, k)) = hints::LOCKS.iter().find(|(d, ..)| *d == o.name) {
        props.push(("locked".to_string(), *locked));
        key = k.map(id);
    }
    if o.has_flag("LIGHTBIT") {
        props.push(("lit".to_string(), o.has_flag("ONBIT")));
    }

    let description = o
        .text("TEXT")
        .map(|s| vec![Frag::plain(text(s).trim())])
        .unwrap_or_default();

    let untouched = format!("{oid}.interactions == 0");
    let touched = format!("{oid}.interactions > 0");
    let presence = match (o.text("FDESC"), o.text("LDESC")) {
        (Some(f), Some(l)) => vec![
            Frag::when(untouched, text(f).trim()),
            Frag::when(touched, text(l).trim()),
        ],
        (Some(f), None) => vec![
            Frag::when(untouched, text(f).trim()),
            Frag::when(touched, format!("There is {{{oid}.a}} here.")),
        ],
        (None, Some(l)) => vec![Frag::plain(text(l).trim())],
        (None, None) => Vec::new(),
    };

    if let Some(action) = o.atom("ACTION") {
        report.routines.push(Note {
            owner: oid.clone(),
            detail: action.to_string(),
        });
    }

    Some(ThingOut {
        id: oid,
        origin: origin(o),
        name,
        aliases,
        room,
        portable: o.has_flag("TAKEBIT"),
        key,
        listed: !o.has_flag("NDESCBIT") && !door,
        props,
        description,
        presence,
    })
}

/// Grid steps for a direction. `y` grows to the south.
fn step(direction: &str) -> (i32, i32) {
    match direction {
        "north" => (0, -1),
        "south" => (0, 1),
        "east" => (1, 0),
        "west" => (-1, 0),
        "northeast" => (1, -1),
        "northwest" => (-1, -1),
        "southeast" => (1, 1),
        "southwest" => (-1, 1),
        "up" => (1, -2),
        "down" => (-1, 2),
        "in" => (2, 1),
        "out" => (-2, -1),
        _ => (2, 2),
    }
}

/// Map positions: a walk from the start room that puts each room one step in the direction of
/// the exit, or in the nearest free cell. ZIL maps are not planar, so this is only a start.
fn layout(rooms: &[RoomOut], start: &str) -> BTreeMap<String, (i32, i32)> {
    let mut pos: BTreeMap<String, (i32, i32)> = BTreeMap::new();
    let mut used: HashSet<(i32, i32)> = HashSet::new();
    let by_id: BTreeMap<&str, &RoomOut> = rooms.iter().map(|r| (r.id.as_str(), r)).collect();

    let mut seeds: Vec<&str> = vec![start];
    seeds.extend(rooms.iter().map(|r| r.id.as_str()));
    for seed in seeds {
        if pos.contains_key(seed) || !by_id.contains_key(seed) {
            continue;
        }
        let right = used.iter().map(|(x, _)| *x).max().map_or(0, |x| x + 3);
        let p = free(&used, (right, 0));
        pos.insert(seed.to_string(), p);
        used.insert(p);
        let mut queue = VecDeque::from([seed]);
        while let Some(from) = queue.pop_front() {
            let (fx, fy) = pos[from];
            for (e, to) in by_id[from]
                .exits
                .iter()
                .filter_map(|e| Some((e, e.to.as_deref()?)))
            {
                if pos.contains_key(to) || !by_id.contains_key(to) {
                    continue;
                }
                let (dx, dy) = step(&e.direction);
                let p = free(&used, (fx + dx, fy + dy));
                pos.insert(to.to_string(), p);
                used.insert(p);
                queue.push_back(by_id[to].id.as_str());
            }
        }
    }
    pos
}

/// The free cell nearest to `want`, in rings around it.
fn free(used: &HashSet<(i32, i32)>, want: (i32, i32)) -> (i32, i32) {
    for r in 0..i32::MAX {
        for dy in -r..=r {
            for dx in -r..=r {
                let p = (want.0 + dx, want.1 + dy);
                if dx.abs().max(dy.abs()) == r && !used.contains(&p) {
                    return p;
                }
            }
        }
    }
    want
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn world(src: &str) -> World {
        let mut s = Source::default();
        s.add_file("test.zil", src);
        convert(&s, "")
    }

    const MAP: &str = r#"
<DIRECTIONS NORTH EAST WEST SOUTH UP DOWN>
<GLOBAL WON-FLAG <>>
<GLOBAL TROLL-FLAG <>>
<OBJECT ADVENTURER (IN FIELD)>
<OBJECT HOUSE (IN LOCAL-GLOBALS) (DESC "white house") (SYNONYM HOUSE HOME) (FLAGS NDESCBIT)>
<OBJECT DOOR (IN LOCAL-GLOBALS) (DESC "door") (FLAGS DOORBIT)>
<OBJECT BOX (IN FIELD) (DESC "box") (FLAGS CONTBIT)>
<OBJECT GEM (IN BOX) (DESC "gem") (FDESC "A gem glitters.") (FLAGS TAKEBIT)>
<ROOM FIELD (DESC "Field") (LDESC "A field.") (FLAGS ONBIT)
  (NORTH TO HALL IF DOOR IS OPEN)
  (EAST "The house is in the way.")
  (WEST TO HALL IF WON-FLAG)
  (SOUTH TO HALL IF TROLL-FLAG ELSE "The troll stops you.")
  (GLOBAL HOUSE DOOR)>
<ROOM HALL (DESC "Hall") (LDESC "A hall.") (SOUTH TO FIELD) (GLOBAL HOUSE)>
"#;

    #[test]
    fn converts_rooms_and_exits() {
        let w = world(MAP);
        check!(w.story.start == "field");
        let field = &w.rooms[0];
        check!(field.tags.is_empty());
        check!(w.rooms[1].tags == vec!["dark".to_string()]);
        check!(field.exits.len() == 4);
        check!(field.exits[0].door.as_deref() == Some("door"));
        check!(field.exits[1].to.is_none());
        check!(field.exits[1].blocked.as_deref() == Some("The house is in the way."));
        check!(field.exits[2].visible.as_deref() == Some("won_flag"));
        check!(field.exits[3].passable.as_deref() == Some("troll_flag"));
        check!(field.exits[3].blocked.as_deref() == Some("The troll stops you."));
        check!(w.report.message_exits.len() == 1);
        check!(w.report.exits.is_empty());
        check!(w.story.vars.len() == 2);
        check!(w.rooms[1].position == (0, -1));
    }

    #[test]
    fn converts_things() {
        let w = world(MAP);
        let house = w.things.iter().find(|t| t.id == "house").unwrap();
        check!(house.room.as_deref() == Some("field"));
        check!(house.aliases == vec!["house".to_string(), "home".to_string()]);
        check!(!house.listed);
        let gem = w.things.iter().find(|t| t.id == "gem").unwrap();
        check!(gem.room.as_deref() == Some("field"));
        check!(gem.presence[0].when.as_deref() == Some("gem.interactions == 0"));
        let door = w.things.iter().find(|t| t.id == "door").unwrap();
        check!(door.props == vec![("open".to_string(), false)]);
        check!(w.report.placement.iter().any(|n| n.owner == "gem"));
        check!(w.report.placement.iter().any(|n| n.owner == "house"));
    }

    #[test]
    fn drops_exits_with_untranslatable_conditions() {
        let w = world(
            r#"<DIRECTIONS NORTH>
<GLOBAL COUNT 3>
<ROOM A (DESC "A") (NORTH TO B IF COUNT)>
<ROOM B (DESC "B")>"#,
        );
        check!(w.rooms[0].exits.is_empty());
        check!(w.report.exits.len() == 1);
    }

    #[test]
    fn declares_flags_that_rules_use() {
        let mut s = Source::default();
        s.add_file("test.zil", "<GLOBAL RUG-MOVED <>> <GLOBAL RUG <>>");
        let w = convert(&s, "when: not rug_moved");
        check!(w.story.vars.len() == 1);
        check!(w.story.vars[0].id == "rug_moved");
    }
}
