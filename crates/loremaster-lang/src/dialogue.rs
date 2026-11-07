use crate::lower::resolve;
use crate::{LangError, Operand, Scope, Segment, compile_condition, compile_markup, parse_expr};
use dramaturge_model::{
    CharacterId, DialogTarget, DialogText, DialogueGraph, DialogueNode, DialogueNodeId,
    DialogueOption, DialogueOptionId, Fact, FactId, FactScope, FactValue, Memory, Outcome,
    RelationshipAxisId, SymbolTable,
};
use loremaster_model::{Dialogue, Effect, Outcomes, Value};
use std::collections::BTreeMap;

/// A dialogue lowered to a dramaturge graph.
///
/// dramaturge holds the structure, the conditions and the outcomes. A `then`
/// list is a host outcome ([`Outcome::Host`]) that the runtime applies. The text
/// stays loremaster markup: the graph holds the source as
/// [`DialogText::Inline`], and `node_say` / `option_say` hold the compiled
/// markup that the runtime renders.
#[derive(Debug, Clone)]
pub struct CompiledDialogue {
    /// The start nodes are its entry nodes, in order.
    pub graph: DialogueGraph<Vec<Effect>>,
    pub node_say: BTreeMap<DialogueNodeId, Vec<Segment>>,
    pub option_say: BTreeMap<DialogueOptionId, Vec<Segment>>,
}

/// A problem in a dialogue, with the place it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueError {
    /// For example `nodes.meet.options[1].when`.
    pub at: String,
    /// The source text the error is in, to locate it in the file.
    pub source: String,
    pub error: LangError,
}

/// The symbol key of a node.
pub fn node_key(dialogue: &str, node: &str) -> String {
    format!("{dialogue}/{node}")
}

/// The symbol key of an option.
pub fn option_key(dialogue: &str, node: &str, index: usize) -> String {
    format!("{dialogue}/{node}#{index}")
}

/// Lower a dialogue to a dramaturge graph. Node and option keys are interned
/// in `symbols`; the player, things and memories must be in it already (see
/// [`Scope::symbols`]).
///
/// A `go` to a node that does not exist is not an error here: it gets an id
/// that no node has, and the graph validator reports it.
pub fn compile_dialogue(
    d: &Dialogue,
    scope: &Scope,
    symbols: &mut SymbolTable,
) -> Result<CompiledDialogue, DialogueError> {
    let fail = |at: String, source: &str, error: LangError| DialogueError {
        at,
        source: source.to_string(),
        error,
    };
    let Some(with) = scope.entity(d.with.as_str()) else {
        return Err(fail(
            "with".into(),
            d.with.as_str(),
            LangError::new(format!("unknown thing `{}`", d.with), 0..0),
        ));
    };
    let speaker = CharacterId::new(with);
    let mut out = CompiledDialogue {
        graph: DialogueGraph::typed(),
        node_say: BTreeMap::new(),
        option_say: BTreeMap::new(),
    };
    let ids: Vec<DialogueNodeId> = d
        .nodes
        .iter()
        .map(|n| symbols.intern_node(&node_key(d.id.as_str(), &n.key)))
        .collect();
    for (n, &id) in d.nodes.iter().zip(&ids) {
        let at = format!("nodes.{}", n.key);
        let say = compile_markup(&n.say, scope).map_err(|e| fail(at.clone(), &n.say, e))?;
        out.node_say.insert(id, say);
        let mut node = DialogueNode {
            id,
            speaker,
            text: DialogText::Inline(n.say.clone()),
            conditions: Vec::new(),
            outcomes_on_enter: Vec::new(),
            options: Vec::new(),
            lines: Vec::new(),
            next: None,
        };
        if let Some(when) = &n.when {
            node.conditions.push(
                compile_condition(when, scope).map_err(|e| fail(format!("{at}.when"), when, e))?,
            );
        }
        node.outcomes_on_enter =
            outcomes(d, &n.outcomes, scope).map_err(|(source, e)| fail(at.clone(), &source, e))?;
        for (i, o) in n.options.iter().enumerate() {
            let at = format!("{at}.options[{i}]");
            let oid = symbols.intern_option(&option_key(d.id.as_str(), &n.key, i));
            let say = compile_markup(&o.say, scope).map_err(|e| fail(at.clone(), &o.say, e))?;
            out.option_say.insert(oid, say);
            let target = match o.go.as_str() {
                "end" => DialogTarget::End,
                "back" => DialogTarget::Return,
                key => DialogTarget::Node(symbols.intern_node(&node_key(d.id.as_str(), key))),
            };
            let mut option =
                DialogueOption::typed(oid.0, DialogText::Inline(o.say.clone()), target, id);
            if let Some(when) = &o.when {
                option.conditions.push(
                    compile_condition(when, scope)
                        .map_err(|e| fail(format!("{at}.when"), when, e))?,
                );
            }
            option.outcomes = outcomes(d, &o.outcomes, scope)
                .map_err(|(source, e)| fail(at.clone(), &source, e))?;
            node.options.push(option);
        }
        out.graph.nodes.push(node);
    }
    for (i, key) in d.start.iter().enumerate() {
        match d.nodes.iter().position(|n| &n.key == key) {
            Some(index) => out.graph.entry_nodes.push(ids[index]),
            None => {
                return Err(fail(
                    format!("start[{i}]"),
                    key,
                    LangError::new(format!("no node `{key}`"), 0..key.len()),
                ));
            }
        }
    }
    Ok(out)
}

