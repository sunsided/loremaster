//! The ending checks of the explorer (CK, P4-04): dead ends, points of no
//! return, `closes`, and a walk that the budget stopped.

use loremaster_model::{Diagnostic, Severity};
use loremaster_project::{Project, SourceFile, check, load};
use loremaster_runtime::World;
use loremaster_runtime::authoring::{Budget, Exploration, explore};
use std::sync::Arc;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A cell with a way out (`escape`, the best ending) and a way to give up
/// (`surrender`). `rules` decides what closes the way out.
fn cell(rules: &str, extra: &[(&str, &str)]) -> Project {
    let mut files = vec![
        file(
            "story.yaml",
            "title: Cell\nstart: cell\nvars:\n  - { id: burned, type: bool, initial: false }\n  - { id: steps, type: int, initial: 0 }\n",
        ),
        file(
            "rooms/cell.yaml",
            "name: Cell\ndescription: [\"Stone, straw and a map of the tunnels.\"]\n",
        ),
        file(
            "endings/escaped.yaml",
            "kind: win\nrank: 1\ntitle: Escaped\n",
        ),
        file(
            "endings/caught.yaml",
            "kind: loss\nrank: 3\ntitle: Caught\n",
        ),
        file(
            "verbs/escape.yaml",
            "label: Escape\nwords: [escape]\nslots: none\n",
        ),
        file(
            "verbs/burn.yaml",
            "label: Burn\nwords: [burn]\nslots: none\n",
        ),
        file(
            "verbs/surrender.yaml",
            "label: Surrender\nwords: [surrender]\nslots: none\n",
        ),
        file("rules/cell.yaml", rules),
    ];
    files.extend(extra.iter().map(|(p, t)| file(p, t)));
    load(files).unwrap_or_else(|e| panic!("{e:#?}"))
}

const ESCAPE: &str =
    "- id: escape\n  on: { verb: [escape] }\n  when: not burned\n  then: [{ end: escaped }]\n";
const SURRENDER: &str = "- id: surrender\n  on: { verb: [surrender] }\n  then: [{ end: caught }]\n";

fn run(project: &Project, budget: Budget) -> (Exploration, Vec<Diagnostic>) {
    let world = Arc::new(World::compile(project.story.clone()).unwrap());
    let result = explore(world, budget);
    let diagnostics = result
        .ending_checks
        .diagnostics()
        .into_iter()
        .map(|d| project.locate(d))
        .collect();
    (result, diagnostics)
}

fn full(project: &Project) -> (Exploration, Vec<Diagnostic>) {
    run(project, Budget::default())
}

#[test]
fn a_state_without_a_reachable_ending_is_a_dead_end() {
    // Burning the map closes the way out, and there is no other ending.
    let rules = format!(
        "{ESCAPE}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n"
    );
    let project = cell(&rules, &[]);
    let (result, diagnostics) = full(&project);
    let checks = &result.ending_checks;
    assert!(checks.ran && checks.proven, "{checks:#?}");
    assert_eq!(result.stuck, 1, "{checks:#?}");
    assert_eq!(result.stuck_path, ["burn"]);
    let [dead] = checks.dead_ends.as_slice() else {
        panic!("{checks:#?}")
    };
    assert_eq!(dead.path, ["burn"]);
    assert_eq!(dead.room, "cell");
    assert_eq!(dead.rule.as_deref(), Some("burn_map"));
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
    let [error] = errors.as_slice() else {
        panic!("{diagnostics:#?}")
    };
    assert_eq!(error.file, "rules/cell.yaml");
    assert!(error.message.starts_with("dead end"), "{error}");
    assert!(error.message.contains("\"burn\""), "{error}");
}

