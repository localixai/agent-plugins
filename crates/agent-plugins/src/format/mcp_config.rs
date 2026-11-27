//! Port of `src/format/mcp-config.ts`, switched.
//!
//! MCP server entries of a plugin (plugins/components.md, "MCP servers"; mcp.md, "Plugin-provided MCP
//! servers"): stdio, SSE, HTTP and WebSocket. The SDK types the first three and not two things the
//! docs give: the `ws` transport and `headersHelper` on the remote ones ("The `type: "ws"` entry
//! accepts the same `url`, `headers`, `headersHelper`, `timeout`, and `alwaysLoad` fields as
//! `http`"). The kit therefore owns `PluginMcpServer`: the official types, plus exactly those fields.
//!
//! A server (`PluginMcpServer`) is held as the object zod gives, a `JsValue`: its types are the
//! SDK's and stay in the TypeScript wrapper.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::mcp_oauth_schema::MCP_OAUTH_SCHEMA;
use crate::js::{JsObject, JsValue, property_order, string_of};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, Shape, ZodError, fields};

/// The transport names of a server's `type`; a server without a `type` is stdio.
pub const MCP_TRANSPORTS: &[&str] = &["stdio", "sse", "http", "ws"];

static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
const WRONG_TEXT: &str = "expected a string";
fn common() -> Shape {
    fields! {
        "timeout" => zod::number().exact_optional(),
        "alwaysLoad" => zod::boolean().exact_optional(),
    }
}
fn remote() -> Shape {
    fields! {
        "url" => TEXT.clone(),
        "headers" => zod::record(TEXT.clone(), TEXT.clone()).exact_optional(),
        "headersHelper" => TEXT.exact_optional(),
        "oauth" => MCP_OAUTH_SCHEMA.exact_optional(),
        ..common(),
    }
}

// Judgment (spec/judgments.md): the schemas check the fields the docs name and keep every other
// field untouched (`oauth`, `tools`, ...), since mcp.md documents more keys than any one table lists.
// An empty command names nothing to run, like an empty hook command.
static STDIO: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "type" => zod::literal("stdio").exact_optional(),
        "command" => TEXT.min_with(1.0, WRONG_TEXT),
        "args" => zod::array(TEXT.clone()).exact_optional(),
        "env" => zod::record(TEXT.clone(), TEXT.clone()).exact_optional(),
        ..common(),
    })
});
static SSE: LazyLock<Schema> =
    LazyLock::new(|| zod::loose_object(fields! { "type" => zod::literal("sse"), ..remote() }));
static HTTP: LazyLock<Schema> =
    LazyLock::new(|| zod::loose_object(fields! { "type" => zod::literal("http"), ..remote() }));
static WS: LazyLock<Schema> =
    LazyLock::new(|| zod::loose_object(fields! { "type" => zod::literal("ws"), ..remote() }));

/// `SCHEMAS[transport]`, `None` for a value that is no transport (`isTransport`).
fn schemas(transport: &str) -> Option<&'static Schema> {
    match transport {
        "stdio" => Some(&STDIO),
        "sse" => Some(&SSE),
        "http" => Some(&HTTP),
        "ws" => Some(&WS),
        _ => None,
    }
}

/// Which transports are reached over a URL (`url`, `headers` and `headersHelper` apply to them).
const REMOTE: &[(&str, bool)] = &[
    ("stdio", false),
    ("sse", true),
    ("http", true),
    ("ws", true),
];

/// A table typed over the transports, read as `TABLE[transport]` (`undefined`, so false, for any other value).
fn lookup(table: &[(&str, bool)], transport: &JsValue) -> bool {
    transport.as_str().is_some_and(|transport| {
        table
            .iter()
            .any(|(name, value)| *name == transport && *value)
    })
}

/// `server.type ?? 'stdio'`.
fn type_or_stdio(server: &JsValue) -> JsValue {
    match server.get("type") {
        None | Some(JsValue::Undefined | JsValue::Null) => JsValue::from("stdio"),
        Some(kind) => kind.clone(),
    }
}

/// The transport of a server; one without a `type` is stdio.
#[must_use]
pub fn mcp_transport_of(server: &JsValue) -> JsValue {
    type_or_stdio(server)
}

/// "OAuth authentication works with HTTP servers", and the flags apply "to HTTP and SSE transports"; WebSocket "supports neither".
const SIGNS_IN_WITH_OAUTH: &[(&str, bool)] = &[
    ("stdio", false),
    ("sse", true),
    ("http", true),
    ("ws", false),
];