fn outcomes(
    d: &Dialogue,
    o: &Outcomes,
    scope: &Scope,
) -> Result<Vec<Outcome<Vec<Effect>>>, (String, LangError)> {
    let player = scope.player();
    // `compile_dialogue` checked `with`.
    let with = scope.entity(d.with.as_str()).unwrap_or(player);
    let mut out = Vec::new();
    for (name, value) in &o.set {
        fact_name(name, scope)?;
        out.push(Outcome::SetFact {
            scope: FactScope::World,
            fact: Fact::new(
                FactId::new(name.as_str()),
                match value {
                    Value::Bool(b) => FactValue::Bool(*b),
                    Value::Int(i) => FactValue::Int(*i),
                    Value::Text(t) => FactValue::Text(t.clone()),
                },
            ),
        });
    }
    for (name, n) in &o.add {
        fact_name(name, scope)?;
        out.push(Outcome::AddFact {
            scope: FactScope::World,
            fact_id: FactId::new(name.as_str()),
            delta: *n,
        });
    }
    for (axis, delta) in &o.relation {
        if !scope.relations.contains(axis) {
            return Err((
                axis.clone(),
                LangError::new(
                    format!("unknown relation `{axis}`; declare it in `relations` in story.yaml"),
                    0..axis.len(),
                ),
            ));
        }
        out.push(Outcome::AddRelationship {
            source: player,
            target: with,
            axis: RelationshipAxisId::new(axis.as_str()),
            delta: *delta,
        });
    }
    for key in &o.remember {
        let id = scope.symbols.memory(key).ok_or_else(|| {
            (
                key.clone(),
                LangError::new(format!("unknown memory `{key}`"), 0..key.len()),
            )
        })?;
        out.push(Outcome::AddMemory {
            owner: with,
            memory: Memory::new(id).with_subject(player),
        });
    }
    if !o.then.is_empty() {
        out.push(Outcome::Host(o.then.clone()));
    }
    Ok(out)
}