#[test]
fn a_story_without_dead_ends_has_no_error() {
    let rules = format!(
        "{ESCAPE}{SURRENDER}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n"
    );
    let (result, diagnostics) = full(&cell(&rules, &[]));
    assert_eq!(result.stuck, 0);
    assert!(result.ending_checks.dead_ends.is_empty());
    assert!(
        !diagnostics.iter().any(Diagnostic::is_error),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_rank_drop_is_a_point_of_no_return() {
    // After burning the map, only the worse ending is left.
    let rules = format!(
        "{ESCAPE}{SURRENDER}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n"
    );
    let (result, diagnostics) = full(&cell(&rules, &[]));
    let [point] = result.ending_checks.no_return.as_slice() else {
        panic!("{:#?}", result.ending_checks)
    };
    assert_eq!(point.path, ["burn"]);
    assert_eq!(point.rule.as_deref(), Some("burn_map"));
    assert_eq!(point.lost, ["escaped"]);
    assert_eq!(point.unaccepted, ["escaped"]);
    assert_eq!((point.rank_before, point.rank_after), (Some(1), Some(3)));
    assert!(!point.accepted());
    let warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    let [warning] = warnings.as_slice() else {
        panic!("{diagnostics:#?}")
    };
    assert_eq!(warning.file, "rules/cell.yaml");
    assert_eq!(warning.at, "rule:burn_map/closes");
    assert!(
        warning.message.starts_with("point of no return"),
        "{warning}"
    );
    assert!(warning.message.contains("`escaped`"), "{warning}");
    // The turn into an ending is not a point of no return: surrender ends the game.
}

#[test]
fn closes_accepts_the_point_of_no_return() {
    let rules = format!(
        "{ESCAPE}{SURRENDER}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n  closes: [escaped]\n  reason: Burning the map strands the player on purpose.\n"
    );
    let project = cell(&rules, &[]);
    assert!(
        check(&project).is_empty(),
        "the schema check takes `closes`: {:#?}",
        check(&project)
    );
    let (result, diagnostics) = full(&project);
    let [point] = result.ending_checks.no_return.as_slice() else {
        panic!("{:#?}", result.ending_checks)
    };
    assert!(point.accepted(), "{point:#?}");
    assert_eq!(point.accepted_by, ["burn_map"]);
    assert_eq!(
        point.reasons,
        ["Burning the map strands the player on purpose."]
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn closes_that_names_the_wrong_ending_is_reported() {
    let rules = format!(
        "{ESCAPE}{SURRENDER}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n  closes: [caught]\n  reason: A wrong list.\n"
    );
    let (result, diagnostics) = full(&cell(&rules, &[]));
    let [point] = result.ending_checks.no_return.as_slice() else {
        panic!("{:#?}", result.ending_checks)
    };
    assert_eq!(point.unaccepted, ["escaped"]);
    let messages: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("point of no return") && m.contains("`escaped`")),
        "{messages:#?}"
    );
    // `caught` stays reachable after the burn: the list names it for nothing.
    assert_eq!(result.ending_checks.stale.len(), 1, "{messages:#?}");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`closes` of `burn_map` names `caught`")),
        "{messages:#?}"
    );
}

#[test]
fn closes_on_a_machine_step_accepts_the_drop() {
    let machine = "start: whole\nsteps:\n  - from: whole\n    on: { verb: [burn] }\n    to: ash\n    closes: [escaped]\n    reason: The map burns on purpose.\n";
    let rules = format!(
        "- id: escape\n  on: {{ verb: [escape] }}\n  when: map == whole\n  then: [{{ end: escaped }}]\n{SURRENDER}"
    );
    let project = cell(&rules, &[("machines/map.yaml", machine)]);
    let (result, diagnostics) = full(&project);
    let [point] = result.ending_checks.no_return.as_slice() else {
        panic!("{:#?}", result.ending_checks)
    };
    assert_eq!(point.accepted_by, ["map#1"]);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    // Without the mark, the warning names the machine file.
    let bare = machine.replace(
        "    closes: [escaped]\n    reason: The map burns on purpose.\n",
        "",
    );
    let (_, diagnostics) = full(&cell(&rules, &[("machines/map.yaml", &bare)]));
    let [warning] = diagnostics.as_slice() else {
        panic!("{diagnostics:#?}")
    };
    assert_eq!(warning.file, "machines/map.yaml");
    assert_eq!(warning.at, "rule:map#1/closes");
}

#[test]
fn closes_accepts_the_turns_before_a_timed_step() {
    // At midnight a rule burns the map; the player needs three turns to get
    // it from the far room. Turns that make the way out too late lose the
    // best ending before the rule fires; its `closes` accepts them too.
    let files = vec![
        file(
            "story.yaml",
            "title: Tunnels\nstart: cell\nclock:\n  start: \"23:45\"\n  minutes_per_turn: 5\nvars:\n  - { id: burned, type: bool, initial: false }\n",
        ),
        file(
            "rooms/cell.yaml",
            "name: Cell\ndescription: [\"Stone and straw.\"]\nexits:\n  - { direction: east, to: tunnel }\n",
        ),
        file(
            "rooms/tunnel.yaml",
            "name: Tunnel\ndescription: [\"A low tunnel.\"]\nexits:\n  - { direction: west, to: cell }\n",
        ),
        file(
            "endings/escaped.yaml",
            "kind: win\nrank: 1\ntitle: Escaped\n",
        ),
        file(
            "endings/caught.yaml",
            "kind: loss\nrank: 3\ntitle: Caught\n",
        ),
        file(
            "verbs/escape.yaml",
            "label: Escape\nwords: [escape]\nslots: none\n",
        ),
        file(
            "verbs/surrender.yaml",
            "label: Surrender\nwords: [surrender]\nslots: none\n",
        ),
        file(
            "rules/tunnels.yaml",
            "- id: escape\n  on: { verb: [escape] }\n  when: in(tunnel) and not burned\n  then: [{ end: escaped }]\n- id: surrender\n  on: { verb: [surrender] }\n  then: [{ end: caught }]\n- id: midnight\n  on: { time: \"00:00\" }\n  then: [{ set: { burned: true } }]\n  closes: [escaped]\n  reason: The guard burns the map at midnight.\n",
        ),
    ];
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let (result, diagnostics) = full(&project);
    let checks = &result.ending_checks;
    assert!(checks.proven, "{checks:#?}");
    assert!(!checks.no_return.is_empty(), "{checks:#?}");
    assert!(checks.no_return.iter().all(|p| p.accepted()), "{checks:#?}");
    // Some of them are turns before midnight, on which the rule does not fire.
    assert!(
        checks
            .no_return
            .iter()
            .any(|p| !p.fired.iter().any(|r| r == "midnight")),
        "{checks:#?}"
    );
    assert!(
        checks
            .no_return
            .iter()
            .all(|p| p.accepted_by == ["midnight"]),
        "{checks:#?}"
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn only_fired_once_rules_tell_states_apart() {
    // A rule that can fire again changes nothing by firing: burning in the
    // cell is the same state as before. A `once` rule cannot fire again, so
    // having fired is a new state.
    let mutter = "- id: mutter\n  on: { verb: [burn] }\n  then: [{ say: You mutter. }]\n";
    let (again, _) = full(&cell(&format!("{ESCAPE}{mutter}"), &[]));
    let once = mutter.replace("  then:", "  once: true\n  then:");
    let (once, _) = full(&cell(&format!("{ESCAPE}{once}"), &[]));
    // The start and the end. With `once`, the start and the end each come
    // twice: before and after the rule fired.
    assert_eq!((again.states, once.states), (2, 4));
}

#[test]
fn a_timer_reaches_its_ending() {
    // The guards come when the fuse runs out. The walk gets there only when
    // it tells the turns left of the timer apart.
    let timer = "turns: 3\nrunning: true\n";
    let rules =
        format!("{ESCAPE}- id: guards\n  on: {{ timer: fuse }}\n  then: [{{ end: caught }}]\n");
    let (result, diagnostics) = full(&cell(&rules, &[("timers/fuse.yaml", timer)]));
    assert!(result.ending_checks.proven, "{:#?}", result.ending_checks);
    let caught = result
        .endings
        .iter()
        .find(|e| e.text.starts_with("Caught"))
        .unwrap_or_else(|| panic!("{:#?}", result.endings));
    // Three turns of anything but `escape`.
    assert_eq!(caught.path.len(), 3, "{:?}", caught.path);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn a_stopped_walk_proves_nothing() {
    let rules = format!(
        "{ESCAPE}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n"
    );
    let project = cell(&rules, &[]);
    // The depth stops the walk at the start: nothing after it is known.
    let (result, diagnostics) = run(
        &project,
        Budget {
            max_states: 2000,
            max_depth: 0,
        },
    );
    let checks = &result.ending_checks;
    assert!(result.truncated);
    assert!(!checks.proven && checks.unproven == 1, "{checks:#?}");
    assert_eq!(result.stuck, 0, "an unfinished state is not a dead end");
    assert!(checks.dead_ends.is_empty() && checks.no_return.is_empty());
    let [warning] = diagnostics.as_slice() else {
        panic!("{diagnostics:#?}")
    };
    assert_eq!(warning.severity, Severity::Warning);
    assert_eq!(warning.file, "story.yaml");
    assert!(warning.message.contains("not proven"), "{warning}");

    // The state budget: the ending after `escape` does not fit, so the start
    // is not proven. The state after `burn` is finished, and it is a dead end.
    let (result, diagnostics) = run(
        &project,
        Budget {
            max_states: 2,
            max_depth: 40,
        },
    );
    let checks = &result.ending_checks;
    assert!(result.truncated && !checks.proven, "{checks:#?}");
    assert_eq!(checks.unproven, 1, "{checks:#?}");
    assert_eq!(result.stuck, 1, "{checks:#?}");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("\"burn\"")),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|d| d.message.contains("not proven")));
}

#[test]
fn a_proven_dead_end_shows_even_when_the_walk_stopped_elsewhere() {
    // A second, endless branch (`wander` counts up) runs into the budget; the
    // burn branch is finished and is still a dead end.
    let rules = format!(
        "{ESCAPE}- id: burn_map\n  on: {{ verb: [burn] }}\n  when: not burned\n  then: [{{ set: {{ burned: true }} }}]\n- id: wander\n  on: {{ verb: [wander] }}\n  when: not burned\n  then: [{{ add: {{ steps: 1 }} }}]\n"
    );
    let project = cell(
        &rules,
        &[(
            "verbs/wander.yaml",
            "label: Wander\nwords: [wander]\nslots: none\n",
        )],
    );
    let (result, diagnostics) = run(
        &project,
        Budget {
            max_states: 2000,
            max_depth: 5,
        },
    );
    assert!(result.truncated && !result.ending_checks.proven);
    assert!(result.stuck >= 1, "{:#?}", result.ending_checks);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("\"burn\"")),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|d| d.message.contains("not proven")));
}

