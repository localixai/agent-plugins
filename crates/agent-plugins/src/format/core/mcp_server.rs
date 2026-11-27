//! Port of `src/format/core/mcp-server.ts`, switched.
//!
//! One server of a core `mcp.json` (core §7.2.1): an object with a `type` that matches exactly one of
//! three closed variants. An unknown field, an unknown `type` or a field of another variant makes the
//! entry invalid; each variant takes an optional `extensions` (core §8.1). The rules are the schema's
//! (`schemas/1.1.0/mcp.schema.json`, `#/$defs/server`) with the text's: the command token, the `cwd`
//! forms and containment, the reserved `env` names (core §9.2), the URL and header rules.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::command::command_problem;
use super::extensions::{CoreExtensions, extensions_problem, implemented_extensions};
use super::placeholders::{CORE_VARIABLES, CwdResolution, resolve_cwd, symbolic_paths};
use super::remote::{headers_problem, url_problem};
use crate::js::{JsObject, JsValue, json_stringify, property_order};
use crate::protocol::is_record::is_record;

/// The transports of a core server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreTransport {
    /// `stdio`
    #[serde(rename = "stdio")]
    Stdio,
    /// `streamable-http`
    #[serde(rename = "streamable-http")]
    StreamableHttp,
    /// `sse`
    #[serde(rename = "sse")]
    Sse,
}

impl CoreTransport {
    /// The transport as its string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::StreamableHttp => "streamable-http",
            Self::Sse => "sse",
        }
    }
}

/// The three transports, in the package's order.
pub const CORE_TRANSPORTS: [CoreTransport; 3] = [
    CoreTransport::Stdio,
    CoreTransport::StreamableHttp,
    CoreTransport::Sse,
];

/// The one `type` of a stdio server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StdioType {
    /// `stdio`
    #[serde(rename = "stdio")]
    Stdio,
}

/// The `type` of a remote server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteType {
    /// `streamable-http`
    #[serde(rename = "streamable-http")]
    StreamableHttp,
    /// `sse`
    #[serde(rename = "sse")]
    Sse,
}

/// A server a host launches as a subprocess.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreStdioServer {
    /// Always `stdio`.
    #[serde(rename = "type")]
    pub transport: StdioType,
    /// The command token.
    pub command: String,
    /// Its arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Its environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<IndexMap<String, String>>,
    /// Its working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Only the namespaces the host implements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CoreExtensions>,
}

/// A server a host connects to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreRemoteServer {
    /// `streamable-http` or `sse`.
    #[serde(rename = "type")]
    pub transport: RemoteType,
    /// Its address.
    pub url: String,
    /// Its headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    /// Only the namespaces the host implements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CoreExtensions>,
}

/// Either server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreMcpServer {
    /// A stdio server.
    Stdio(CoreStdioServer),
    /// A remote server.
    Remote(CoreRemoteServer),
}

/// The check of one field's value: why it is wrong, or `None`.
type Check = fn(&JsValue) -> Option<String>;

fn strings(value: &JsValue) -> Option<String> {
    if value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string))
    {
        None
    } else {
        Some("must be an array of strings".to_owned())
    }
}

fn command_check(value: &JsValue) -> Option<String> {
    command_problem(value).map(str::to_owned)
}

fn env_problem(value: &JsValue) -> Option<String> {
    let JsValue::Object(members) = value else {
        return Some("must be an object of strings".to_owned());
    };
    if let Some(reserved) = CORE_VARIABLES
        .iter()
        .find(|name| members.contains_key(name.name()))
    {
        return Some(format!(
            "must not set {}: the client supplies it",
            reserved.name()
        ));
    }
    property_order(members)
        .into_iter()
        .find(|(_, member)| !member.is_string())
        .map(|(key, _)| format!("\"{key}\" must be a string"))
}

fn cwd_problem(value: &JsValue) -> Option<String> {
    let JsValue::String(text) = value else {
        return Some("must be a string".to_owned());
    };
    match resolve_cwd(text, &symbolic_paths()) {
        CwdResolution::Problem(problem) => Some(problem),
        CwdResolution::Cwd(_) => None,
    }
}

/// `typeCheck(transport)`: the value must be the transport's name.
fn type_check(transport: CoreTransport, value: &JsValue) -> Option<String> {
    if value.as_str() == Some(transport.as_str()) {
        None
    } else {
        Some(format!("must be \"{}\"", transport.as_str()))
    }
}

fn stdio_type(value: &JsValue) -> Option<String> {
    type_check(CoreTransport::Stdio, value)
}

fn streamable_http_type(value: &JsValue) -> Option<String> {
    type_check(CoreTransport::StreamableHttp, value)
}

fn sse_type(value: &JsValue) -> Option<String> {
    type_check(CoreTransport::Sse, value)
}

