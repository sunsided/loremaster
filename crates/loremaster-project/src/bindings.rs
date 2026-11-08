//! Types for a host, generated from the `interface:` of a story: Rust
//! (IF-3), and JSON Schema and TypeScript for other languages (IF-4).

use loremaster_model::{ArgType, Interface};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fmt::Write;

/// `broken_pick` is `BrokenPick`. Two ids can meet (`level_2`, `level2`);
/// the check reports that (`check/contract.rs`).
pub(crate) fn pascal(id: &str) -> String {
    let out: String = id
        .split('_')
        .map(|part| {
            let mut c = part.chars();
            c.next()
                .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
        })
        .collect();
    if out == "Self" { "Self_".into() } else { out }
}

/// A Rust field name: a keyword gets the `r#` prefix, and the names that
/// cannot be raw get a trailing `_`.
fn field(name: &str) -> String {
    if matches!(name, "self" | "Self" | "crate" | "super") {
        return format!("{name}_");
    }
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
        "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in",
        "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
        "return", "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe",
        "unsized", "use", "virtual", "where", "while", "yield",
    ];
    if KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

/// A unit-variant enum that converts from and to its ids.
fn id_enum(out: &mut String, krate: &str, doc: &str, name: &str, ids: &[String]) {
    let _ = writeln!(
        out,
        "/// {doc}\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum {name} {{"
    );
    for id in ids {
        let _ = writeln!(out, "    {},", pascal(id));
    }
    let _ = writeln!(
        out,
        "}}\nimpl {name} {{\n    /// The id in the story.\n    pub fn id(self) -> &'static str {{\n        match self {{"
    );
    for id in ids {
        let _ = writeln!(out, "            Self::{} => {id:?},", pascal(id));
    }
    let _ = writeln!(
        out,
        "        }}\n    }}\n    /// The value that the session takes.\n    pub fn value(self) -> {krate}::Value {{\n        {krate}::Value::Text(self.id().to_string())\n    }}\n    /// The variant of an id.\n    pub fn parse(id: &str) -> Option<Self> {{\n        match id {{"
    );
    for id in ids {
        let _ = writeln!(out, "            {id:?} => Some(Self::{}),", pascal(id));
    }
    let _ = writeln!(out, "            _ => None,\n        }}\n    }}\n}}\n");
}

fn rust_type(owner: &str, arg: &str, ty: &ArgType) -> String {
    match ty {
        ArgType::Bool => "bool".into(),
        ArgType::Int => "i64".into(),
        ArgType::Text => "String".into(),
        ArgType::Enum(_) => format!("{}{}", pascal(owner), pascal(arg)),
    }
}

fn arg_enums(out: &mut String, krate: &str, owner: &str, args: &BTreeMap<String, ArgType>) {
    for (arg, ty) in args {
        if let ArgType::Enum(values) = ty {
            let name = rust_type(owner, arg, ty);
            id_enum(
                out,
                krate,
                &format!("The argument `{arg}` of `{owner}`."),
                &name,
                values,
            );
        }
    }
}

