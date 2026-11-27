//! Port of `src/format/validate/mcp-check.ts`, switched.
//!
//! The MCP server checks of `claude plugin validate` on `.mcp.json` (oracle 2.1.319; the changelog's
//! "reports `.mcp.json` entries that would be silently dropped at load, undeclared
//! `${user_config.*}` references, and insecure URLs"). The file is a `mcpServers` wrapper or the bare
//! map of servers; each entry is read on its own.

use std::sync::LazyLock;

use indexmap::{IndexMap, IndexSet};

use super::finding::{Finding, error_at};
use super::mcp_remote_checks::remote_findings;
use super::received::kind_of;
use super::zod_findings::dotted;
use crate::format::mcp_oauth_schema::MCP_OAUTH_SCHEMA;
use crate::format::user_config_refs::user_config_references;
use crate::js::{JsObject, JsValue};
use crate::registry::Set;
use crate::zod::{self, Schema, Shape, fields};

const DROPPED: &str = " The plugin loader silently drops this server at load.";
static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
fn common() -> Shape {
    fields! { "timeout" => zod::number().optional(), "alwaysLoad" => zod::boolean().optional() }
}
fn remote() -> Shape {
    fields! {
        "url" => TEXT.clone(),
        "headers" => zod::record(TEXT.clone(), TEXT.clone()).optional(),
        "headersHelper" => TEXT.optional(),
        "oauth" => MCP_OAUTH_SCHEMA.optional(),
        ..common(),
    }
}

static SCHEMAS: LazyLock<IndexMap<&'static str, Schema>> = LazyLock::new(|| {
    IndexMap::from([
        (
            "stdio",
            zod::object(fields! {
                "command" => TEXT.clone(),
                "args" => zod::array(TEXT.clone()).optional(),
                "env" => zod::record(TEXT.clone(), TEXT.clone()).optional(),
                ..common(),
            }),
        ),
        ("http", zod::object(remote())),
        ("sse", zod::object(remote())),
        ("ws", zod::object(remote())),
    ])
});

/// A transport's name, as a key of `SCHEMAS`.
type Transport = &'static str;

fn is_transport(value: &JsValue) -> Option<Transport> {
    let text = value.as_str()?;
    SCHEMAS.get_key_value(text).map(|(name, _)| *name)
}

/// Another name of a transport (mcp.md, "Option 2": `streamable-http` is an alias for `http`).
const ALIASES: [(&str, Transport); 1] = [("streamable-http", "http")];
fn alias_of(value: &JsValue) -> Option<Transport> {
    let text = value.as_str()?;
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == text)
        .map(|(_, transport)| *transport)
}

