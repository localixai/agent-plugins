//! Port of `src/convert/to-claude-code/mcp.ts`, switched.
//!
//! `.mcp.json` from a core `mcp.json` (Agent Plugins Extended §18.4.2 item 3, the table of §18.3.3):
//! `stdio` stays, `streamable-http` is `http`, `sse` stays; a `./` command is
//! `${CLAUDE_PLUGIN_ROOT}/…`; `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` become Claude Code's names, and a
//! stdio server gets `PLUGIN_ROOT` and `PLUGIN_DATA` in `env` (J18.18). `cwd` and `extensions` are
//! left out and reported; a server with a `${` left in `args`, `env`, `url` or `headers`, which Claude
//! Code would expand as a variable (§8.4.2) with no escape, is not written. Entries the core skipped
//! are not in the package, so they are not written either.

use indexmap::IndexMap;
use serde::Serialize;

use crate::convert::notes::{ConversionNote, not_converted};
use crate::convert::placeholders::{
    bridged_environment, holds_other_reference, to_claude_code_placeholders,
};
use crate::format::core::extensions::CoreExtensions;
use crate::format::core::mcp_server::{
    CoreMcpServer, CoreRemoteServer, CoreStdioServer, RemoteType,
};
use crate::js::{JsObject, JsValue};

/// Where the converted servers are written.
pub const CLAUDE_MCP: &str = ".mcp.json";
const SOURCE: &str = "mcp.json";

/// One server as Claude Code writes it, and what to say about it.
struct ServerPart {
    server: Option<JsObject>,
    notes: Vec<ConversionNote>,
}

fn at(name: &str, field: Option<&str>) -> String {
    match field {
        None => format!("mcpServers.{name}"),
        Some(field) => format!("mcpServers.{name}.{field}"),
    }
}

fn left_references(name: &str, values: &[(&str, Vec<&str>)]) -> Option<ConversionNote> {
    let fields: Vec<&str> = values
        .iter()
        .filter(|(_, texts)| texts.iter().any(|text| holds_other_reference(text, "core")))
        .map(|(field, _)| *field)
        .collect();
    if fields.is_empty() {
        return None;
    }
    Some(not_converted(
        SOURCE.to_owned(),
        Some(at(name, None)),
        format!(
            "server \"{name}\" not converted: {} hold \"${{\" text, which Claude Code would expand as a variable",
            fields.join(", ")
        ),
    ))
}

fn strings(values: &[String]) -> JsValue {
    JsValue::Array(
        values
            .iter()
            .map(|value| JsValue::from(value.as_str()))
            .collect(),
    )
}

fn stdio_server(name: &str, server: &CoreStdioServer) -> ServerPart {
    let server_args = server.args.clone().unwrap_or_default();
    let env_values: Vec<&str> = server
        .env
        .iter()
        .flat_map(|env| env.values().map(String::as_str))
        .collect();
    if let Some(refused) = left_references(
        name,
        &[
            ("args", server_args.iter().map(String::as_str).collect()),
            ("env", env_values),
        ],
    ) {
        return ServerPart {
            server: None,
            notes: vec![refused],
        };
    }
    let args: Vec<String> = server_args
        .iter()
        .map(|arg| to_claude_code_placeholders(arg))
        .collect();
    let env = bridged_environment(server.env.as_ref(), "core", to_claude_code_placeholders);
    let notes = server
        .cwd
        .as_ref()
        .map(|cwd| {
            not_converted(
                SOURCE.to_owned(),
                Some(at(name, Some("cwd"))),
                format!(
                    "\"cwd\" has no Claude Code counterpart: server \"{name}\" starts in Claude Code's working directory, not \"{cwd}\""
                ),
            )
        })
        .into_iter()
        .collect();
    let command = server.command.strip_prefix("./").map_or_else(
        || server.command.clone(),
        |rest| format!("${{CLAUDE_PLUGIN_ROOT}}/{rest}"),
    );
    let mut written = JsObject::new();
    written.insert("type".to_owned(), "stdio".into());
    written.insert("command".to_owned(), command.into());
    if !args.is_empty() {
        written.insert("args".to_owned(), strings(&args));
    }
    written.insert(
        "env".to_owned(),
        JsValue::Object(
            env.into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
        ),
    );
    ServerPart {
        server: Some(written),
        notes,
    }
}

/// The Claude Code transport of each remote core transport (§18.3.3).
const fn remote_type(transport: RemoteType) -> &'static str {
    match transport {
        RemoteType::StreamableHttp => "http",
        RemoteType::Sse => "sse",
    }
}

fn remote_server(name: &str, server: &CoreRemoteServer) -> ServerPart {
    let headers = server.headers.clone().unwrap_or_default();
    if let Some(refused) = left_references(
        name,
        &[
            ("url", vec![server.url.as_str()]),
            ("headers", headers.values().map(String::as_str).collect()),
        ],
    ) {
        return ServerPart {
            server: None,
            notes: vec![refused],
        };
    }
    let mut written = JsObject::new();
    written.insert("type".to_owned(), remote_type(server.transport).into());
    written.insert("url".to_owned(), server.url.as_str().into());
    if server.headers.is_some() {
        written.insert(
            "headers".to_owned(),
            JsValue::Object(
                headers
                    .into_iter()
                    .map(|(key, value)| (key, value.into()))
                    .collect(),
            ),
        );
    }
    ServerPart {
        server: Some(written),
        notes: Vec::new(),
    }
}

fn extension_notes(name: &str, extensions: Option<&CoreExtensions>) -> Vec<ConversionNote> {
    extensions
        .into_iter()
        .flat_map(|extensions| {
            extensions.keys().map(|namespace| {
                not_converted(
                    SOURCE.to_owned(),
                    Some(at(name, Some(&format!("extensions.{namespace}")))),
                    "client extension data has no Claude Code counterpart".to_owned(),
                )
            })
        })
        .collect()
}

/// The servers written and what to say about them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpPart {
    /// Each server written, by name.
    pub servers: IndexMap<String, JsObject>,
    /// What was not carried over.
    pub notes: Vec<ConversionNote>,
}

/// `.mcp.json` from a core `mcp.json`.
#[must_use]
pub fn claude_code_mcp(servers: &IndexMap<String, CoreMcpServer>) -> McpPart {
    let mut written = IndexMap::new();
    let mut notes = Vec::new();
    for (name, server) in servers {
        let (part, extensions) = match server {
            CoreMcpServer::Stdio(stdio) => (stdio_server(name, stdio), stdio.extensions.as_ref()),
            CoreMcpServer::Remote(remote) => {
                (remote_server(name, remote), remote.extensions.as_ref())
            }
        };
        notes.extend(part.notes);
        notes.extend(extension_notes(name, extensions));
        if let Some(server) = part.server {
            written.insert(name.clone(), server);
        }
    }
    McpPart {
        servers: written,
        notes,
    }
}

crate::registry::exports! {
    "convert/to-claude-code/mcp" {
        "CLAUDE_MCP" => ok || CLAUDE_MCP;
        "claudeCodeMcp" => ok |servers: IndexMap<String, CoreMcpServer>| claude_code_mcp(&servers);
    }
}