#[test]
fn closes_needs_a_known_ending_and_a_reason() {
    let rules = format!(
        "{ESCAPE}{SURRENDER}- id: burn_map\n  on: {{ verb: [burn] }}\n  then: [{{ set: {{ burned: true }} }}]\n  closes: [escpaed]\n"
    );
    let found = check(&cell(&rules, &[]));
    let messages: Vec<&str> = found.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("unknown ending `escpaed`")),
        "{messages:#?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("needs a `reason`")),
        "{messages:#?}"
    );

    // A blank `reason` without `closes` is no reason at all: no warning.
    let rules = format!("{ESCAPE}{SURRENDER}{BURN}  reason: \"   \"\n");
    let found = check(&cell(&rules, &[]));
    assert!(
        !found.iter().any(|d| d.message.contains("without `closes`")),
        "{found:#?}"
    );
}

#[test]
fn two_commands_into_one_state_are_two_turns() {
    // After `burn, cool, torch`, both `burn` and `torch` lead to the same
    // state. Only `burn` closes the way out on purpose: the `torch` turn
    // into that state is its own, unaccepted point of no return.
    let files = vec![
        file(
            "story.yaml",
            "title: Cell\nstart: cell\nvars:\n  - { id: heat, type: int, initial: 0 }\n  - { id: cooled, type: bool, initial: false }\n",
        ),
        file("rooms/cell.yaml", "name: Cell\ndescription: [\"Stone.\"]\n"),
        file(
            "endings/escaped.yaml",
            "kind: win\nrank: 1\ntitle: Escaped\n",
        ),
        file(
            "endings/caught.yaml",
            "kind: loss\nrank: 3\ntitle: Caught\n",
        ),
        file(
            "verbs/escape.yaml",
            "label: Escape\nwords: [escape]\nslots: none\n",
        ),
        file(
            "verbs/surrender.yaml",
            "label: Surrender\nwords: [surrender]\nslots: none\n",
        ),
        file(
            "verbs/burn.yaml",
            "label: Burn\nwords: [burn]\nslots: none\n",
        ),
        file(
            "verbs/torch.yaml",
            "label: Torch\nwords: [torch]\nslots: none\n",
        ),
        file(
            "verbs/cool.yaml",
            "label: Cool\nwords: [cool]\nslots: none\n",
        ),
        file(
            "rules/cell.yaml",
            "- id: escape\n  on: { verb: [escape] }\n  when: heat < 2\n  then: [{ end: escaped }]\n\
- id: surrender\n  on: { verb: [surrender] }\n  then: [{ end: caught }]\n\
- id: burn_map\n  on: { verb: [burn] }\n  when: heat < 2\n  then: [{ add: { heat: 1 } }]\n  closes: [escaped]\n  reason: On purpose.\n\
- id: torch_map\n  on: { verb: [torch] }\n  when: heat < 2 and cooled\n  then: [{ add: { heat: 1 } }]\n\
- id: cool\n  on: { verb: [cool] }\n  when: heat == 1 and not cooled\n  then: [{ add: { heat: -1 } }, { set: { cooled: true } }]\n",
        ),
    ];
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let (result, _) = full(&project);
    let checks = &result.ending_checks;
    assert!(checks.proven, "{checks:#?}");
    let turns = |rule: &str| {
        checks
            .no_return
            .iter()
            .filter(|p| p.rule.as_deref() == Some(rule))
            .map(|p| (p.accepted(), p.turns))
            .collect::<Vec<_>>()
    };
    // `burn` at heat 1, before and after `cool`; `torch` only after it.
    assert_eq!(turns("burn_map"), [(true, 2)], "{checks:#?}");
    assert_eq!(turns("torch_map"), [(false, 1)], "{checks:#?}");
}

