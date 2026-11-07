//! Tests of `compile_dialogue`.

use super::*;
use assert2::{assert, check};
use dramaturge_model::{Condition, DialogTarget};
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
    check!(c.nodes.len() == 3);
    // The start nodes, in order: again, meet.
    check!(c.entry_nodes == [c.nodes[0].id, c.nodes[1].id]);
    // serde_json sorts map keys: again, meet, names.
    let meet = &c.nodes[1];
    check!(meet.options[0].target == DialogTarget::Node(c.nodes[2].id));
    // The lines are compiled markup in the graph.
    let line = meet.text.as_host().expect("host text");
    check!(line.len() == 1);
    check!(line[0].text == vec![crate::Segment::Text("Sit.".into())]);
    check!(meet.options[1].target == DialogTarget::Return);
    check!(meet.options[2].target == DialogTarget::End);
    assert!(let [Outcome::AddRelationship { delta: 1, .. }] = meet.options[0].outcomes.as_slice());
    check!(matches!(
        meet.options[1].conditions[0],
        Condition::RelationshipAtLeast { value: 1, .. }
    ));
    let names = &c.nodes[2];
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
        c.nodes[0].conditions[0],
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

#[test]
fn an_option_id_is_its_key_and_a_bad_start_is_at_start() {
    let mut story = story();
    story.dialogues[0].nodes[1].options[0].id = Some("ask".into());
    let scope = Scope::from_story(&story);
    let mut symbols = scope.symbols.clone();
    let c = compile_dialogue(&story.dialogues[0], &scope, &mut symbols).unwrap();
    check!(symbols.option_key(c.nodes[1].options[0].id) == Some("bb/ask"));
    check!(symbols.option_key(c.nodes[1].options[1].id) == Some("bb/meet#1"));

    story.dialogues[0].start[1] = "mett".into();
    let err = compile_dialogue(&story.dialogues[0], &scope, &mut symbols).unwrap_err();
    check!(err.at == "start[1]");
    check!(err.source == "mett");
    check!(err.error.message == "no node `mett`");
}