/// Whether the server signs in with OAuth.
#[must_use]
pub fn signs_in_with_oauth(server: &JsValue) -> bool {
    lookup(SIGNS_IN_WITH_OAUTH, &mcp_transport_of(server))
}

/// True for a server whose transport is `sse`, `http` or `ws`. The transport decides, not the
/// presence of a `url`: a stdio entry may carry a `url` key (its `type` is then `stdio`) and stays stdio.
#[must_use]
pub fn is_remote_mcp_server(server: &JsValue) -> bool {
    lookup(REMOTE, &type_or_stdio(server))
}

/// Another name of a transport (mcp.md, "Option 2": "the `type` field accepts `streamable-http` as an
/// alias for `http`"). Judgment: the entry is read as the transport it names, so the server the kit
/// returns carries `type: "http"`, the type the SDK knows.
const ALIASES: &[(&str, &str)] = &[("streamable-http", "http")];

/// Types only an SDK host application can register (mcp.md, "Option 2"): the entry is skipped.
const HOST_ONLY: &[&str] = &["sdk"];

fn alias_of(value: Option<&JsValue>) -> Option<&'static str> {
    let value = value.and_then(JsValue::as_str)?;
    ALIASES
        .iter()
        .find(|(name, _)| *name == value)
        .map(|(_, transport)| *transport)
}

fn is_host_only(value: Option<&JsValue>) -> bool {
    value
        .and_then(JsValue::as_str)
        .is_some_and(|value| HOST_ONLY.contains(&value))
}

fn reason(failure: &ZodError) -> String {
    failure
        .issues
        .iter()
        .map(|found| {
            if found.path().is_empty() {
                found.message().to_owned()
            } else {
                let path: Vec<String> = found.path().iter().map(string_of).collect();
                format!("{}: {}", path.join("."), found.message())
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn dropped(name: &str, origin: &str, why: &str) -> PluginIssue {
    error(
        origin,
        &format!("Invalid MCP server config for \"{name}\": {why}"),
    )
}

/// "Claude Code drops that server and records `Invalid MCP server config for <server> in <path>` only in
/// that debug log" (plugins/troubleshooting.md, "Server is configured but never connects"). The kit
/// reports the same entry as an error issue, which `claude plugin validate` reports too and a host
/// may show or log; this is the line Claude Code writes to its debug log (no quotes around the name).
#[must_use]
pub fn dropped_server_log_line(server: &str, path: &str) -> String {
    format!("Invalid MCP server config for {server} in {path}")
}

/// What reading one entry gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpServerRead {
    /// Undefined when Claude Code would drop the entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<JsValue>,
    /// Why it is dropped.
    pub issues: Vec<PluginIssue>,
}

fn refused(issue: PluginIssue) -> McpServerRead {
    McpServerRead {
        server: None,
        issues: vec![issue],
    }
}

/// The two entries mcp.md ("Option 2") says Claude Code skips with a quoted message: a `url` with no
/// `type` (read as stdio, so it fails) and a `type: "sdk"` entry.
fn refusal_of(name: &str, raw: &JsObject) -> Option<String> {
    if is_host_only(raw.get("type")) {
        return Some(format!(
            "Skipped — MCP server \"{name}\" declares type \"sdk\", which only an SDK host application can register"
        ));
    }
    (matches!(raw.get("type"), None | Some(JsValue::Undefined)) && raw.contains_key("url")).then(|| {
        format!(
            "MCP server \"{name}\" has a \"url\" but no \"type\"; add \"type\": \"http\" (or \"sse\" / \"ws\") to this entry"
        )
    })
}

/// Reads one entry. An entry Claude Code would drop is an error naming the server, in the form
/// plugins/troubleshooting.md quotes: `Invalid MCP server config for "<server>": <error>`. The text
/// after the colon is the kit's (the docs give no text for a malformed entry).
#[must_use]
pub fn read_mcp_server(name: &str, raw: &JsValue, origin: &str) -> McpServerRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return refused(dropped(name, origin, "expected an object"));
    };
    if let Some(refusal) = refusal_of(name, members) {
        return refused(error(origin, &refusal));
    }
    let alias = alias_of(members.get("type"));
    let transport = alias.map_or_else(|| type_or_stdio(raw), JsValue::from);
    let Some(schema) = transport.as_str().and_then(schemas) else {
        let names: Vec<String> = MCP_TRANSPORTS
            .iter()
            .chain(ALIASES.iter().map(|(name, _)| name))
            .map(|each| format!("\"{each}\""))
            .collect();
        let why = format!("\"type\" must be one of {}", names.join(", "));
        return refused(dropped(name, origin, &why));
    };
    let parsed = match alias {
        Some(alias) => {
            let mut renamed = members.clone();
            renamed.insert("type".to_owned(), JsValue::from(alias));
            schema.safe_parse(&JsValue::Object(renamed))
        }
        None => schema.safe_parse(raw),
    };
    match parsed {
        Ok(server) => McpServerRead {
            server: Some(server),
            issues: Vec::new(),
        },
        Err(failure) => refused(dropped(name, origin, &reason(&failure))),
    }
}

/// Whether an entry of the manifest's inline `mcpServers` keeps the plugin from loading (oracle
/// 2.1.319): the manifest is validated, so any entry that cannot be read fails it, `type: "sdk"`
/// excepted. The same entry in a `.mcp.json` is only dropped.
#[must_use]
pub fn rejects_manifest(raw: &JsValue) -> bool {
    if is_record(raw) && is_host_only(raw.get("type")) {
        return false;
    }
    read_mcp_server("", raw, "").server.is_none()
}

/// A bundled server's own settings (plugins/components.md, "Include a packaged MCPB server"): what its
/// bundle declares, in the plugin option shape, for a host's settings form and for the resolve. No
/// saved value is kept here: the values are the session's (`pluginConfigs.<id>.mcpServers.<server>`,
/// the host's store under `<server>.<key>`), read when the server is resolved (judgment,
/// spec/judgments.md).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBundleConfig {
    /// The bundle's options, by key (`UserConfigOption`s).
    pub user_config: JsObject,
}