/// A name that `set` or `add` can change: a story variable or a thing property.
fn fact_name(name: &str, scope: &Scope) -> Result<(), (String, LangError)> {
    let expr = parse_expr(name).map_err(|e| (name.to_string(), e))?;
    match resolve(&expr, scope) {
        Ok((Operand::Fact(_), _)) => Ok(()),
        Ok(_) => Err((
            name.to_string(),
            LangError::new(
                format!("`{name}` is computed by the game; it can not be set"),
                0..name.len(),
            ),
        )),
        Err(e) => Err((name.to_string(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::{assert, check};
    use dramaturge_model::Condition;
    use loremaster_model::Story;

    pub(super) fn story() -> Story {
        serde_json::from_value(serde_json::json!({
            "title": "t",
            "start": "bar",
            "relations": ["trust"],
            "vars": [{"id": "known", "type": "bool", "initial": false}, {"id": "visits_bb", "type": "int", "initial": 0}],
            "rooms": [{"id": "bar", "name": "Bar"}],
            "things": [{"id": "bb", "name": "man", "location": {"room": "bar"}}],
            "dialogues": [{
                "id": "bb", "with": "bb", "start": ["again", "meet"],
                "nodes": {
                    "meet": {
                        "say": "Sit.",
                        "options": [
                            {"say": "Who?", "go": "names", "relation": {"trust": 1}},
                            {"say": "Ship?", "when": "bb.trust >= 1", "remember": ["asked"], "go": "back"},
                            {"say": "Bye."}
                        ]
                    },
                    "names": {"say": "Blackbeard.", "set": {"known": true}, "add": {"visits_bb": 2}, "then": [{"goto": "bar"}]},
                    "again": {"say": "Again.", "when": "remembers(bb, asked)"}
                }
            }]
        }))
        .unwrap()
    }

    #[test]
    fn a_dialogue_becomes_a_graph() {
        let story = story();
        let scope = Scope::from_story(&story);
        let mut symbols = scope.symbols.clone();
        let c = compile_dialogue(&story.dialogues[0], &scope, &mut symbols).unwrap();
        check!(c.graph.nodes.len() == 3);
        // The start nodes, in order: again, meet.
        check!(c.graph.entry_nodes == [c.graph.nodes[0].id, c.graph.nodes[1].id]);
        // serde_json sorts map keys: again, meet, names.
        let meet = &c.graph.nodes[1];
        check!(meet.options[0].target == DialogTarget::Node(c.graph.nodes[2].id));
        check!(meet.options[1].target == DialogTarget::Return);
        check!(meet.options[2].target == DialogTarget::End);
        assert!(let [Outcome::AddRelationship { delta: 1, .. }] = meet.options[0].outcomes.as_slice());
        check!(matches!(
            meet.options[1].conditions[0],
            Condition::RelationshipAtLeast { value: 1, .. }
        ));
        let names = &c.graph.nodes[2];
        check!(matches!(
            names.outcomes_on_enter[0],
            Outcome::SetFact { .. }
        ));
        check!(matches!(
            &names.outcomes_on_enter[1],
            Outcome::AddFact { scope: FactScope::World, fact_id, delta: 2 } if fact_id.0.as_ref() == "visits_bb"
        ));
        check!(
            matches!(&names.outcomes_on_enter[2], Outcome::Host(then) if matches!(then.as_slice(), [Effect::Goto(room)] if room.as_str() == "bar"))
        );
        check!(matches!(
            c.graph.nodes[0].conditions[0],
            Condition::HasMemory { .. }
        ));
        check!(symbols.node_key(names.id) == Some("bb/names"));
    }

    #[test]
    fn errors_name_their_place() {
        let mut story = story();
        story.dialogues[0].nodes[1].options[1].when = Some("bb.trsut >= 1".into());
        let scope = Scope::from_story(&story);
        let err =
            compile_dialogue(&story.dialogues[0], &scope, &mut scope.symbols.clone()).unwrap_err();
        check!(err.at == "nodes.meet.options[1].when");
        check!(err.error.message.contains("trust"));

        let mut story = self::story();
        story.dialogues[0].nodes[1].options[0]
            .outcomes
            .relation
            .insert("fear".into(), 1);
        let scope = Scope::from_story(&story);
        let err =
            compile_dialogue(&story.dialogues[0], &scope, &mut scope.symbols.clone()).unwrap_err();
        check!(err.error.message.starts_with("unknown relation `fear`"));
    }

    #[test]
    fn remembers_needs_a_known_memory() {
        let story = story();
        let scope = Scope::from_story(&story);
        let err = compile_condition("remembers(bb, forgot)", &scope).unwrap_err();
        check!(err.message == "no dialogue remembers `forgot`");
    }
}
