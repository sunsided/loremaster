//! The dependency chart of a walkthrough (CK-7): its steps with an id, what
//! each one needs, and how many steps the player can work on at each point
//! (the puzzle width, US-43).
//!
//! The chart comes from `needs` and `before` of the steps (CK-4). A step
//! without `needs` can be done from the start. The checker reports unknown
//! ids and a `needs` that points forward; the chart skips them.

use crate::{Id, Walkthrough};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How many steps in a row with width 1 make the walkthrough "in series
/// only" (the CK-7 lint).
pub const SERIAL_RUN: usize = 5;

/// The dependency chart of one walkthrough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepChart {
    /// The key of the walkthrough: its file, or its Rust place.
    pub walkthrough: String,
    /// The name of the walkthrough, or its key.
    pub name: String,
    /// The steps with an id, in the order of the walkthrough.
    pub steps: Vec<ChartStep>,
    /// The largest width of a step.
    pub max_width: usize,
}

/// A step of the chart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartStep {
    pub id: Id,
    /// The command of the step.
    pub command: String,
    /// The index of the step in the walkthrough, from 0.
    pub index: usize,
    /// The room of the step, when the step names it with `at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Id>,
    /// The steps that this one needs: its `needs`, and the steps that name
    /// it in `before`. Only known, earlier steps.
    pub needs: Vec<Id>,
    /// The longest chain of needs before this step: 0 for a step that needs
    /// nothing. The chart draws one column per depth.
    pub depth: usize,
    /// How many steps the player can work on when this step comes up: the
    /// steps not done yet whose needs are all done. At least 1.
    pub width: usize,
    /// How many hints the step has.
    pub hints: usize,
}

impl StepChart {
    /// The runs of at least `min` steps in a row with width 1, as index
    /// ranges into `steps` (first and last, inclusive).
    pub fn serial_runs(&self, min: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = None;
        for (i, s) in self.steps.iter().enumerate() {
            match (s.width == 1, start) {
                (true, None) => start = Some(i),
                (false, Some(from)) => {
                    if i - from >= min {
                        out.push((from, i - 1));
                    }
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(from) = start
            && self.steps.len() - from >= min
        {
            out.push((from, self.steps.len() - 1));
        }
        out
    }

    /// The mean width over the steps, 0 for an empty chart.
    pub fn mean_width(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.steps.iter().map(|s| s.width).sum::<usize>() as f32 / self.steps.len() as f32
    }
}

/// The dependency chart of a walkthrough, or `None` when no step has an id.
pub fn step_chart(key: &str, walk: &Walkthrough) -> Option<StepChart> {
    // Each id with its position among the steps with an id. A second step
    // with the same id is an error of the checker; the first one counts.
    let mut position: BTreeMap<Id, usize> = BTreeMap::new();
    let mut steps = Vec::new();
    for (index, step) in walk.steps.iter().enumerate() {
        let (Some(plan), Some(id)) = (step.plan(), step.id()) else {
            continue;
        };
        if position.contains_key(id) {
            continue;
        }
        position.insert(id.clone(), steps.len());
        steps.push(ChartStep {
            id: id.clone(),
            command: step.command().to_string(),
            index,
            at: plan.at.clone(),
            needs: Vec::new(),
            depth: 0,
            width: 0,
            hints: plan.hints.len(),
        });
    }
    if steps.is_empty() {
        return None;
    }
    // Needs: a step's own `needs`, and `before` of the earlier steps.
    let mut needs: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];
    for (index, step) in walk.steps.iter().enumerate() {
        let (Some(plan), Some(id)) = (step.plan(), step.id()) else {
            continue;
        };
        let me = position[id];
        // A second step with the same id is not in the chart.
        if steps[me].index != index {
            continue;
        }
        for n in &plan.needs {
            if let Some(&p) = position.get(n)
                && p < me
                && !needs[me].contains(&p)
            {
                needs[me].push(p);
            }
        }
        for b in &plan.before {
            if let Some(&p) = position.get(b)
                && p > me
                && !needs[p].contains(&me)
            {
                needs[p].push(me);
            }
        }
    }
    for (i, list) in needs.iter_mut().enumerate() {
        list.sort_unstable();
        steps[i].needs = list.iter().map(|&p| steps[p].id.clone()).collect();
    }
    // Needs point backward, so one pass in order gives the depth.
    for i in 0..steps.len() {
        steps[i].depth = needs[i]
            .iter()
            .map(|&p| steps[p].depth + 1)
            .max()
            .unwrap_or(0);
    }
    // Before step i, the steps before it are done. The open steps are the
    // ones from i on whose needs all come before i.
    for i in 0..steps.len() {
        steps[i].width = (i..steps.len())
            .filter(|&j| needs[j].iter().all(|&p| p < i))
            .count();
    }
    let max_width = steps.iter().map(|s| s.width).max().unwrap_or(0);
    Some(StepChart {
        walkthrough: key.to_string(),
        name: walk.name.clone().unwrap_or_else(|| key.to_string()),
        steps,
        max_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn walk(steps: serde_json::Value) -> Walkthrough {
        serde_json::from_value(serde_json::json!({ "steps": steps })).unwrap()
    }

    #[test]
    fn the_chart_has_the_steps_with_an_id_and_their_needs() {
        let w = walk(serde_json::json!([
            "n",
            { "id": "key", "do": "open drawer" },
            { "id": "lamp", "do": "take lamp", "before": ["lit"] },
            { "id": "lit", "do": "light lamp" },
            { "id": "door", "do": "unlock door", "needs": "key" },
            { "id": "beacon", "do": "light beacon", "needs": ["lit", "door"] },
        ]));
        let chart = step_chart("tests/win.yaml", &w).unwrap();
        let ids: Vec<&str> = chart.steps.iter().map(|s| s.id.as_str()).collect();
        check!(ids == ["key", "lamp", "lit", "door", "beacon"]);
        check!(chart.steps[2].needs == [Id::new("lamp")]);
        check!(chart.steps[4].depth == 2);
        // key, lamp and door's prerequisite are free at the start.
        let widths: Vec<usize> = chart.steps.iter().map(|s| s.width).collect();
        check!(widths == [2, 2, 2, 1, 1]);
        check!(chart.max_width == 2);
        check!(chart.serial_runs(2) == [(3, 4)]);
        check!(chart.serial_runs(3).is_empty());
    }

    #[test]
    fn a_walkthrough_without_ids_has_no_chart() {
        check!(step_chart("t", &walk(serde_json::json!(["n", "s"]))).is_none());
    }

    #[test]
    fn a_chain_has_width_one() {
        let w = walk(serde_json::json!([
            { "id": "a", "do": "a" },
            { "id": "b", "do": "b", "needs": "a" },
            { "id": "c", "do": "c", "needs": "b" },
        ]));
        let chart = step_chart("t", &w).unwrap();
        check!(chart.serial_runs(3) == [(0, 2)]);
    }
}
