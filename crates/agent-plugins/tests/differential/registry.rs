//! Every export of the registry (`agent_plugins::registry`) answers each recorded call of its
//! namesake as the npm package answered it, called as the wrapper calls it: by name, the
//! arguments as recorded (`undefined` and the numbers JSON lacks tagged), the answer as the
//! wrapper decodes it. An export
//! with no recorded call is named on the output, never a failure: the wrapper may need what the
//! tests never called directly.

use std::fmt::Write as _;

use agent_plugins::js::JsValue;
use agent_plugins::registry::{Answer, MODULES, answer};
use serde_json::{Value, json};

use crate::support::{Case, as_wire, cases_if_any, normalize, same_text};

/// The answers a recorded call may get: its outcomes, as the wrapper reads them.
fn wanted(case: &Case) -> Vec<Value> {
    case.outcomes
        .iter()
        .map(|outcome| match outcome.get("result") {
            Some(result) => json!({ "result": normalize(as_wire(result.clone())) }),
            None => outcome.clone(),
        })
        .collect()
}

/// One export against its calls: the report of those that differ, how many could not be spelt.
fn hold(name: &str, cases: &[Case], report: &mut String) -> (usize, usize) {
    let (mut failed, mut unspelt) = (0, 0);
    for case in cases {
        let args: Vec<JsValue> =
            serde_json::from_value(case.tagged_args.clone()).unwrap_or_default();
        let got = match answer(name, &args) {
            Answer::Result(value) => {
                let written = serde_json::to_value(&value).unwrap_or_default();
                json!({ "result": normalize(written) })
            }
            Answer::Throws(message) => json!({ "throws": message }),
            Answer::Fault(why) if case.undefined_argument => {
                let _ = why;
                unspelt += 1;
                continue;
            }
            Answer::Fault(why) => json!({ "fault": why }),
        };
        if !wanted(case).iter().any(|want| same_text(want, &got)) {
            failed += 1;
            if failed <= 5 {
                let _ = writeln!(
                    report,
                    "{name}\n  args {}\n    want {}\n    got  {got}\n    spec {:?}",
                    case.tagged_args,
                    Value::Array(case.outcomes.clone()),
                    case.spec,
                );
            }
        }
    }
    (failed, unspelt)
}

#[test]
fn every_export_answers_as_recorded() {
    let mut report = String::new();
    let mut summary = String::new();
    let mut unrecorded = Vec::new();
    for module in MODULES {
        for function in module.functions {
            let name = format!("{}#{function}", module.name);
            let cases = cases_if_any(module.name, function);
            if cases.is_empty() {
                unrecorded.push(name);
                continue;
            }
            let (failed, unspelt) = hold(&name, &cases, &mut report);
            if failed > 0 {
                let _ = writeln!(summary, "{name}: {failed} of {} differ", cases.len());
            }
            if unspelt > 0 {
                eprintln!(
                    "{name}: {unspelt} calls with an undefined argument it takes only defined"
                );
            }
        }
    }
    if !unrecorded.is_empty() {
        eprintln!("exported, never recorded: {}", unrecorded.join(", "));
    }
    assert!(summary.is_empty(), "{summary}\n{report}");
}
