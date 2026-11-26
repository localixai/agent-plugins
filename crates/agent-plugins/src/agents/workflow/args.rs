//! The core's counterpart of `src/agents/workflow/args.ts` at sha256:2d83f3507c00, which stays TypeScript:
//! it decides whether any host value (a function, a cycle, a bigint included) is structured data,
//! which no value crossing the boundary can be (docs/RUST.md, "What stays TypeScript").
//!
//! Input to a saved workflow (workflows.md, "Pass input to a saved workflow"): "A saved workflow can
//! accept input through the `args` parameter. The script reads it as a global named `args`", passed
//! "as structured data, so the script can call array and object methods on `args` directly without
//! parsing it first. If `args` is omitted, the global is `undefined` inside the script."

use serde::Serialize;

use crate::format::issues::{PluginIssue, error};
use crate::foundation::json_value::to_json_value;
use crate::js::JsValue;

/// The `args` global and what was wrong with what was passed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkflowArgs {
    /// The value of the `args` global; `undefined` when none was passed.
    pub args: JsValue,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// The `args` global for a call. Data stays data: a list is a list and a string is a string, nothing is
/// parsed. A value JSON cannot hold (a function, a cycle, a bigint) cannot cross into the isolated
/// runtime, so it is refused (judgment: the docs say structured data, not what is refused).
#[must_use]
pub fn workflow_args(passed: &JsValue) -> WorkflowArgs {
    if passed.is_undefined() {
        return WorkflowArgs {
            args: JsValue::Undefined,
            issues: Vec::new(),
        };
    }
    if let Some(args) = to_json_value(passed) {
        return WorkflowArgs {
            args,
            issues: Vec::new(),
        };
    }
    WorkflowArgs {
        args: JsValue::Undefined,
        issues: vec![error(
            "args",
            "args must be structured data: JSON values only",
        )],
    }
}
