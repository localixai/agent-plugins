//! Port of `src/settings/plugins/plugin-config.ts`, switched.
//!
//! The answers stored for a plugin under `pluginConfigs` (settings-reference.md, `pluginConfigs`).

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::value::JsObject;
use crate::js::{JsValue, property_order};
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::plugins::plugin_id::{AGENTS_MD_ID_FORMS, is_guard_plugin_id, spellings_of};
use crate::settings::view::PolicyView;

/// What is stored for a plugin.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    /// The non-sensitive answers to the plugin's `userConfig` dialog.
    pub options: JsObject,
    /// The per-server user configuration values, by MCP server name.
    pub mcp_servers: JsObject,
    /// What was dropped from the entry, as warnings.
    pub issues: Vec<PluginIssue>,
}

const KEY: &str = "pluginConfigs";

/// The built-in plugins `pluginConfigs` reads under two ids. The guard is not here on purpose:
/// "Claude Code reads the options only under `cc-plugin-sec-default@builtin` ... `pluginConfigs`
/// doesn't" accept `sec-default@builtin` (plugins/mods/admin.md, "Set options on the built-in
/// guard").
fn config_id_forms() -> [[String; 2]; 1] {
    [AGENTS_MD_ID_FORMS.map(str::to_owned)]
}

/// What an option or a server value is: "a string, number, Boolean, or array of strings", as the
/// SDK types it.
fn is_option_value(value: &JsValue) -> bool {
    match value {
        JsValue::String(_) | JsValue::Bool(_) => true,
        JsValue::Number(number) => number.is_finite(),
        JsValue::Array(items) => items.iter().all(|item| item.as_str().is_some()),
        JsValue::Undefined | JsValue::Null | JsValue::Object(_) => false,
    }
}

fn entries_of(entry: Option<&JsValue>) -> Vec<(&String, &JsValue)> {
    match entry {
        Some(JsValue::Object(members)) => property_order(members),
        _ => Vec::new(),
    }
}

/// The values of `entry` that are option values; a value of another shape is dropped with a
/// warning.
fn read_values(entry: Option<&JsValue>, path: &str) -> (JsObject, Vec<PluginIssue>) {
    let mut issues = Vec::new();
    let mut values = JsObject::new();
    for (name, value) in entries_of(entry) {
        if is_option_value(value) {
            values.insert(name.clone(), value.clone());
        } else {
            issues.push(warning(
                &format!("{path}.{name}"),
                "Expected a string, number, Boolean or array of strings; the value is dropped",
            ));
        }
    }
    (values, issues)
}

fn read_servers(entry: Option<&JsValue>, path: &str) -> (JsObject, Vec<PluginIssue>) {
    let mut issues = Vec::new();
    let mut servers = JsObject::new();
    for (name, server) in entries_of(entry) {
        if !server.is_object() {
            issues.push(warning(
                &format!("{path}.{name}"),
                "Expected an object of values; the server is dropped",
            ));
            continue;
        }
        let (values, read) = read_values(Some(server), &format!("{path}.{name}"));
        servers.insert(name.clone(), JsValue::Object(values));
        issues.extend(read);
    }
    (servers, issues)
}

/// The answers stored for a plugin (settings-reference.md, `pluginConfigs`). Only user and managed
/// settings carry the key, so a cloned repository cannot supply them. The highest file with a
/// usable entry for the plugin supplies it whole. A built-in plugin is found under either of its
/// ids. The built-in guard's options count in managed settings only: "the same entry in a user,
/// project, or local settings file, or in a file passed with `--settings`, neither sets an option
/// nor loosens one" (plugins/mods/admin.md, "Set options on the built-in guard").
///
/// Judgment (spec/judgments.md): entries of two files are not combined field by field, with both
/// ids of a built-in plugin in one file the current id wins, and an entry that is not an object is
/// dropped with a warning so the next file's entry applies; the docs say none of these.
pub async fn plugin_config(view: &dyn PolicyView, id: &str) -> PluginConfig {
    let spellings = spellings_of(&config_id_forms(), id);
    let holders = view.each_from_highest(KEY).await;
    let mut dropped = Vec::new();
    for holder in holders
        .iter()
        .filter(|holder| !is_guard_plugin_id(id) || holder.scope == PolicyScope::Managed)
    {
        let Some(spelling) = spellings
            .iter()
            .find(|form| holder.value.get(form).is_some())
        else {
            continue;
        };
        let entry = holder.value.get(spelling).cloned().unwrap_or_default();
        let path = format!("{KEY}:{spelling}");
        if !entry.is_object() {
            dropped.push(warning(
                &path,
                "Expected an object with an options field; the entry is dropped",
            ));
            continue;
        }
        let (options, option_issues) =
            read_values(entry.get("options"), &format!("{path}.options"));
        let (servers, server_issues) =
            read_servers(entry.get("mcpServers"), &format!("{path}.mcpServers"));
        dropped.extend(option_issues);
        dropped.extend(server_issues);
        return PluginConfig {
            options,
            mcp_servers: servers,
            issues: dropped,
        };
    }
    PluginConfig {
        options: JsObject::new(),
        mcp_servers: JsObject::new(),
        issues: dropped,
    }
}

crate::registry::exports! {
    "settings/plugins/plugin-config" {
        "pluginConfig" => async ok |view: Port, id: String| plugin_config(&view, &id).await;
    }
}
