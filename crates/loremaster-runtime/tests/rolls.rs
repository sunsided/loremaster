//! Numeric stats: forced rolls and seeds in walkthroughs, limits on
//! variables, `calc`, and the RNG in the save.

use loremaster_model::{Value, Walkthrough};
use loremaster_project::{Severity, SourceFile, check, load};
use loremaster_runtime::{Input, Session, Status, World, run_walkthrough};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

const RULES: &str = r#"
- id: test_luck
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { roll: 2d6, into: r }
    - calc: { total: "r + skill - 1" }
    - say: "Total {total}."
- id: drink
  on: { verb: [push, lever] }
  instead: true
  then:
    - add: { stamina: 10 }
- id: wound
  on: { verb: [wait] }
  instead: true
  then:
    - add: { stamina: -100 }
"#;

fn files(vars: &str) -> Vec<SourceFile> {
    vec![
        file(
            "story.yaml",
            &format!(
                "title: Stats\nstart: hall\nvars:\n  - {{ id: r, type: int, initial: 0 }}\n  - {{ id: total, type: int, initial: 0 }}\n  - {{ id: skill, type: int, initial: 3 }}\n  - {{ id: stamina_initial, type: int, initial: 12 }}\n{vars}"
            ),
        ),
        file(
            "rooms/hall.yaml",
            "name: Hall\ndescription: [\"A hall.\"]\n",
        ),
        file(
            "things/lever.yaml",
            "name: lever\nlocation: { room: hall }\n",
        ),
        file("verbs/pull.yaml", "label: Pull\nwords: [pull]\n"),
        file("verbs/push.yaml", "label: Push\nwords: [push]\n"),
        file("rules/rules.yaml", RULES),
    ]
}

const STAMINA: &str = "  - { id: stamina, type: int, initial: 10, min: 0, max: stamina_initial }\n";

fn world() -> Arc<World> {
    Arc::new(World::compile(load(files(STAMINA)).unwrap().story).unwrap())
}

fn walk(yaml: &str) -> Walkthrough {
    let mut f = files(STAMINA);
    f.push(file("tests/t.yaml", yaml));
    load(f).unwrap().walkthroughs.remove(0).1
}

fn fact(s: &Session, name: &str) -> i64 {
    s.state().facts[name].as_int().unwrap()
}

fn errors(vars: &str) -> Vec<String> {
    check(&load(files(&format!("{STAMINA}{vars}"))).unwrap())
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_forced_roll_gives_the_total() {
    let w = walk("steps:\n  - { do: pull lever, rolls: [5], facts: { r: 5, total: 7 } }\n");
    let result = run_walkthrough(world(), &w, "t");
    assert_eq!(result.failures, Vec::<String>::new());
}

#[test]
fn a_forced_roll_out_of_range_fails() {
    let w = walk("steps:\n  - { do: pull lever, rolls: [13] }\n");
    let result = run_walkthrough(world(), &w, "t");
    assert_eq!(
        result.failures,
        ["step 1 (`pull lever`): the forced roll 13 is not in the range of 2d6 (2 to 12)"]
    );
}

#[test]
fn an_unused_forced_roll_fails() {
    let w = walk("steps:\n  - { do: pull lever, rolls: [5, 6] }\n");
    let result = run_walkthrough(world(), &w, "t");
    assert_eq!(
        result.failures,
        ["step 1 (`pull lever`): no roll took the forced rolls 6"]
    );
}

#[test]
fn the_seed_of_a_walkthrough_selects_the_rolls() {
    let total = |seed: u64| {
        let world = world();
        let mut s = Session::with_seed(world.clone(), seed);
        s.start();
        s.submit(Input::Text("pull lever".into()));
        let r = fact(&s, "r");
        let w = walk(&format!(
            "seed: {seed}\nsteps:\n  - {{ do: pull lever, facts: {{ r: {r} }} }}\n"
        ));
        let result = run_walkthrough(world, &w, "t");
        assert_eq!(result.failures, Vec::<String>::new());
        r
    };
    assert!((1..200).map(total).any(|r| r != total(0)));
}

#[test]
fn each_write_is_clamped() {
    let mut s = Session::new(world());
    s.start();
    s.submit(Input::Text("push lever".into()));
    assert_eq!(fact(&s, "stamina"), 12, "max is a variable");
    s.submit(Input::Text("wait".into()));
    assert_eq!(fact(&s, "stamina"), 0, "min is a number");
}

#[test]
fn a_host_answer_is_clamped() {
    let files = files("  - { id: low, type: int, initial: 0, max: 5 }\n")
        .into_iter()
        .map(|mut f| {
            if f.path == "rules/rules.yaml" {
                f.text = "- { id: r, on: { verb: [pull, lever] }, instead: true, then: [{ await: { roll: 2d6, into: low } }] }\n".into();
            }
            f
        })
        .collect();
    let world = Arc::new(World::compile(load(files).unwrap().story).unwrap());
    let mut s = Session::new(world);
    s.set_host_rolls(true);
    s.start();
    let Status::Await { awaiting } = s.submit(Input::Text("pull lever".into())).status else {
        panic!("no await");
    };
    s.resume(&awaiting.id, Value::Int(11)).unwrap();
    assert_eq!(fact(&s, "low"), 5);
}

#[test]
fn a_save_replays_the_next_roll() {
    let world = world();
    let mut s = Session::with_seed(world.clone(), 7);
    s.start();
    let save = serde_json::to_value(s.save()).unwrap();
    s.submit(Input::Text("pull lever".into()));
    let first = fact(&s, "r");
    let (mut again, _) = Session::load(world, save).unwrap();
    again.submit(Input::Text("pull lever".into()));
    assert_eq!(fact(&again, "r"), first);
}

#[test]
fn limits_are_checked_at_load() {
    assert_eq!(errors(""), Vec::<String>::new());
    assert_eq!(
        errors("  - { id: flag, type: bool, initial: false, min: 0 }\n"),
        ["`min` needs an `int` variable"]
    );
    assert_eq!(
        errors("  - { id: hp, type: int, initial: 1, max: nope }\n"),
        ["`max: nope` names no `int` variable"]
    );
    assert_eq!(
        errors("  - { id: hp, type: int, initial: 1, min: 5, max: 1 }\n"),
        ["`min` 5 is above `max` 1"]
    );
}

#[test]
fn a_calc_is_checked_at_load() {
    let with = |sum: &str| {
        let mut f = files(STAMINA);
        f.last_mut().unwrap().text =
            format!("- {{ id: c, on: start, then: [{{ calc: {{ total: \"{sum}\" }} }}] }}\n");
        check(&load(f).unwrap())
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message)
            .collect::<Vec<_>>()
    };
    assert_eq!(with("skill + 1"), Vec::<String>::new());
    assert_eq!(
        with("skil + 1"),
        ["`skil + 1`: `skil` is not a number variable or property"]
    );
    assert_eq!(with("skill +"), ["`skill +`: a number or a name expected"]);
}