const TYPES_TAIL: &str = "expected \"stdio\" (or no type) for a local command, or \"http\", \"sse\", \"ws\" for a remote server.";
const NO_TYPE: &str = "server has a \"url\" but no \"type\". Remote servers must set \"type\" to \"http\" (or \"sse\" / \"ws\"); without it the entry is parsed as a stdio server, fails for lacking \"command\", and is silently dropped at load.";
const HOST_ONLY: &str = "type \"sdk\" servers are registered by the host application at runtime and cannot be declared by a plugin. The plugin loader drops this server at load.";
/// How the oracle names the declared type of a server it cannot place.
fn shown_type(r#type: &JsValue) -> String {
    r#type
        .as_str()
        .map_or_else(|| kind_of(r#type).to_owned(), |text| format!("\"{text}\""))
}

/// The transport a server declares, or the finding that says it cannot be placed.
fn transport_of(path: &str, server: &JsObject) -> Result<Transport, Finding> {
    let Some(r#type) = server.get("type").filter(|value| !value.is_undefined()) else {
        return if server.contains_key("url") && !server.contains_key("command") {
            Err(error_at(&format!("{path}.type"), NO_TYPE))
        } else {
            Ok("stdio")
        };
    };
    if r#type.as_str() == Some("sdk") {
        return Err(error_at(&format!("{path}.type"), HOST_ONLY));
    }
    if let Some(transport) = is_transport(r#type) {
        return Ok(transport);
    }
    if let Some(alias) = alias_of(r#type) {
        return Ok(alias);
    }
    Err(error_at(
        &format!("{path}.type"),
        &format!(
            "unknown server type {} — {TYPES_TAIL}{DROPPED}",
            shown_type(r#type)
        ),
    ))
}

/// The strings of a server a `${user_config.*}` reference may sit in, with their paths.
fn referencing(server: &JsObject) -> Vec<(String, String)> {
    let strings = |key: &str| -> Vec<(String, String)> {
        match server.get(key) {
            Some(JsValue::String(value)) => vec![(key.to_owned(), value.clone())],
            Some(JsValue::Array(items)) => items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    item.as_str()
                        .map(|text| (format!("{key}.{index}"), text.to_owned()))
                })
                .collect(),
            Some(JsValue::Object(members)) => members
                .iter()
                .filter_map(|(name, item)| {
                    item.as_str()
                        .map(|text| (format!("{key}.{name}"), text.to_owned()))
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    ["command", "args", "env", "url", "headers"]
        .into_iter()
        .flat_map(strings)
        .collect()
}

fn undeclared_references(
    path: &str,
    server: &JsObject,
    declared: &IndexSet<String>,
) -> Vec<Finding> {
    referencing(server)
        .into_iter()
        .flat_map(|(r#where, value)| {
            user_config_references(&value)
                .into_iter()
                .filter(|key| !declared.contains(key))
                .map(move |key| {
                    error_at(
                        &format!("{path}.{where}"),
                        &format!("references ${{user_config.{key}}}, which plugin.json does not declare under \"userConfig\" (or in this server's \"channels\" entry), so on a fresh install it cannot resolve: the loader drops the server or passes the literal text through. Declare the option or fix the key."),
                    )
                })
        })
        .collect()
}

fn server_findings(path: &str, server: &JsValue, declared: &IndexSet<String>) -> Vec<Finding> {
    let JsValue::Object(members) = server else {
        return vec![error_at(
            path,
            &format!(
                "server config must be an object (got {}).{DROPPED}",
                kind_of(server)
            ),
        )];
    };
    let transport = match transport_of(path, members) {
        Ok(transport) => transport,
        Err(finding) => return vec![finding],
    };
    let schema = match SCHEMAS
        .get(transport)
        .map(|schema| schema.safe_parse(server))
    {
        Some(Err(failure)) => failure
            .issues
            .iter()
            .map(|found| {
                error_at(
                    &dotted(path, found.path()),
                    &format!("{}.{DROPPED}", found.message()),
                )
            })
            .collect(),
        Some(Ok(_)) | None => Vec::new(),
    };
    [
        schema,
        undeclared_references(path, members, declared),
        remote_findings(path, members),
    ]
    .concat()
}

fn server_loads(server: &JsValue) -> bool {
    let JsValue::Object(members) = server else {
        return false;
    };
    transport_of("", members).is_ok_and(|transport| {
        SCHEMAS
            .get(transport)
            .is_some_and(|schema| schema.safe_parse(server).is_ok())
    })
}

/// Whether every server of an inline `mcpServers` map is one the loader keeps.
#[must_use]
pub fn inline_servers_valid(servers: &JsObject) -> bool {
    servers.values().all(server_loads)
}

/// Only what the manifest's own schema leaves to the MCP check: undeclared references, urls and headers.
#[must_use]
pub fn inline_findings(servers: &JsObject, declared: &IndexSet<String>) -> Vec<Finding> {
    servers
        .iter()
        .flat_map(|(name, server)| {
            let JsValue::Object(members) = server else {
                return Vec::new();
            };
            [
                undeclared_references(&format!("mcpServers.{name}"), members, declared),
                remote_findings(&format!("mcpServers.{name}"), members),
            ]
            .concat()
        })
        .collect()
}

/// The findings of a parsed `.mcp.json`.
#[must_use]
pub fn mcp_findings(raw: &JsValue, declared: &IndexSet<String>) -> Vec<Finding> {
    let JsValue::Object(raw) = raw else {
        return Vec::new();
    };
    let wrapped = raw.contains_key("mcpServers");
    let servers = if wrapped {
        raw.get("mcpServers").cloned().unwrap_or_default()
    } else {
        JsValue::Object(raw.clone())
    };
    let JsValue::Object(servers) = servers else {
        return if !wrapped || matches!(servers, JsValue::Array(_)) {
            Vec::new()
        } else {
            vec![error_at(
                "mcpServers",
                &format!(
                    "must be an object mapping server names to server configs (got {}). No servers load from this file.",
                    kind_of(&servers)
                ),
            )]
        };
    };
    servers
        .iter()
        .flat_map(|(name, server)| {
            let path = if wrapped {
                format!("mcpServers.{name}")
            } else {
                name.clone()
            };
            server_findings(&path, server, declared)
        })
        .collect()
}

crate::registry::exports! {
    "format/validate/mcp-check" {
        "inlineFindings" => ok |servers: JsObject, declared: Set<String>|
            inline_findings(&servers, &declared.members.into_iter().collect());
        "mcpFindings" => ok |raw: JsValue, declared: Set<String>|
            mcp_findings(&raw, &declared.members.into_iter().collect());
    }
}