const BURN: &str = "- id: burn_map\n  on: { verb: [burn] }\n  when: not burned\n  then: [{ set: { burned: true } }]\n";

#[test]
fn a_turn_from_an_unfinished_state_proves_nothing() {
    // `wander` runs into the depth limit, so every state before a burn can
    // reach a cut state: its set of endings is incomplete, and the rank drop
    // of `burn` is not reported.
    let rules = format!(
        "{ESCAPE}{SURRENDER}{BURN}- id: wander\n  on: {{ verb: [wander] }}\n  when: not burned\n  then: [{{ add: {{ steps: 1 }} }}]\n"
    );
    let project = cell(
        &rules,
        &[(
            "verbs/wander.yaml",
            "label: Wander\nwords: [wander]\nslots: none\n",
        )],
    );
    let (result, diagnostics) = run(
        &project,
        Budget {
            max_states: 2000,
            max_depth: 5,
        },
    );
    let checks = &result.ending_checks;
    assert!(!checks.proven);
    assert!(checks.no_return.is_empty(), "{checks:#?}");
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.starts_with("point of no return")),
        "{diagnostics:#?}"
    );
}

#[test]
fn closes_into_a_dead_end_is_not_stale() {
    // The burn closes the only ending: a dead end, and the `closes` list
    // names what the rule really loses.
    let rules = format!("{ESCAPE}{BURN}  closes: [escaped]\n  reason: On purpose.\n");
    let (result, diagnostics) = full(&cell(&rules, &[]));
    let checks = &result.ending_checks;
    assert!(checks.proven && checks.dead_ends.len() == 1, "{checks:#?}");
    assert!(checks.stale.is_empty(), "{checks:#?}");
    assert_eq!(
        diagnostics.iter().filter(|d| d.is_error()).count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("no explored turn")),
        "{diagnostics:#?}"
    );
}

