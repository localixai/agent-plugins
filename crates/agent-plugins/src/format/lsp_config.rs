//! Port of `src/format/lsp-config.ts`, switched.
//!
//! LSP server entries of a plugin (plugins/manifest-reference.md, "lspServers"; plugins/components.md,
//! "LSP servers"). The SDK has no LSP config type (no match in sdk.d.ts), so `LspServer` is the kit's,
//! transcribed from the field table: "Each server config is a strict object with these fields. An
//! unknown key fails validation."

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::lsp_schema::LSP_SERVER_SCHEMA;
use crate::js::{JsValue, array_join, in_key_order, key_order, property_order};
use crate::protocol::is_record::is_record;
use crate::zod::ZodError;

/// `stdio` (default) or `socket`: "Claude Code accepts `socket` but runs every server over stdio".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspTransport {
    /// `stdio`.
    Stdio,
    /// `socket`.
    Socket,
}

/// One LSP server's configuration, its members in the table's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspServer {
    /// Language server binary: no whitespace unless the value starts with `/`.
    pub command: String,
    /// File extension (with its dot) to LSP language ID; at least one entry.
    pub extension_to_language: IndexMap<String, String>,
    /// Its arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// How it is reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<LspTransport>,
    /// Its environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<IndexMap<String, String>>,
    /// Sent in the initialize request. Any JSON value.
    #[serde(default, skip_serializing_if = "JsValue::is_undefined")]
    pub initialization_options: JsValue,
    /// Sent by `workspace/didChangeConfiguration`. Any JSON value.
    #[serde(default, skip_serializing_if = "JsValue::is_undefined")]
    pub settings: JsValue,
    /// Its workspace folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    /// Milliseconds, a positive integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_timeout: Option<f64>,
    /// Milliseconds, a positive integer; unset means no timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shutdown_timeout: Option<f64>,
    /// Milliseconds, a positive integer; Claude Code defaults to 60000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_timeout: Option<f64>,
    /// Claude Code defaults to `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_on_crash: Option<bool>,
    /// Restart attempts before giving up, zero or more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_restarts: Option<f64>,
    /// Claude Code defaults to `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<bool>,
}

/// The fourteen rows of the table.
pub const LSP_FIELDS: [&str; 14] = [
    "command",
    "extensionToLanguage",
    "args",
    "transport",
    "env",
    "initializationOptions",
    "settings",
    "workspaceFolder",
    "startupTimeout",
    "shutdownTimeout",
    "requestTimeout",
    "restartOnCrash",
    "maxRestarts",
    "diagnostics",
];

fn reason(failure: &ZodError) -> String {
    failure
        .issues
        .iter()
        .map(|found| {
            if found.path().is_empty() {
                found.message().to_owned()
            } else {
                format!("{}: {}", array_join(found.path(), "."), found.message())
            }
        })
        .collect::<Vec<String>>()
        .join("; ")
}

/// `Invalid LSP server config for ".lsp.json"` is the prefix components.md quotes.
fn invalid(origin: &str, why: &str) -> PluginIssue {
    error(
        origin,
        &format!("Invalid LSP server config for \"{origin}\": {why}"),
    )
}

/// What reading one entry gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LspServerRead {
    /// Undefined when Claude Code would not load the entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<LspServer>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
}

/// One entry of a map; `origin` is the file (or `lspServers` for an inline map) it came from.
#[must_use]
pub fn read_lsp_server(name: &str, raw: &JsValue, origin: &str) -> LspServerRead {
    if !is_record(raw) {
        return LspServerRead {
            server: None,
            issues: vec![invalid(origin, &format!("{name}: expected an object"))],
        };
    }
    match LSP_SERVER_SCHEMA.safe_parse(raw) {
        // The schema's output is the server: it reads as one.
        Ok(parsed) => LspServerRead {
            server: LspServer::deserialize(parsed).ok(),
            issues: Vec::new(),
        },
        Err(failure) => LspServerRead {
            server: None,
            issues: vec![invalid(origin, &format!("{name}: {}", reason(&failure)))],
        },
    }
}

/// The servers of one map, or of several joined.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LspConfigRead {
    /// Each server by name.
    pub servers: IndexMap<String, LspServer>,
    /// What is wrong.
    pub issues: Vec<PluginIssue>,
}

/// The servers of one map: a `.lsp.json` (or another `.json` file `lspServers` names) or an inline
/// map. Components.md: "The file maps each server name directly to its configuration, with no
/// wrapper object", and "When any entry is invalid, the whole file is skipped at load": one invalid
/// entry leaves no server from this map. An inline map never gets here invalid: it is part of the
/// manifest's schema (manifest-schema.ts), so a bad inline entry keeps the plugin from loading
/// (spec §9.1.1 item 3, J9.5).
#[must_use]
pub fn read_lsp_config(raw: &JsValue, origin: &str) -> LspConfigRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return LspConfigRead {
            servers: IndexMap::new(),
            issues: vec![invalid(origin, "expected an object keyed by server name")],
        };
    };
    let reads: Vec<(&String, LspServerRead)> = property_order(members)
        .into_iter()
        .map(|(name, entry)| (name, read_lsp_server(name, entry, origin)))
        .collect();
    let issues: Vec<PluginIssue> = reads
        .iter()
        .flat_map(|(_, read)| read.issues.iter().cloned())
        .collect();
    LspConfigRead {
        servers: if issues.is_empty() {
            entries_of(reads)
        } else {
            IndexMap::new()
        },
        issues,
    }
}

// `fromEntries` defines own properties, so a server called `__proto__` stays a plain entry.
fn entries_of(reads: Vec<(&String, LspServerRead)>) -> IndexMap<String, LspServer> {
    in_key_order(
        reads
            .into_iter()
            .filter_map(|(name, read)| read.server.map(|server| (name.clone(), server)))
            .collect(),
    )
}

/// Joins the maps of a plugin in loading order: `.lsp.json` first, then each declared source
/// (manifest-reference: "A server name declared later replaces an earlier one"; components.md: the
/// manifest server replaces the `.lsp.json` one of the same name). Judgment: replacement is of the
/// whole entry, and a replaced name keeps its first position, so the registration order that decides
/// which server keeps a shared extension is the order names first appeared. Limitation: the result is
/// a plain object, so integer-like names (`"9"`, `"10"`) come first in ascending order whatever their
/// position (spec/judgments.md).
#[must_use]
pub fn merge_lsp_sources(sources: &[LspConfigRead]) -> LspConfigRead {
    let mut servers: IndexMap<String, LspServer> = IndexMap::new();
    for source in sources {
        for name in key_order(source.servers.keys()) {
            if let Some(server) = source.servers.get(name) {
                servers.insert(name.clone(), server.clone());
            }
        }
    }
    LspConfigRead {
        servers: in_key_order(servers),
        issues: sources
            .iter()
            .flat_map(|source| source.issues.iter().cloned())
            .collect(),
    }
}

crate::registry::exports! {
    "format/lsp-config" {
        "LSP_FIELDS" => ok || LSP_FIELDS;
        "readLspServer" => ok |name: String, raw: JsValue, origin: String|
            read_lsp_server(&name, &raw, &origin);
        "readLspConfig" => ok |raw: JsValue, origin: String| read_lsp_config(&raw, &origin);
        "mergeLspSources" => ok |sources: Vec<LspConfigRead>| merge_lsp_sources(&sources);
    }
}
