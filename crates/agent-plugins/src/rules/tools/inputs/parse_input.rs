//! Port of `src/rules/tools/inputs/parse-input.ts`, switched.
//!
//! A hook's `tool_input` read against its tool's table.

use serde::{Deserialize, Serialize};

use super::schemas::INPUT_SCHEMAS;
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsObject, JsValue, string_of};
use crate::protocol::is_record::is_record;
use crate::zod::Issue;

/// What reading a `tool_input` gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputParse {
    /// The input as sent, a copy; `{}` when it was not an object.
    pub input: JsObject,
    /// Each problem, at `tool_input.<field>` (`tool_input` for the input itself).
    pub issues: Vec<PluginIssue>,
}

const ROOT: &str = "tool_input";

fn issue_of(found: &Issue) -> PluginIssue {
    let path: Vec<String> = std::iter::once(ROOT.to_owned())
        .chain(found.path().iter().map(string_of))
        .collect();
    error(&path.join("."), found.message())
}

/// Reads a hook's `tool_input` against the table of its tool (hooks.md, "PreToolUse input"): a wrong
/// type or a missing required field is an issue and nothing throws. The input comes back whole, so a
/// hook can still read what the table does not list. A tool without a table (an MCP tool, `Skill`) is
/// returned as it is (spec/judgments.md).
#[must_use]
pub fn parse_tool_input(tool: &str, raw: &JsValue) -> InputParse {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return InputParse {
            input: JsObject::new(),
            issues: vec![error(ROOT, "expected an object")],
        };
    };
    let input = members.clone();
    let result = INPUT_SCHEMAS
        .get(tool)
        .map(|schema| schema.safe_parse(&JsValue::from(input.clone())));
    let issues = match result {
        Some(Err(failure)) => failure.issues.iter().map(issue_of).collect(),
        None | Some(Ok(_)) => Vec::new(),
    };
    InputParse { input, issues }
}

crate::registry::exports! {
    "rules/tools/inputs/parse-input" {
        "parseToolInput" => ok |tool: String, raw: JsValue| parse_tool_input(&tool, &raw);
    }
}
