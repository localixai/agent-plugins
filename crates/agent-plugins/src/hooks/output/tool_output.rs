//! Port of `src/hooks/output/tool-output.ts`, switched.
//!
//! The output shape of the built-in tools a `PostToolUse` hook may replace (hooks.md, "PostToolUse
//! decision control"): "For built-in tools, a value that doesn't match the tool's output schema is
//! ignored and the original output is used. MCP tool output is passed through without schema
//! validation." Agent Plugins Extended J7.138: the shape is the tool's output type in the SDK's
//! `sdk-tools.d.ts`, where one is defined; a tool without one passes unchecked. The shapes are read
//! from that file (`tool_shapes/`, written by `scripts/tool-output-shapes.ts`), every nested field
//! included; a field the type does not declare is allowed, as the types are open.

use serde::Serialize;

use super::tool_shape::shape_problem;
use super::tool_shapes::TOOL_SHAPES;
use crate::js::JsValue;

/// The tools whose output shape the kit checks: every built-in tool the SDK gives an output type.
#[must_use]
pub fn checked_tool_outputs() -> Vec<String> {
    TOOL_SHAPES.keys().cloned().collect()
}

/// `mcp__<server>__<tool>`: "MCP tool output is passed through without schema validation".
#[must_use]
pub fn is_mcp_tool_name(name: &str) -> bool {
    name.starts_with("mcp__")
}

/// Whether a value may replace the tool's output; `message` says why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolOutputCheck {
    /// Whether it may.
    pub accepted: bool,
    /// The first place it breaks the tool's shape, when it may not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Whether `value` may replace the output of the built-in tool `tool_name`.
#[must_use]
pub fn check_tool_output(tool_name: &str, value: &JsValue) -> ToolOutputCheck {
    let Some(shape) = TOOL_SHAPES.get(tool_name) else {
        return ToolOutputCheck {
            accepted: true,
            message: None,
        };
    };
    let problem = shape_problem(shape, value, "");
    ToolOutputCheck {
        accepted: problem.is_none(),
        message: problem,
    }
}

crate::registry::exports! {
    "hooks/output/tool-output" {
        "CHECKED_TOOL_OUTPUTS" => ok checked_tool_outputs;
    }
}
