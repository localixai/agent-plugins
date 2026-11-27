//! Port of `src/loading/resolve/lsp-runtime.ts`, switched.
//!
//! The parts of an LSP server's runtime the kit decides as pure functions over data
//! (plugins/manifest-reference.md, "lspServers"; plugins/components.md, "LSP servers"). Starting,
//! stopping and timing the process, and speaking the protocol, are the host's LSP client.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::lsp::LSP_DEFAULTS;
use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsObject, JsValue, code_unit_length};

/// Whether a crashed server restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartDecision {
    /// It restarts.
    Restart,
    /// It stays stopped.
    StayStopped,
}

/// `Pick<LspServer, 'restartOnCrash' | 'maxRestarts'>`.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestartPolicy {
    /// `restartOnCrash`.
    #[serde(default)]
    pub restart_on_crash: Option<bool>,
    /// `maxRestarts`.
    #[serde(default)]
    pub max_restarts: Option<f64>,
}

/// Whether a crashed server is restarted. `restartOnCrash` false "leave\[s\] a crashed server
/// stopped"; `maxRestarts` is the "restart attempts before giving up, zero or more".
/// `restartsSoFar` counts the attempts already made for this server (judgment: not the crash in
/// hand, so with `maxRestarts: 2` the first two crashes restart and the third does not). With
/// `maxRestarts` unset, restarts are unbounded (judgment J-L2: the docs name no default).
#[must_use]
pub fn lsp_restart_decision(server: RestartPolicy, restarts_so_far: f64) -> RestartDecision {
    let enabled = server
        .restart_on_crash
        .unwrap_or(LSP_DEFAULTS.restart_on_crash);
    let within_limit = server
        .max_restarts
        .is_none_or(|limit| restarts_so_far < limit);
    if enabled && within_limit {
        RestartDecision::Restart
    } else {
        RestartDecision::StayStopped
    }
}

/// The member a payload is read from (`initializationOptions`, `settings`), `undefined` when the
/// server has none.
fn member(server: &JsValue, key: &str) -> JsValue {
    server.get(key).cloned().unwrap_or_default()
}

/// The part of the `initialize` request params this config decides: "Options sent in the
/// initialize request". The host adds its own (process ID, root, capabilities) around it.
#[must_use]
pub fn initialize_options_of(server: &JsValue) -> JsObject {
    let options = member(server, "initializationOptions");
    if options.is_undefined() {
        JsObject::new()
    } else {
        JsObject::from([("initializationOptions".to_owned(), options)])
    }
}

/// The notification that carries `settings` ("Settings sent by
/// `workspace/didChangeConfiguration`"). Judgment: a server without `settings` is sent none.
#[must_use]
pub fn did_change_configuration_of(server: &JsValue) -> Option<JsObject> {
    let settings = member(server, "settings");
    if settings.is_undefined() {
        return None;
    }
    Some(JsObject::from([
        (
            "method".to_owned(),
            "workspace/didChangeConfiguration".into(),
        ),
        (
            "params".to_owned(),
            JsValue::Object(JsObject::from([("settings".to_owned(), settings)])),
        ),
    ]))
}

/// `Pick<LspServer, 'extensionToLanguage'>`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routed {
    /// The file extensions' languages.
    #[serde(default)]
    pub extension_to_language: IndexMap<String, String>,
}

/// The server and language of a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRoute {
    /// The server's key.
    pub server: String,
    /// The language ID.
    pub language_id: String,
}

fn last_segment(file_path: &str) -> &str {
    file_path
        .split(['/', '\\'])
        .next_back()
        .unwrap_or(file_path)
}

/// The server and language ID for a file: "each file extension gets one server" and "the first
/// registered handles those files". `servers` is in registration order (the order of the record)
/// and its key must be unique across every plugin, so qualify it (`plugin:server`): two plugins
/// that each declare `go` would otherwise collide in the record and one would vanish.
/// Judgment: extensions match as written (case-sensitive) at the end of the last path segment, a
/// name that is only the extension (`.ts`) has none, and the longest matching extension wins
/// (`.d.ts` over `.ts`), the first registered server among equals.
#[must_use]
pub fn language_id_for(
    servers: &IndexMap<String, Routed>,
    file_path: &str,
) -> Option<LanguageRoute> {
    let name = last_segment(file_path);
    let mut best: Option<(&str, &str, usize)> = None;
    for (server, config) in servers {
        for (extension, language_id) in &config.extension_to_language {
            let length = code_unit_length(extension);
            if code_unit_length(name) > length
                && name.ends_with(extension.as_str())
                && best.is_none_or(|(_, _, found)| length > found)
            {
                best = Some((server, language_id, length));
            }
        }
    }
    best.map(|(server, language_id, _)| LanguageRoute {
        server: server.to_owned(),
        language_id: language_id.to_owned(),
    })
}

/// The warning components.md quotes for a server that loses an extension to an earlier one:
/// `LSP server "<name>" is not used for <ext> files`. One per server and extension. `<name>` is
/// the record key, so it is plugin-qualified (`plugin:server`) when the servers come from several
/// plugins.
#[must_use]
pub fn lsp_extension_conflicts(servers: &IndexMap<String, Routed>) -> Vec<PluginIssue> {
    let mut owner: IndexMap<&str, &str> = IndexMap::new();
    let mut issues = Vec::new();
    for (name, config) in servers {
        for extension in config.extension_to_language.keys() {
            if owner.contains_key(extension.as_str()) {
                let message = format!("LSP server \"{name}\" is not used for {extension} files");
                issues.push(warning(name, &message));
            } else {
                owner.insert(extension, name);
            }
        }
    }
    issues
}

crate::registry::exports! {
    "loading/resolve/lsp-runtime" {
        "lspRestartDecision" => ok |server: RestartPolicy, restarts_so_far: f64|
            lsp_restart_decision(server, restarts_so_far);
        "initializeOptionsOf" => ok |server: JsValue| initialize_options_of(&server);
        "didChangeConfigurationOf" => ok |server: JsValue| did_change_configuration_of(&server);
        "languageIdFor" => ok |servers: IndexMap<String, Routed>, file_path: String|
            language_id_for(&servers, &file_path);
        "lspExtensionConflicts" => ok |servers: IndexMap<String, Routed>|
            lsp_extension_conflicts(&servers);
    }
}