/// What reading a `.mcp.json` value gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigRead {
    /// The servers kept, by name.
    pub servers: JsObject,
    /// The bundled servers among `servers`, by name, with their own settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundles: Option<IndexMap<String, McpBundleConfig>>,
    /// What was dropped, and why.
    pub issues: Vec<PluginIssue>,
    /// The plugin does not load: an inline manifest entry that cannot be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejects_plugin: Option<bool>,
}

/// The servers of a `.mcp.json` value (plugins/components.md: "You can also omit the `mcpServers`
/// wrapper and put `db` at the top level of the file"). Judgment: the wrapper is the key `mcpServers`
/// holding an object that is not itself a server (no `command`, no `url`), so a server a plugin
/// names `mcpServers` still reads as a bare map.
fn wrapped(raw: &JsObject) -> Option<&JsObject> {
    let inner = raw.get("mcpServers")?;
    let members = inner.as_object().filter(|_| is_record(inner))?;
    (!members.contains_key("command") && !members.contains_key("url")).then_some(members)
}

/// Reads a `.mcp.json` value.
#[must_use]
pub fn read_mcp_config(raw: &JsValue, origin: &str) -> McpConfigRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return McpConfigRead {
            servers: JsObject::new(),
            bundles: None,
            issues: vec![error(
                origin,
                "MCP servers must be a JSON object keyed by server name",
            )],
            rejects_plugin: None,
        };
    };
    let entries = wrapped(members).unwrap_or(members);
    let mut servers = JsObject::new();
    let mut issues = Vec::new();
    for (name, entry) in property_order(entries) {
        let read = read_mcp_server(name, entry, origin);
        // `fromEntries` defines own properties, so a server called `__proto__` stays a plain entry.
        if let Some(server) = read.server {
            servers.insert(name.clone(), server);
        }
        issues.extend(read.issues);
    }
    McpConfigRead {
        servers,
        bundles: None,
        issues,
        rejects_plugin: None,
    }
}

crate::registry::exports! {
    "format/mcp-config" {
        "MCP_TRANSPORTS" => ok || MCP_TRANSPORTS;
        "isRemoteMcpServer" => ok |server: JsValue| is_remote_mcp_server(&server);
        "droppedServerLogLine" => ok |server: String, path: String| dropped_server_log_line(&server, &path);
        "readMcpServer" => ok |name: String, raw: JsValue, origin: String| read_mcp_server(&name, &raw, &origin);
        "readMcpConfig" => ok |raw: JsValue, origin: String| read_mcp_config(&raw, &origin);
    }
}
