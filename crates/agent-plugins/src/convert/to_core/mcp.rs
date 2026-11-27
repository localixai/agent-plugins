//! Ported from `src/convert/to-core/mcp.ts`; only the core uses it now, so it has no wrapper.
//!
//! One core `mcp.json` from a Claude Code plugin's servers (Agent Plugins Extended §18.4.3 item 3):
//! `.mcp.json`, the manifest's files and inline maps, already merged by the later-name-wins rule of
//! §2.9 when the plugin loaded. `stdio` (or no `type`) stays `stdio`, `http` is `streamable-http`,
//! `sse` stays; `${CLAUDE_PLUGIN_ROOT}/path` as `command` becomes `./path`, a bare name stays;
//! `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` in `args` and `env` become the core's names, and
//! a stdio server gets `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA` in `env` (J18.18). A `ws` server,
//! a bundle, a field the core lacks (`headersHelper`, `oauth`, `timeout`, `alwaysLoad`), any other
//! `${…}` reference (`${VAR}`, `${user_config.*}`) and an entry the core schema refuses (an `http` URL
//! off loopback, user information, a fragment) are not converted, each reported.

use indexmap::IndexMap;
use serde::Serialize;

use crate::convert::notes::{ConversionNote, not_converted};
use crate::convert::placeholders::{
    bridged_environment, holds_other_reference, holds_reference, to_core_placeholders,
};
use crate::format::core::mcp_server::{CoreServerRead, read_core_server};
use crate::format::mcp_config::mcp_transport_of;
use crate::js::{JsObject, JsValue, is_whitespace, json_stringify, string_of};

/// The core file's name.
pub const CORE_MCP: &str = "mcp.json";
const SOURCE: &str = ".mcp.json";

type Raw = JsObject;

/// A server mapped, or why it is not.
enum Mapped {
    Raw(Raw),
    Problem(String),
}

/// The fields each Claude Code transport keeps in the core.
const FIELDS: &[(&str, &[&str])] = &[
    ("stdio", &["type", "command", "args", "env"]),
    ("http", &["type", "url", "headers"]),
    ("sse", &["type", "url", "headers"]),
    ("ws", &[]),
];

const ROOT_PREFIX: &str = "${CLAUDE_PLUGIN_ROOT}/";

fn core_command(command: &str) -> Option<String> {
    if let Some(rest) = command.strip_prefix(ROOT_PREFIX) {
        return Some(format!("./{rest}"));
    }
    // `/^[^\s/\\$]+$/`, `\s` JavaScript's whitespace.
    let bare = !command.is_empty()
        && command
            .chars()
            .all(|c| !is_whitespace(c) && !matches!(c, '/' | '\\' | '$'));
    bare.then(|| command.to_owned())
}