/// The fields of a remote variant: `remoteFields(transport)`.
const fn remote_fields(type_field: Check) -> [(&'static str, Check); 4] {
    [
        ("type", type_field),
        ("url", url_problem),
        ("headers", headers_problem),
        ("extensions", extensions_problem),
    ]
}

/// The closed fields of a stdio server and their checks.
const STDIO_FIELDS: [(&str, Check); 6] = [
    ("type", stdio_type),
    ("command", command_check),
    ("args", strings),
    ("env", env_problem),
    ("cwd", cwd_problem),
    ("extensions", extensions_problem),
];

/// The closed fields of each variant and their checks: a registry over the transports.
fn variant(transport: CoreTransport) -> Vec<(&'static str, Check)> {
    match transport {
        CoreTransport::Stdio => STDIO_FIELDS.to_vec(),
        CoreTransport::StreamableHttp => remote_fields(streamable_http_type).to_vec(),
        CoreTransport::Sse => remote_fields(sse_type).to_vec(),
    }
}

/// The fields every server of a transport must have.
const fn required(transport: CoreTransport) -> [&'static str; 2] {
    match transport {
        CoreTransport::Stdio => ["type", "command"],
        CoreTransport::StreamableHttp | CoreTransport::Sse => ["type", "url"],
    }
}

fn is_transport(value: &JsValue) -> Option<CoreTransport> {
    CORE_TRANSPORTS
        .iter()
        .copied()
        .find(|known| value.as_str() == Some(known.as_str()))
}

fn other_variant_has(field: &str) -> bool {
    CORE_TRANSPORTS
        .iter()
        .any(|transport| variant(*transport).iter().any(|(name, _)| *name == field))
}

fn field_problem(transport: CoreTransport, field: &str, value: &JsValue) -> Option<String> {
    let check = variant(transport)
        .into_iter()
        .find(|(name, _)| *name == field)
        .map(|(_, check)| check);
    if let Some(check) = check {
        return check(value).map(|problem| format!("\"{field}\" {problem}"));
    }
    Some(if other_variant_has(field) {
        format!(
            "\"{field}\" belongs to another transport than \"{}\"",
            transport.as_str()
        )
    } else {
        format!("unknown field \"{field}\"")
    })
}

/// The entry as a server, or why it is invalid (core §7.2.2 item 3: the server is skipped).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoreServerRead {
    /// The server.
    Server(CoreMcpServer),
    /// Why the entry is invalid.
    Problem(String),
}

/// The entry as a server, or why it is invalid (core §7.2.2 item 3: the server is skipped).
#[must_use]
pub fn read_core_server(raw: &JsValue, implemented: &[String]) -> CoreServerRead {
    let problem = |text: String| CoreServerRead::Problem(text);
    let JsValue::Object(members) = raw else {
        return problem("a server configuration must be an object".to_owned());
    };
    let declared = members.get("type").unwrap_or(&JsValue::Undefined);
    if declared.is_undefined() {
        return problem("a server configuration must have a \"type\"".to_owned());
    }
    let Some(transport) = is_transport(declared) else {
        return problem(format!(
            "unknown \"type\" {}",
            json_stringify(declared).unwrap_or_default()
        ));
    };
    let missing = required(transport)
        .into_iter()
        .find(|field| members.get(*field).is_none_or(JsValue::is_undefined));
    if let Some(missing) = missing {
        return problem(format!(
            "\"{missing}\" is required for \"{}\"",
            transport.as_str()
        ));
    }
    for (field, value) in property_order(members) {
        if let Some(found) = field_problem(transport, field, value) {
            return problem(found);
        }
    }
    let extensions = implemented_extensions(
        members.get("extensions").unwrap_or(&JsValue::Undefined),
        implemented,
    );
    CoreServerRead::Server(build(transport, members, extensions))
}

fn text(value: Option<&JsValue>) -> String {
    value
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn string_list(value: Option<&JsValue>) -> Option<Vec<String>> {
    value.and_then(JsValue::as_array).map(|items| {
        items
            .iter()
            .filter_map(|one| one.as_str().map(str::to_owned))
            .collect()
    })
}

fn string_map(value: Option<&JsValue>) -> Option<IndexMap<String, String>> {
    let record = value.filter(|one| is_record(one))?;
    let members = record.as_object()?;
    Some(
        property_order(members)
            .into_iter()
            .filter_map(|(key, member)| member.as_str().map(|one| (key.clone(), one.to_owned())))
            .collect(),
    )
}

/// The typed server of each variant, built once its fields passed their checks.
fn build(
    transport: CoreTransport,
    raw: &JsObject,
    extensions: Option<CoreExtensions>,
) -> CoreMcpServer {
    match transport {
        CoreTransport::Stdio => CoreMcpServer::Stdio(CoreStdioServer {
            transport: StdioType::Stdio,
            command: text(raw.get("command")),
            args: string_list(raw.get("args")),
            env: string_map(raw.get("env")),
            cwd: raw.get("cwd").and_then(JsValue::as_str).map(str::to_owned),
            extensions,
        }),
        CoreTransport::StreamableHttp => remote(RemoteType::StreamableHttp, raw, extensions),
        CoreTransport::Sse => remote(RemoteType::Sse, raw, extensions),
    }
}

fn remote(
    transport: RemoteType,
    raw: &JsObject,
    extensions: Option<CoreExtensions>,
) -> CoreMcpServer {
    CoreMcpServer::Remote(CoreRemoteServer {
        transport,
        url: text(raw.get("url")),
        headers: string_map(raw.get("headers")),
        extensions,
    })
}

crate::registry::exports! {
    "format/core/mcp-server" {
        "CORE_TRANSPORTS" => ok || CORE_TRANSPORTS;
        "readCoreServer" => ok |raw: JsValue, implemented: Vec<String>| read_core_server(&raw, &implemented);
    }
}
