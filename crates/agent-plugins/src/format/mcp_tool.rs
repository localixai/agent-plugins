//! Port of `src/format/mcp-tool.ts`, switched.
//!
//! What a server's `tools/list` entry says about a tool and the name Claude sees (mcp.md, "Tools
//! with names longer than 128 characters", "Require approval for a specific tool", "Mark tools
//! that are safe to call twice", "Raise the limit for a specific tool", "Plugin-provided MCP
//! servers"; plugins/components.md, "Server names, tool names, and reloads"). Pure data: listing
//! the tools, calling them and prompting are the host's. The shapes of a tool's input schema are
//! in `mcp-tool-schema`.

use crate::js::JsValue;
use indexmap::IndexSet;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

use crate::js::code_unit_length;

const PREFIX: &str = "mcp__";
const SEPARATOR: &str = "__";

/// `my.server` is `my_server`: the characters outside `A-Za-z0-9_-` become `_` (mcp.md, "Plugin
/// MCP tool names"). The expression is JavaScript's without `u`, so a character above U+FFFF is
/// two code units and becomes two.
#[must_use]
pub fn normalize_mcp_server_name(name: &str) -> String {
    let mut out = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
            out.push(character);
        } else {
            out.extend(std::iter::repeat_n('_', character.len_utf16()));
        }
    }
    out
}

/// The server name of a plugin's MCP server (mcp.md, "Plugin-provided MCP servers"): the plugin
/// name and the server key under one prefix, normalised as a whole, so `database-tools` of
/// `my-plugin` is `plugin_my-plugin_database-tools`.
#[must_use]
pub fn plugin_mcp_server_name(plugin_name: &str, server_key: &str) -> String {
    normalize_mcp_server_name(&format!("plugin_{plugin_name}_{server_key}"))
}

/// The callable name of a tool of `server`: `mcp__<server>__<tool>`.
#[must_use]
pub fn mcp_tool_name(server: &str, tool: &str) -> String {
    format!(
        "{PREFIX}{}{SEPARATOR}{tool}",
        normalize_mcp_server_name(server)
    )
}

/// "Claude Code leaves an MCP tool out of the tools it sends to Claude when its full name is over
/// 128 characters."
pub const MCP_TOOL_NAME_LIMIT: usize = 128;

/// Whether a tool goes to the model, and the debug line when it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpSendCheck {
    /// `{ sent: true }`.
    Sent,
    /// `{ sent: false, log }`.
    NotSent {
        /// The debug line.
        log: String,
    },
}

impl Serialize for McpSendCheck {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Sent => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("sent", &true)?;
                map.end()
            }
            Self::NotSent { log } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("sent", &false)?;
                map.serialize_entry("log", log)?;
                map.end()
            }
        }
    }
}

/// Whether a tool goes to the model, and the debug line when it does not (errors.md, "MCP tool
/// name over 128 characters"). The count is the name's length in UTF-16 code units, as the API
/// counts a string (spec/judgments.md: the docs say "characters").
#[must_use]
pub fn mcp_tool_send_check(full_name: &str) -> McpSendCheck {
    let length = code_unit_length(full_name);
    if length <= MCP_TOOL_NAME_LIMIT {
        return McpSendCheck::Sent;
    }
    McpSendCheck::NotSent {
        log: format!(
            "Tool \"{full_name}\" is not sent to Claude: its name has {length} characters and the API accepts at most {MCP_TOOL_NAME_LIMIT}"
        ),
    }
}

/// What a tool's entry declares, read once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's McpToolHints has these four flags"
)]
pub struct McpToolHints {
    /// `annotations.readOnlyHint` is `true`: "the tool changes nothing".
    pub read_only: bool,
    /// `annotations.idempotentHint` is `true`: calling it again with the same arguments is safe.
    pub idempotent: bool,
    /// Either hint: Claude Code can repeat a call that was pending when a connection closed.
    pub safe_to_repeat: bool,
    /// "Claude Code also runs the tool's calls in parallel with other read-only calls":
    /// `readOnlyHint` only.
    pub parallel_with_read_only: bool,
    /// `_meta["anthropic/maxResultSizeChars"]`, cut to the ceiling; absent when the tool declares
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_result_size_chars: Option<u64>,
    /// `_meta["anthropic/requiresUserInteraction"]` is the JSON Boolean `true`.
    pub requires_user_interaction: bool,
}

/// "Up to a hard ceiling of 500,000 characters."
pub const MCP_RESULT_SIZE_CEILING: u64 = 500_000;

const MAX_RESULT_SIZE: &str = "anthropic/maxResultSizeChars";
const REQUIRES_INTERACTION: &str = "anthropic/requiresUserInteraction";

fn member_of<'a>(value: Option<&'a JsValue>, key: &str) -> Option<&'a JsValue> {
    value
        .and_then(JsValue::as_object)
        .and_then(|members| members.get(key))
}

/// Judgment (spec/judgments.md): a size that is not a positive finite number declares no limit
/// and a fraction is cut to a whole number of characters; the docs only describe a number up to
/// the ceiling.
fn size_of(value: Option<&JsValue>) -> Option<u64> {
    let number = value?.as_f64()?;
    if !number.is_finite() || number < 1.0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "at least 1 and cut to the ceiling first"
    )]
    let whole = number.floor().min(MCP_RESULT_SIZE_CEILING as f64) as u64;
    Some(whole)
}

fn is_true(value: Option<&JsValue>) -> bool {
    value == Some(&JsValue::Bool(true))
}

