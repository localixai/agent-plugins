//! `agent_plugins::zod` held to zod 4 itself: every schema of the oracle
//! (`scripts/rust/js-oracle/zod-schemas.ts`, built again in `zod_schemas.rs`) on every recorded
//! input answers what `safeParse` answered, its output or its issues, member for member and in
//! order; and `error.message` is the text `parse` threw.

use std::fmt::Write as _;
use std::path::PathBuf;

use agent_plugins::js::JsValue;
use agent_plugins::zod::{Issue, Schema};
use serde_json::{Value, json};

use super::zod_schemas::schemas;
use crate::support::{normalize, same_text};

fn lines() -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/js/zod.jsonl");
    std::fs::read_to_string(path)
        .expect("the zod oracle is recorded: bun scripts/rust/js-oracle.ts")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("a vector line is JSON"))
        .collect()
}

fn value_of(recorded: &Value) -> JsValue {
    serde_json::from_value(recorded.clone()).expect("a recorded input is a JavaScript value")
}

fn wire(value: &impl serde::Serialize) -> Value {
    normalize(serde_json::to_value(value).expect("a value encodes"))
}

fn answer(schema: &Schema, input: &JsValue) -> Value {
    match schema.safe_parse(input) {
        Ok(data) => json!({ "success": true, "data": wire(&data) }),
        Err(failure) => {
            let issues: Vec<&Issue> = failure.issues.iter().collect();
            json!({ "success": false, "issues": wire(&issues) })
        }
    }
}

fn schema_named<'a>(all: &'a [(&'static str, Schema)], name: &str) -> Option<&'a Schema> {
    all.iter()
        .find(|(known, _)| *known == name)
        .map(|(_, schema)| schema)
}

#[test]
fn the_schemas_are_the_oracles_in_its_order() {
    let mut recorded: Vec<String> = Vec::new();
    for line in lines() {
        let name = line["fn"].as_str().unwrap_or_default().to_owned();
        if name != "message" && recorded.last() != Some(&name) {
            recorded.push(name);
        }
    }
    let built: Vec<&str> = schemas().iter().map(|(name, _)| *name).collect();
    assert_eq!(built, recorded);
}

#[test]
fn every_schema_answers_as_zod() {
    let all = schemas();
    let (mut failed, mut total, mut report) = (0usize, 0usize, String::new());
    for line in lines().iter().filter(|line| line["fn"] != "message") {
        total += 1;
        let name = line["fn"].as_str().unwrap_or_default();
        let input = value_of(&line["args"][0]);
        let Some(schema) = schema_named(&all, name) else {
            failed += 1;
            let _ = writeln!(report, "  no Rust schema named {name}");
            continue;
        };
        let got = answer(schema, &input);
        let want = normalize(line["outcomes"][0]["result"].clone());
        if !same_text(&want, &got) {
            failed += 1;
            if failed <= 12 {
                let _ = writeln!(
                    report,
                    "  {name} on {}\n    want {want}\n    got  {got}",
                    line["args"][0]
                );
            }
        }
    }
    assert!(failed == 0, "zod: {failed} of {total} differ\n{report}");
}

#[test]
fn the_error_message_is_what_parse_throws() {
    let all = schemas();
    for line in lines().iter().filter(|line| line["fn"] == "message") {
        let name = line["args"][0].as_str().unwrap_or_default();
        let input = value_of(&line["args"][1]);
        let schema = schema_named(&all, name);
        assert!(schema.is_some(), "no Rust schema named {name}");
        let text = schema
            .and_then(|schema| schema.parse(&input).err())
            .map(|failure| failure.to_string())
            .unwrap_or_default();
        assert_eq!(
            text,
            line["outcomes"][0]["result"].as_str().unwrap_or_default(),
            "{name}"
        );
    }
}
