//! The parts of a ZIL game that the importer reads: rooms, objects, globals and routines.

use crate::reader::{Node, read};
use std::collections::BTreeMap;

/// A `<ROOM ...>` or `<OBJECT ...>` definition.
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub props: Vec<(String, Vec<Node>)>,
    pub file: String,
}

impl Entity {
    pub fn prop(&self, name: &str) -> Option<&[Node]> {
        self.props
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, args)| args.as_slice())
    }

    /// The first string argument of a property, such as `DESC` or `LDESC`.
    pub fn text(&self, name: &str) -> Option<&str> {
        self.prop(name)?.first()?.string()
    }

    /// The first atom argument of a property, such as `IN` or `ACTION`.
    /// The first number argument of a property, such as `VALUE`.
    pub fn int(&self, name: &str) -> Option<i64> {
        match self.prop(name)?.first()? {
            Node::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn atom(&self, name: &str) -> Option<&str> {
        self.prop(name)?.first()?.atom()
    }

    /// All atom arguments of a property, such as `FLAGS` or `SYNONYM`.
    pub fn atoms(&self, name: &str) -> Vec<&str> {
        self.prop(name)
            .map(|args| args.iter().filter_map(Node::atom).collect())
            .unwrap_or_default()
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.atoms("FLAGS").contains(&flag)
    }
}

/// A `<ROUTINE NAME (params) body...>` definition.
#[derive(Debug, Clone)]
pub struct Routine {
    pub params: Vec<Node>,
    pub body: Vec<Node>,
    pub file: String,
}

/// Everything the importer reads from the source files of one game.
#[derive(Debug, Default)]
pub struct Source {
    pub rooms: Vec<Entity>,
    pub objects: Vec<Entity>,
    /// `<GLOBAL NAME value>`, first definition wins.
    pub globals: BTreeMap<String, Node>,
    pub routines: BTreeMap<String, Routine>,
    /// The words of `<DIRECTIONS ...>`.
    pub directions: Vec<String>,
    /// Files that could not be read, with the reason.
    pub errors: Vec<(String, String)>,
}

impl Source {
    /// Add the definitions of one source file.
    pub fn add_file(&mut self, file: &str, text: &str) {
        let nodes = match read(text) {
            Ok(n) => n,
            Err(e) => {
                self.errors.push((file.to_string(), e.to_string()));
                return;
            }
        };
        for node in &nodes {
            let Some((head, args)) = node.form() else {
                continue;
            };
            let name = args.first().and_then(Node::atom).map(str::to_string);
            match (head, name) {
                ("ROOM", Some(name)) => self.rooms.push(entity(name, args, file)),
                ("OBJECT", Some(name)) => self.objects.push(entity(name, args, file)),
                ("GLOBAL", Some(name)) => {
                    let value = args.get(1).cloned().unwrap_or(Node::Form(vec![]));
                    self.globals.entry(name).or_insert(value);
                }
                ("ROUTINE", Some(name)) => {
                    let params = args.get(1).and_then(Node::list).unwrap_or_default();
                    self.routines.entry(name).or_insert(Routine {
                        params: params.to_vec(),
                        body: args.iter().skip(2).cloned().collect(),
                        file: file.to_string(),
                    });
                }
                ("DIRECTIONS", _) => {
                    self.directions = args
                        .iter()
                        .filter_map(Node::atom)
                        .map(str::to_string)
                        .collect();
                }
                _ => {}
            }
        }
    }

    pub fn room(&self, name: &str) -> Option<&Entity> {
        self.rooms.iter().find(|r| r.name == name)
    }

    pub fn object(&self, name: &str) -> Option<&Entity> {
        self.objects.iter().find(|o| o.name == name)
    }

    pub fn is_direction(&self, word: &str) -> bool {
        self.directions.iter().any(|d| d == word)
    }
}

fn entity(name: String, args: &[Node], file: &str) -> Entity {
    let props = args
        .iter()
        .skip(1)
        .filter_map(Node::list)
        .filter_map(|items| {
            let (head, rest) = items.split_first()?;
            Some((head.atom()?.to_string(), rest.to_vec()))
        })
        .collect();
    Entity {
        name,
        props,
        file: file.to_string(),
    }
}