/// The hints of one `tools/list` entry. A hint counts only as the JSON value `true` ("The value
/// must be the JSON Boolean `true`; any other value is ignored" is said of
/// `requiresUserInteraction`; the other two are "set ... to `true`").
#[must_use]
pub fn read_mcp_tool_hints(entry: &JsValue) -> McpToolHints {
    let annotations = member_of(Some(entry), "annotations");
    let meta = member_of(Some(entry), "_meta");
    let read_only = is_true(member_of(annotations, "readOnlyHint"));
    let idempotent = is_true(member_of(annotations, "idempotentHint"));
    let size = size_of(member_of(meta, MAX_RESULT_SIZE));
    McpToolHints {
        read_only,
        idempotent,
        safe_to_repeat: read_only || idempotent,
        parallel_with_read_only: read_only,
        max_result_size_chars: size,
        requires_user_interaction: is_true(member_of(meta, REQUIRES_INTERACTION)),
    }
}

/// Whether a server lists a tool for Claude (mcp.md, "Tool availability": "The server lists the
/// tool for its own interface and not for Claude ... add `model` to the tool's
/// `_meta.ui.visibility` to give Claude the tool"). Judgment (spec/judgments.md): an entry with no
/// `_meta.ui.visibility` array, or with a malformed one, is visible; an array without `model`
/// hides the tool from Claude.
#[must_use]
pub fn model_visible_mcp_tool(entry: &JsValue) -> bool {
    let visibility = member_of(
        member_of(member_of(Some(entry), "_meta"), "ui"),
        "visibility",
    );
    match visibility.and_then(JsValue::as_array) {
        None => true,
        Some(items) => items.iter().any(|item| item.as_str() == Some("model")),
    }
}

/// The callable names of the tools of one server that "require explicit approval on every call",
/// for `DecideContext.requiresUserInteraction` of the permission check. `server` is the name the
/// tools are registered under (`plugin_mcp_server_name` for a plugin's).
#[must_use]
pub fn requiring_interaction(server: &str, tools: &[JsValue]) -> IndexSet<String> {
    tools
        .iter()
        .filter_map(|entry| {
            let name = member_of(Some(entry), "name")?.as_str()?;
            read_mcp_tool_hints(entry)
                .requires_user_interaction
                .then(|| mcp_tool_name(server, name))
        })
        .collect()
}

/// What a `--permission-prompt-tool` result says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "lowercase")]
pub enum McpPromptToolAnswer {
    /// The call may go on.
    Allow,
    /// The call is refused.
    Deny {
        /// Why.
        message: String,
    },
}

/// errors.md and mcp.md quote it.
const PROMPT_TOOL_REFUSAL: &str =
    "MCP tool requires user interaction; not supported via --permission-prompt-tool";

/// What a `--permission-prompt-tool` result means for a call: "an `allow` result from the prompt
/// tool for a flagged tool is converted to a deny with the message `MCP tool requires user
/// interaction; not supported via --permission-prompt-tool`". A deny, and any call to an
/// unflagged tool, stands.
#[must_use]
pub fn mcp_prompt_tool_answer(flagged: bool, answer: McpPromptToolAnswer) -> McpPromptToolAnswer {
    if flagged && answer == McpPromptToolAnswer::Allow {
        McpPromptToolAnswer::Deny {
            message: PROMPT_TOOL_REFUSAL.to_owned(),
        }
    } else {
        answer
    }
}

/// The error of a run whose `--permission-prompt-tool` is not among the connected MCP tools when
/// the first permission decision is needed (errors.md, "MCP permission prompt tool not found").
/// The run exits with code 1; `available` names the connected MCP tools, and none reads `none`.
#[must_use]
pub fn permission_prompt_tool_not_found(tool: &str, available: &[String]) -> String {
    let list = if available.is_empty() {
        "none".to_owned()
    } else {
        available.join(", ")
    };
    format!(
        "Error: MCP tool {tool} (passed via --permission-prompt-tool) not found. Available MCP tools: {list}"
    )
}

crate::registry::exports! {
    "format/mcp-tool" {
        "normalizeMcpServerName" => ok |name: String| normalize_mcp_server_name(&name);
        "pluginMcpServerName" => ok |plugin_name: String, server_key: String| plugin_mcp_server_name(&plugin_name, &server_key);
        "mcpToolName" => ok |server: String, tool: String| mcp_tool_name(&server, &tool);
        "MCP_TOOL_NAME_LIMIT" => ok || MCP_TOOL_NAME_LIMIT;
        "mcpToolSendCheck" => ok |full_name: String| mcp_tool_send_check(&full_name);
        "MCP_RESULT_SIZE_CEILING" => ok || MCP_RESULT_SIZE_CEILING;
        "readMcpToolHints" => ok |entry: JsValue| read_mcp_tool_hints(&entry);
        "modelVisibleMcpTool" => ok |entry: JsValue| model_visible_mcp_tool(&entry);
        "requiringInteraction" => ok |server: String, tools: Vec<JsValue>| crate::registry::Set { members: requiring_interaction(&server, &tools).into_iter().collect::<Vec<_>>() };
        "mcpPromptToolAnswer" => ok |flagged: bool, answer: McpPromptToolAnswer| mcp_prompt_tool_answer(flagged, answer);
        "permissionPromptToolNotFound" => ok |tool: String, available: Vec<String>| permission_prompt_tool_not_found(&tool, &available);
    }
}
