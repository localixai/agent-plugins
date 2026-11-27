//! Port of `src/format/mcp-tool-schema.ts`, switched.
//!
//! What Claude Code does with a tool's input schema before the tool goes to the API (mcp.md, "Tool
//! input schemas with a root-level combinator", "Tools with invalid input schemas", "Tools whose input
//! schema the API refuses"): a root `anyOf`, `oneOf` or `allOf` is flattened into one object, a tool
//! whose schema the API would refuse is excluded (and Claude is told), and a tool the API did refuse
//! is declared without parameters. Pure over data; the log is returned, never written.
//!
//! A tool is a `JsValue` object (`Pick<Tool, 'name' | 'description'>` with an `inputSchema`): the
//! server's own object, which a tool sent as it came keeps whole.

use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

mod flatten;

use flatten::{COMBINATORS, flatten, group_sentence, has_root_combinator};

use super::mcp_schema_check::{checks_meta_schema, meta_schema_problems, property_name_problems};
use crate::js::{JsObject, JsValue, in_property_order, property_order};

/// A tool as it goes to the API (`McpToolDeclaration`): `name`, `description`, `inputSchema`.
pub type McpToolDeclaration = JsObject;

/// What the host's policy allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolPolicy {
    /// The remote configuration that enables the rewrite of root-level combinators was received.
    pub rewrite_root_combinators: bool,
    /// The checks of "Tools with invalid input schemas" exclude the tool: false "when feature flag
    /// fetching is off or the flags have never arrived" (the tool is sent and the log records it).
    pub exclude_invalid: bool,
}

/// Whether a tool is sent, as the tool or the reasons it is not.
#[derive(Clone, Debug, PartialEq)]
pub enum McpToolPreparation {
    /// `{ sent: true, tool, log }`.
    Sent {
        /// The tool as it goes to the API.
        tool: McpToolDeclaration,
        /// What the server's log records.
        log: Vec<String>,
    },
    /// `{ sent: false, reasons, log }`.
    NotSent {
        /// Why the tool is excluded.
        reasons: Vec<String>,
        /// What the server's log records.
        log: Vec<String>,
    },
}

impl Serialize for McpToolPreparation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        match self {
            Self::Sent { tool, log } => {
                map.serialize_entry("sent", &true)?;
                map.serialize_entry("tool", tool)?;
                map.serialize_entry("log", log)?;
            }
            Self::NotSent { reasons, log } => {
                map.serialize_entry("sent", &false)?;
                map.serialize_entry("reasons", reasons)?;
                map.serialize_entry("log", log)?;
            }
        }
        map.end()
    }
}