#[test]
fn two_closes_rules_with_one_id_do_not_crash() {
    // A duplicate id is a project error, but `explore` can run without `check`.
    let rules = format!(
        "{ESCAPE}{SURRENDER}{BURN}  closes: [escaped]\n  reason: On purpose.\n- id: torch_map\n  on: {{ verb: [torch] }}\n  then: [{{ set: {{ burned: true }} }}]\n  closes: [escaped]\n  reason: Also.\n"
    );
    let mut project = cell(
        &rules,
        &[(
            "verbs/torch.yaml",
            "label: Torch\nwords: [torch]\nslots: none\n",
        )],
    );
    let burn = project
        .story
        .rules
        .iter()
        .find(|r| r.id.as_str() == "burn_map")
        .unwrap()
        .clone();
    project.story.rules.insert(0, burn);
    let (result, _) = full(&project);
    assert!(result.ending_checks.ran);
}

#[test]
fn a_state_without_commands_at_the_depth_limit_is_finished() {
    // The pit has no way out and nothing to do: it is a dead end, also
    // where the depth limit stops the walk.
    let files = vec![
        file("story.yaml", "title: Pit\nstart: edge\n"),
        file(
            "rooms/edge.yaml",
            "name: Edge\ndescription: [\"A ledge.\"]\nexits:\n  - { direction: down, to: pit }\n  - { direction: north, to: road }\n",
        ),
        file(
            "rooms/pit.yaml",
            "name: Pit\ndescription: [\"Smooth walls.\"]\n",
        ),
        file(
            "rooms/road.yaml",
            "name: Road\ndescription: [\"The road home.\"]\n",
        ),
        file("endings/home.yaml", "kind: win\nrank: 1\ntitle: Home\n"),
        file(
            "rules/road.yaml",
            "- id: home\n  on: { enter: road }\n  then: [{ end: home }]\n",
        ),
    ];
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    let (result, diagnostics) = run(
        &project,
        Budget {
            max_states: 2000,
            max_depth: 1,
        },
    );
    assert!(result.ending_checks.proven, "{:#?}", result.ending_checks);
    assert_eq!(result.stuck, 1);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.is_error() && d.message.contains("\"go down\"")),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_fragment_key_with_hash_finds_its_own_room() {
    // `room:vault#1` is fragment 1 of the room `vault`, also when a room
    // with the id `vault#1` exists; only rule keys are taken whole.
    let mut project = cell(ESCAPE, &[]);
    project.origins.insert(
        "room:vault".into(),
        loremaster_project::Origin::yaml("rooms/vault.yaml"),
    );
    project.origins.insert(
        "room:vault#1".into(),
        loremaster_project::Origin::yaml("rooms/other.yaml"),
    );
    let d = project.locate(Diagnostic::warning("", "room:vault#1/when", "x"));
    assert_eq!(d.file, "rooms/vault.yaml");
}