/// The fields of a variant, for its declaration.
fn fields(owner: &str, args: &BTreeMap<String, ArgType>) -> String {
    args.iter()
        .map(|(a, t)| format!("{}: {}", field(a), rust_type(owner, a, t)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The Rust expression that reads argument `arg` from `args`, or returns `None`.
fn read_arg(krate: &str, owner: &str, arg: &str, ty: &ArgType) -> String {
    let get = format!("args.get({arg:?})?");
    match ty {
        ArgType::Bool => {
            format!("match {get} {{ {krate}::Value::Bool(v) => *v, _ => return None }}")
        }
        ArgType::Int => format!("match {get} {{ {krate}::Value::Int(v) => *v, _ => return None }}"),
        ArgType::Text => {
            format!("match {get} {{ {krate}::Value::Text(v) => v.clone(), _ => return None }}")
        }
        ArgType::Enum(_) => format!(
            "match {get} {{ {krate}::Value::Text(v) => {}::parse(v)?, _ => return None }}",
            rust_type(owner, arg, ty)
        ),
    }
}

/// The Rust expression for the value of the bound field `arg`.
fn write_arg(krate: &str, arg: &str, ty: &ArgType) -> String {
    let f = field(arg);
    match ty {
        ArgType::Bool => format!("{krate}::Value::Bool(*{f})"),
        ArgType::Int => format!("{krate}::Value::Int(*{f})"),
        ArgType::Text => format!("{krate}::Value::Text({f}.clone())"),
        ArgType::Enum(_) => format!("{f}.value()"),
    }
}

/// The types that [`rust_bindings`] declares, each with what it is for. Two
/// with one name do not compile; the check reports them.
pub(crate) fn type_names(interface: &Interface) -> Vec<(String, String)> {
    let mut out = vec![
        ("Request".to_string(), "the requests".to_string()),
        ("HostEvent".to_string(), "the host events".to_string()),
    ];
    for a in &interface.actions {
        if !a.results.is_empty() {
            let what = format!("the results of the action `{}`", a.id);
            out.push((format!("{}Result", pascal(a.id.as_str())), what));
        }
    }
    let owners = (interface.actions.iter().map(|a| ("action", &a.id, &a.args)))
        .chain(interface.events.iter().map(|e| ("event", &e.id, &e.args)));
    for (kind, id, args) in owners {
        for (arg, ty) in args {
            if let ArgType::Enum(_) = ty {
                let what = format!("the argument `{arg}` of the {kind} `{id}`");
                out.push((rust_type(id.as_str(), arg, ty), what));
            }
        }
    }
    out
}

/// Rust source with the hash, `Request`, one result enum per action and
/// `HostEvent`. `krate` is the path of `loremaster-runtime` in the host.
pub fn rust_bindings(interface: &Interface, krate: &str) -> String {
    let json = serde_json::to_string(interface).expect("an interface is plain data");
    let mut out = format!(
        "/// The hash of the interface that this code was generated for (IF-5).\npub const INTERFACE_HASH: &str = {:?};\n/// The same interface as JSON.\npub const INTERFACE_JSON: &str = {json:?};\n\n/// The interface that this host is built for, for `Session::for_host`.\npub fn interface() -> {krate}::Interface {{\n    {krate}::Interface::from_json(INTERFACE_JSON).expect(\"the generated JSON reads\")\n}}\n\n",
        interface.hash()
    );
    for a in &interface.actions {
        arg_enums(&mut out, krate, a.id.as_str(), &a.args);
        if !a.results.is_empty() {
            let ids: Vec<String> = a.results.iter().map(ToString::to_string).collect();
            let name = format!("{}Result", pascal(a.id.as_str()));
            id_enum(
                &mut out,
                krate,
                &format!("The results of `{}`.", a.id),
                &name,
                &ids,
            );
        }
    }
    for e in &interface.events {
        arg_enums(&mut out, krate, e.id.as_str(), &e.args);
    }
    out.push_str("/// What the story can ask the host to do.\n#[derive(Debug, Clone, PartialEq)]\npub enum Request {\n");
    for a in &interface.actions {
        let _ = writeln!(
            out,
            "    {} {{ {} }},",
            pascal(a.id.as_str()),
            fields(a.id.as_str(), &a.args)
        );
    }
    let _ = writeln!(
        out,
        "}}\nimpl Request {{\n    /// The request of an `Await` status; `None` for a roll, a choice or a walk.\n    pub fn from_await(request: &{krate}::AwaitRequest) -> Option<Self> {{\n        let {krate}::AwaitRequest::Action {{ name, args, .. }} = request else {{\n            return None;\n        }};\n        let _ = args;\n        match name.as_str() {{"
    );
    for a in &interface.actions {
        let read: Vec<String> = a
            .args
            .iter()
            .map(|(arg, ty)| {
                format!(
                    "{}: {}",
                    field(arg),
                    read_arg(krate, a.id.as_str(), arg, ty)
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "            {:?} => Some(Self::{} {{ {} }}),",
            a.id.as_str(),
            pascal(a.id.as_str()),
            read.join(", ")
        );
    }
    out.push_str("            _ => None,\n        }\n    }\n}\n\n");
    out.push_str("/// What the host can report to the story.\n#[derive(Debug, Clone, PartialEq)]\npub enum HostEvent {\n");
    for e in &interface.events {
        let _ = writeln!(
            out,
            "    {} {{ {} }},",
            pascal(e.id.as_str()),
            fields(e.id.as_str(), &e.args)
        );
    }
    out.push_str("}\nimpl HostEvent {\n    /// The id for `Session::host_event`.\n    pub fn name(&self) -> &'static str {\n        match *self {\n");
    for e in &interface.events {
        let _ = writeln!(
            out,
            "            Self::{} {{ .. }} => {:?},",
            pascal(e.id.as_str()),
            e.id.as_str()
        );
    }
    let _ = writeln!(
        out,
        "        }}\n    }}\n    /// The arguments for `Session::host_event`.\n    pub fn args(&self) -> ::std::collections::BTreeMap<String, {krate}::Value> {{\n        match *self {{"
    );
    for e in &interface.events {
        let binds: Vec<String> = e.args.keys().map(|a| format!("ref {}", field(a))).collect();
        let items: Vec<String> = e
            .args
            .iter()
            .map(|(a, t)| format!("({a:?}.to_string(), {})", write_arg(krate, a, t)))
            .collect();
        let _ = writeln!(
            out,
            "            Self::{} {{ {} }} => ::std::collections::BTreeMap::from([{}]),",
            pascal(e.id.as_str()),
            binds.join(", "),
            items.join(", ")
        );
    }
    out.push_str("        }\n    }\n}\n");
    out
}

fn schema_of(ty: &ArgType) -> Value {
    match ty {
        ArgType::Bool => json!({ "type": "boolean" }),
        ArgType::Int => json!({ "type": "integer" }),
        ArgType::Text => json!({ "type": "string" }),
        ArgType::Enum(values) => json!({ "enum": values }),
    }
}

fn args_schema(args: &BTreeMap<String, ArgType>) -> Value {
    let props: Map<String, Value> = args
        .iter()
        .map(|(a, t)| (a.clone(), schema_of(t)))
        .collect();
    json!({
        "type": "object",
        "properties": props,
        "required": args.keys().collect::<Vec<_>>(),
        "additionalProperties": false,
    })
}

/// The JSON Schema of the requests of a session and of the host events, in
/// the forms that the protocol and the WASM session use.
pub fn json_schema(interface: &Interface) -> Value {
    let mut defs = Map::new();
    let requests: Vec<Value> = interface
        .actions
        .iter()
        .map(|a| {
            let mut required = vec!["kind", "name"];
            // The session leaves out empty `args` and `results`.
            if !a.args.is_empty() {
                required.push("args");
            }
            let mut props = json!({
                "kind": { "const": "action" },
                "name": { "const": a.id },
                "args": args_schema(&a.args),
            });
            if !a.results.is_empty() {
                let def = format!("{}Result", pascal(a.id.as_str()));
                defs.insert(def.clone(), json!({ "enum": a.results }));
                props["results"] =
                    json!({ "type": "array", "items": { "$ref": format!("#/$defs/{def}") } });
                required.push("results");
            }
            json!({ "type": "object", "properties": props, "required": required })
        })
        .collect();
    let events: Vec<Value> = interface
        .events
        .iter()
        .map(|e| {
            let required = if e.args.is_empty() {
                json!(["name"])
            } else {
                json!(["name", "args"])
            };
            json!({
                "type": "object",
                "properties": { "name": { "const": e.id }, "args": args_schema(&e.args) },
                "required": required,
            })
        })
        .collect();
    // An empty `oneOf` is no valid schema; `false` matches nothing.
    let one_of = |v: Vec<Value>| {
        if v.is_empty() {
            json!(false)
        } else {
            json!({ "oneOf": v })
        }
    };
    defs.insert("Request".into(), one_of(requests));
    defs.insert("HostEvent".into(), one_of(events));
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "loremaster host interface",
        "x-interface-hash": interface.hash(),
        "$defs": defs,
    })
}

fn ts_type(ty: &ArgType) -> String {
    match ty {
        ArgType::Bool => "boolean".into(),
        ArgType::Int => "number".into(),
        ArgType::Text => "string".into(),
        ArgType::Enum(values) => values
            .iter()
            .map(|v| format!("{v:?}"))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

/// The `args` property: optional when there are none, as on the wire.
fn ts_args(args: &BTreeMap<String, ArgType>) -> String {
    let fields: Vec<String> = args
        .iter()
        .map(|(a, t)| format!("{a:?}: {}", ts_type(t)))
        .collect();
    let colon = if args.is_empty() { "?:" } else { ":" };
    format!("args{colon} {{ {} }}", fields.join("; "))
}

/// TypeScript types of the same contract as [`json_schema`].
pub fn typescript(interface: &Interface) -> String {
    let mut out = format!("export const INTERFACE_HASH = {:?};\n\n", interface.hash());
    let mut requests = Vec::new();
    for a in &interface.actions {
        let mut body = format!(
            "kind: 'action'; name: {:?}; {}",
            a.id.as_str(),
            ts_args(&a.args)
        );
        if !a.results.is_empty() {
            let name = format!("{}Result", pascal(a.id.as_str()));
            let ids: Vec<String> = a
                .results
                .iter()
                .map(|r| format!("{:?}", r.as_str()))
                .collect();
            let _ = writeln!(out, "export type {name} = {};", ids.join(" | "));
            body.push_str(&format!("; results: {name}[]"));
        }
        requests.push(format!("{{ {body} }}"));
    }
    let events: Vec<String> = interface
        .events
        .iter()
        .map(|e| format!("{{ name: {:?}; {} }}", e.id.as_str(), ts_args(&e.args)))
        .collect();
    let union = |v: Vec<String>| {
        if v.is_empty() {
            "never".to_string()
        } else {
            v.join(" | ")
        }
    };
    let _ = writeln!(out, "export type Request = {};", union(requests));
    let _ = writeln!(out, "export type HostEvent = {};", union(events));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Interface {
        serde_json::from_value(json!({
            "actions": [{ "id": "lockpick", "args": { "difficulty": "int" }, "results": ["success", "broken_pick"] }],
            "events": [{ "id": "killed", "args": { "boss": "text" } }]
        }))
        .unwrap()
    }

    #[test]
    fn the_schema_names_each_result_and_event() {
        let s = json_schema(&sample());
        assert_eq!(
            s["$defs"]["LockpickResult"]["enum"],
            json!(["success", "broken_pick"])
        );
        assert_eq!(
            s["$defs"]["HostEvent"]["oneOf"][0]["properties"]["name"]["const"],
            json!("killed")
        );
        assert_eq!(s["x-interface-hash"], json!(sample().hash()));
        assert_eq!(
            s["$defs"]["Request"]["oneOf"][0]["required"],
            json!(["kind", "name", "args", "results"])
        );
    }

    #[test]
    fn a_name_that_cannot_be_raw_gets_an_underscore() {
        assert_eq!(field("type"), "r#type");
        assert_eq!(field("try"), "r#try");
        assert_eq!(field("self"), "self_");
        assert_eq!(field("boss"), "boss");
        assert_eq!(pascal("self"), "Self_");
        assert_eq!(pascal("broken_pick"), "BrokenPick");
    }

    #[test]
    fn no_arguments_and_no_entries_stay_valid() {
        let i: Interface =
            serde_json::from_value(json!({ "events": [{ "id": "found" }] })).unwrap();
        let s = json_schema(&i);
        assert_eq!(s["$defs"]["Request"], json!(false));
        assert_eq!(
            s["$defs"]["HostEvent"]["oneOf"][0]["required"],
            json!(["name"])
        );
        assert!(
            typescript(&i).contains("name: \"found\"; args?: {  }"),
            "{}",
            typescript(&i)
        );
    }

    #[test]
    fn typescript_has_the_unions_and_the_hash() {
        let t = typescript(&sample());
        assert!(
            t.contains("export type LockpickResult = \"success\" | \"broken_pick\";"),
            "{t}"
        );
        assert!(t.contains("name: \"killed\""), "{t}");
        assert!(t.contains(&sample().hash()), "{t}");
        assert!(typescript(&Interface::default()).contains("export type Request = never;"));
    }
}