fn without_combinators(schema: &JsObject) -> JsObject {
    property_order(schema)
        .into_iter()
        .filter(|(key, _)| !COMBINATORS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn rewritten(tool: &JsObject, input_schema: &JsObject) -> Result<McpToolDeclaration, String> {
    let flat = flatten(input_schema)?;
    let mut required: Vec<String> = Vec::new();
    for name in &flat.required {
        if !required.contains(name) {
            required.push(name.clone());
        }
    }
    let sentence = group_sentence(&flat.groups);
    let description: Vec<&str> = sentence
        .as_deref()
        .into_iter()
        .chain(tool.get("description").and_then(JsValue::as_str))
        .collect();
    let description = description.join(" ");
    let mut schema = without_combinators(input_schema);
    schema.insert("type".to_owned(), "object".into());
    schema.insert("properties".to_owned(), JsValue::Object(flat.properties));
    if !required.is_empty() {
        schema.insert(
            "required".to_owned(),
            JsValue::Array(required.into_iter().map(JsValue::String).collect()),
        );
    }
    let mut declared = JsObject::new();
    declared.insert(
        "name".to_owned(),
        tool.get("name").cloned().unwrap_or_default(),
    );
    if !description.is_empty() {
        declared.insert("description".to_owned(), description.into());
    }
    declared.insert(
        "inputSchema".to_owned(),
        JsValue::Object(in_property_order(schema)),
    );
    Ok(declared)
}

/// The problems the API would refuse the schema for: property names always, the meta-schema for its dialects.
fn schema_problems(schema: &JsValue) -> Vec<String> {
    let mut problems = property_name_problems(schema);
    if checks_meta_schema(schema) {
        problems.extend(meta_schema_problems(schema));
    }
    problems
}

const WOULD_BE_REJECTED: &str = "input schema would be rejected by the Anthropic API";

fn excluded(name: &str, reasons: Vec<String>) -> McpToolPreparation {
    let log = reasons
        .iter()
        .map(|reason| format!("Tool \"{name}\" skipped: {reason}"))
        .collect();
    McpToolPreparation::NotSent { reasons, log }
}

/// The tool as it goes to the API, or why it does not. Judgments (spec/judgments.md): a branch that is
/// not an object schema, a `$ref` branch or an empty combinator cannot be flattened and skips the tool;
/// properties two branches define differently are kept nested in an `anyOf`; the wording of the log
/// lines other than the phrases the docs quote.
#[must_use]
pub fn prepare_mcp_tool(listed: &JsObject, policy: McpToolPolicy) -> McpToolPreparation {
    let name = listed
        .get("name")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let Some(JsValue::Object(input_schema)) = listed.get("inputSchema") else {
        return excluded(name, vec!["the input schema is not an object".to_owned()]);
    };
    let mut prepared = listed.clone();
    if has_root_combinator(input_schema) {
        if !policy.rewrite_root_combinators {
            return excluded(
                name,
                vec![
                    "the input schema has a root-level combinator and the rewrite is not enabled"
                        .to_owned(),
                ],
            );
        }
        match rewritten(listed, input_schema) {
            Ok(flat) => prepared = flat,
            Err(why) => {
                return excluded(
                    name,
                    vec![format!(
                        "the root-level combinator cannot be flattened: {why}"
                    )],
                );
            }
        }
    }
    let problems = schema_problems(prepared.get("inputSchema").unwrap_or(&JsValue::Undefined));
    if problems.is_empty() {
        return McpToolPreparation::Sent {
            tool: prepared,
            log: Vec::new(),
        };
    }
    if policy.exclude_invalid {
        return excluded(name, problems);
    }
    McpToolPreparation::Sent {
        tool: prepared,
        log: vec![format!("Tool \"{name}\": {WOULD_BE_REJECTED}")],
    }
}

/// A tool the host excluded and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedTool {
    /// The tool's name.
    pub name: String,
    /// Why it was excluded.
    pub reasons: Vec<String>,
}

/// What Claude is told about the tools Claude Code excluded: "tells Claude which tools it excluded and
/// why, so you can ask Claude why a tool is missing". The wording is the kit's. `None` for none.
#[must_use]
pub fn excluded_tools_notice(tools: &[ExcludedTool]) -> Option<String> {
    if tools.is_empty() {
        return None;
    }
    let lines: Vec<String> = tools
        .iter()
        .map(|tool| format!("- {}: {}", tool.name, tool.reasons.join("; ")))
        .collect();
    Some(format!(
        "These MCP tools were excluded because the API would refuse their input schema:\n{}",
        lines.join("\n")
    ))
}

/// The tool the API refused, resent without parameters, and the line for the server's log.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RefusedSchemaFallback {
    /// The tool with no parameters.
    pub tool: McpToolDeclaration,
    /// The server's log line.
    pub log: String,
}

/// The tool the API refused, resent "with the tool declared without parameters", and the line for the
/// server's log: "the API refused this tool's input schema ... The log line ends with `The API said:`
/// and the API's reason".
#[must_use]
pub fn refused_schema_fallback(tool: &JsObject, api_reason: &str) -> RefusedSchemaFallback {
    let name = tool.get("name").cloned().unwrap_or_default();
    let mut declared = JsObject::new();
    declared.insert("name".to_owned(), name.clone());
    if let Some(description) = tool.get("description").filter(|each| !each.is_undefined()) {
        declared.insert("description".to_owned(), description.clone());
    }
    declared.insert(
        "inputSchema".to_owned(),
        JsValue::from([
            ("type", "object".into()),
            ("properties", JsValue::Object(JsObject::new())),
        ]),
    );
    RefusedSchemaFallback {
        tool: declared,
        log: format!(
            "Tool \"{}\": the API refused this tool's input schema and the tool is sent without parameters. The API said: {api_reason}",
            name.as_str().unwrap_or_default()
        ),
    }
}

crate::registry::exports! {
    "format/mcp-tool-schema" {
        "prepareMcpTool" => ok |listed: JsObject, policy: McpToolPolicy| prepare_mcp_tool(&listed, policy);
        "excludedToolsNotice" => ok |tools: Vec<ExcludedTool>| excluded_tools_notice(&tools);
        "refusedSchemaFallback" => ok |tool: JsObject, api_reason: String| refused_schema_fallback(&tool, &api_reason);
    }
}