fn strings(value: Option<&JsValue>) -> Vec<String> {
    match value {
        Some(JsValue::Array(items)) => items
            .iter()
            .filter_map(|one| one.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// `Object.entries` of an object (an array's entries are its indexes), the string members kept.
fn string_map(value: Option<&JsValue>) -> IndexMap<String, String> {
    match value {
        Some(JsValue::Object(members)) => members
            .iter()
            .filter_map(|(key, one)| one.as_str().map(|text| (key.clone(), text.to_owned())))
            .collect(),
        Some(JsValue::Array(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(at, one)| one.as_str().map(|text| (at.to_string(), text.to_owned())))
            .collect(),
        _ => IndexMap::new(),
    }
}

fn string_values(map: &IndexMap<String, String>) -> JsValue {
    JsValue::Object(
        map.iter()
            .map(|(key, value)| (key.clone(), JsValue::from(value.as_str())))
            .collect(),
    )
}

fn stdio(server: &Raw) -> Mapped {
    let written = server.get("command");
    let Some(command) = core_command(written.and_then(JsValue::as_str).unwrap_or("")) else {
        let shown = json_stringify(written.unwrap_or(&JsValue::Undefined))
            .unwrap_or_else(|| "undefined".to_owned());
        return Mapped::Problem(format!(
            "\"command\" {shown} is neither ${{CLAUDE_PLUGIN_ROOT}}/<path> nor a bare name"
        ));
    };
    let args = strings(server.get("args"));
    let env = string_map(server.get("env"));
    if args
        .iter()
        .chain(env.values())
        .any(|text| holds_other_reference(text, "claude-code"))
    {
        return Mapped::Problem(
            "its args or env hold a \"${…}\" reference the core does not expand".to_owned(),
        );
    }
    let mut raw = Raw::new();
    raw.insert("type".to_owned(), "stdio".into());
    raw.insert("command".to_owned(), command.into());
    if !args.is_empty() {
        let mapped = args.iter().map(|one| to_core_placeholders(one).into());
        raw.insert("args".to_owned(), JsValue::Array(mapped.collect()));
    }
    let bridged = bridged_environment(Some(&env), "claude-code", to_core_placeholders);
    raw.insert("env".to_owned(), string_values(&bridged));
    Mapped::Raw(raw)
}

fn remote(kind: &str, server: &Raw) -> Mapped {
    let headers = match server.get("headers") {
        None | Some(JsValue::Undefined) => None,
        written => Some(string_map(written)),
    };
    let url = server.get("url").unwrap_or(&JsValue::Undefined);
    let texts: Vec<String> = std::iter::once(string_of(url))
        .chain(headers.iter().flat_map(|map| map.values().cloned()))
        .collect();
    if texts.iter().any(|text| holds_reference(text)) {
        return Mapped::Problem(
            "its url or headers hold a \"${…}\" reference, which the core keeps literally"
                .to_owned(),
        );
    }
    let mut raw = Raw::new();
    raw.insert("type".to_owned(), kind.into());
    raw.insert("url".to_owned(), url.clone());
    if let Some(headers) = &headers {
        raw.insert("headers".to_owned(), string_values(headers));
    }
    Mapped::Raw(raw)
}

/// One mapping per Claude Code transport (K1).
fn mappings(transport: &str, server: &Raw) -> Mapped {
    match transport {
        "stdio" => stdio(server),
        "http" => remote("streamable-http", server),
        "sse" => remote("sse", server),
        _ => Mapped::Problem("the ws transport has no core counterpart".to_owned()),
    }
}

fn mapped(server: &JsValue, bundled: bool) -> Mapped {
    if bundled {
        return Mapped::Problem("an MCP bundle has no core counterpart".to_owned());
    }
    let transport = string_of(&mcp_transport_of(server));
    let own = server.as_object().cloned().unwrap_or_default();
    let result = mappings(&transport, &own);
    let Mapped::Raw(raw) = result else {
        return result;
    };
    let fields = FIELDS
        .iter()
        .find(|(name, _)| *name == transport)
        .map_or(&[][..], |(_, fields)| fields);
    let extra: Vec<String> = own
        .keys()
        .filter(|key| !fields.contains(&key.as_str()))
        .map(|key| format!("\"{key}\""))
        .collect();
    if !extra.is_empty() {
        return Mapped::Problem(format!("{} has no core counterpart", extra.join(", ")));
    }
    match read_core_server(&JsValue::Object(raw.clone()), &[]) {
        CoreServerRead::Problem(problem) => {
            Mapped::Problem(format!("the core refuses it: {problem}"))
        }
        CoreServerRead::Server(_) => Mapped::Raw(raw),
    }
}

/// The core `mcp.json`'s servers, and what was left out.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoreMcpPart {
    /// The servers, by name.
    pub servers: IndexMap<String, Raw>,
    /// What was not converted.
    pub notes: Vec<ConversionNote>,
}

/// The servers as the core writes them.
#[must_use]
pub fn core_mcp_of(servers: &JsObject, bundles: &[String]) -> CoreMcpPart {
    let mut part = CoreMcpPart {
        servers: IndexMap::new(),
        notes: Vec::new(),
    };
    let results: Vec<(&String, Mapped)> = servers
        .iter()
        .map(|(name, server)| (name, mapped(server, bundles.contains(name))))
        .collect();
    for (name, result) in &results {
        if let Mapped::Raw(raw) = result {
            part.servers.insert((*name).clone(), raw.clone());
        }
    }
    for (name, result) in results {
        if let Mapped::Problem(problem) = result {
            part.notes.push(not_converted(
                SOURCE.to_owned(),
                Some(format!("mcpServers.{name}")),
                format!("server \"{name}\" not converted: {problem}"),
            ));
        }
    }
    part
}
