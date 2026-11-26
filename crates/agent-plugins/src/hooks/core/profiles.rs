//! Port of `src/hooks/core/profiles.ts`, switched.
//!
//! The portable view of a pending Claude Code call (Agent Plugins Extended §18.2.7; core §7.3.6): a
//! table by Claude Code tool name of 2.1.321, each entry deciding from the call's defined operation
//! and input whether its profile applies. A tool the table does not name gets none (`Glob`, `Grep`,
//! `Agent`, `Skill`, `AskUserQuestion` and every bookkeeping tool), and so does a call the entry's
//! criteria exclude (a `Monitor` with `ws`, a `WebFetch` of a relative URL).

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::url;
use crate::js::value::JsObject;
use crate::protocol::is_record::is_record;

/// What the host knows of a call that Claude Code's input does not carry (`CoreCallFacts`); only
/// the field a profile reads is named.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCallFacts {
    /// The tool name an MCP server declared, as sent in `tools/call` (`latest_tag`).
    #[serde(default)]
    pub mcp_tool_name: Option<String>,
}

/// A portable tool profile: its name and its input projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoreToolProfile {
    /// The profile's name (`shell`, `file.modify`, …).
    pub name: String,
    /// The input projection.
    pub input: JsObject,
}

/// What a projector reads besides the input.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProfileContext {
    /// The session's working directory: a relative `notebook_path` is made absolute against it.
    pub cwd: String,
    /// What the host knows of the call.
    pub facts: CoreCallFacts,
}

type Input = JsObject;

fn text(input: &Input, key: &str) -> Option<String> {
    input.get(key).and_then(JsValue::as_str).map(str::to_owned)
}

fn profile(name: &str, input: JsObject) -> CoreToolProfile {
    CoreToolProfile {
        name: name.to_owned(),
        input,
    }
}

fn one(key: &str, value: JsValue) -> JsObject {
    JsObject::from_iter([(key.to_owned(), value)])
}

/// `shell`: the command string exactly, which the tool has the shell interpret.
fn shell(input: &Input, _context: &ProfileContext) -> Option<CoreToolProfile> {
    let command = text(input, "command")?;
    Some(profile("shell", one("command", command.into())))
}

fn file_read(input: &Input, _context: &ProfileContext) -> Option<CoreToolProfile> {
    let path = text(input, "file_path")?;
    Some(profile("file.read", one("path", path.into())))
}

/// `file.modify` with one operation of `kind` on the path the key names.
fn modify(
    kind: &str,
    key: &str,
    absolute: bool,
    input: &Input,
    context: &ProfileContext,
) -> Option<CoreToolProfile> {
    let written = text(input, key)?;
    let path = if absolute && !written.starts_with('/') {
        format!("{}/{written}", context.cwd)
    } else {
        written
    };
    let operation = JsObject::from_iter([
        ("kind".to_owned(), kind.into()),
        ("path".to_owned(), path.into()),
    ]);
    Some(profile(
        "file.modify",
        one("operations", vec![JsValue::Object(operation)].into()),
    ))
}

const WEB_SCHEMES: [&str; 2] = ["http:", "https:"];

/// `web.fetch` only for an absolute `http` or `https` URL (core §7.3.6, "Web fetch tool profile").
fn web_fetch(input: &Input, _context: &ProfileContext) -> Option<CoreToolProfile> {
    let address = text(input, "url")?;
    let parsed = url::parse(&address)?;
    WEB_SCHEMES
        .contains(&parsed.protocol.as_str())
        .then(|| profile("web.fetch", one("url", address.into())))
}

fn web_search(input: &Input, _context: &ProfileContext) -> Option<CoreToolProfile> {
    let query = text(input, "query")?;
    Some(profile(
        "web.search",
        one("queries", vec![JsValue::from(query)].into()),
    ))
}

const MCP_PREFIX: &str = "mcp__";

/// The tool name an MCP call sends: the one the host gives (`CoreCallFacts.mcpToolName`), else the
/// text after the server segment of `mcp__<server>__<tool>` when that is the only reading: the text
/// after `mcp__` holds `__` once (J18.21). A server segment or a declared name may itself hold `__`
/// (§8.5), so any other full name gives no profile unless the host names the tool.
fn mcp_tool_name(tool_name: &str, facts: &CoreCallFacts) -> Option<String> {
    if let Some(name) = &facts.mcp_tool_name {
        return Some(name.clone());
    }
    let rest = tool_name.strip_prefix(MCP_PREFIX).unwrap_or_default();
    let at = rest.find("__")?;
    if Some(at) != rest.rfind("__") {
        return None;
    }
    if at == 0 {
        return None;
    }
    rest.get(at + 2..)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// The profile of a pending call, or `undefined` when none applies.
#[must_use]
pub fn pending_profile(
    tool_name: &str,
    tool_input: &JsValue,
    context: &ProfileContext,
) -> Option<CoreToolProfile> {
    let record = tool_input.as_object().filter(|_| is_record(tool_input));
    if tool_name.starts_with(MCP_PREFIX) {
        let name = mcp_tool_name(tool_name, &context.facts)?;
        let input = record.cloned().unwrap_or_default();
        let members = JsObject::from_iter([
            ("name".to_owned(), JsValue::from(name)),
            ("arguments".to_owned(), JsValue::Object(input)),
        ]);
        return Some(profile("mcp", members));
    }
    let input = record?;
    match tool_name {
        "Bash" | "PowerShell" | "Monitor" => shell(input, context),
        "Read" => file_read(input, context),
        "Edit" => modify("edit", "file_path", false, input, context),
        "Write" => modify("write", "file_path", false, input, context),
        "NotebookEdit" => modify("edit", "notebook_path", true, input, context),
        "WebFetch" => web_fetch(input, context),
        "WebSearch" => web_search(input, context),
        _ => None,
    }
}

crate::registry::exports! {
    "hooks/core/profiles" {
        "pendingProfile" => ok |tool_name: String, tool_input: JsValue, context: ProfileContext| pending_profile(&tool_name, &tool_input, &context);
    }
}
