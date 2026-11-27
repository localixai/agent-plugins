//! Port of `src/settings/mcp/managed.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::settings::layers::{PolicyLayer, PolicyScope};

/// The key itself; the entry validation of what a provided server may contain is not read here.
const KEY: &str = "managedMcpServers";

/// Which sources `managedMcpServers` is read from (managed-mcp.md, "Where `managedMcpServers` applies").
const fn read_from(scope: PolicyScope) -> bool {
    match scope {
        PolicyScope::Managed => true,
        PolicyScope::Parent
        | PolicyScope::Flag
        | PolicyScope::Local
        | PolicyScope::Project
        | PolicyScope::User
        | PolicyScope::AddDir => false,
    }
}

/// Sessions of the Claude Desktop app where Claude Code does not read the key: "the Claude Desktop
/// app's Code tab on a third-party deployment or ... the app's Cowork sessions, because Claude Desktop
/// supplies and locks those sessions' MCP servers itself".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesktopManaged {
    /// The Code tab on a third-party deployment.
    ThirdParty,
    /// Cowork sessions.
    Cowork,
}

const fn desktop_reason(desktop: DesktopManaged) -> &'static str {
    match desktop {
        DesktopManaged::ThirdParty => {
            "is not read in the Claude Desktop Code tab on a third-party deployment"
        }
        DesktopManaged::Cowork => "is not read in Claude Desktop Cowork sessions",
    }
}

/// Why a layer's key is not read, or `None` when it is.
fn dropped_reason(scope: PolicyScope, options: ManagedServerOptions) -> Option<&'static str> {
    if !read_from(scope) {
        return Some("is read from managed settings only");
    }
    options.desktop_managed.map(desktop_reason)
}

/// The servers the organization provides.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManagedServerEntries {
    /// Entries by server name, as written, for the entry validation to read.
    pub entries: IndexMap<String, JsValue>,
    /// What was dropped.
    pub issues: Vec<PluginIssue>,
}

/// What the session is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedServerOptions {
    /// The session is one of these Desktop sessions: no managed entry is read and a managed layer that carries the key is reported.
    #[serde(default)]
    pub desktop_managed: Option<DesktopManaged>,
}

/// The servers the organization provides: `managedMcpServers` of the managed layers only. The layers
/// are the managed sources the host selected, highest rank first; "when two sources define the same
/// name, the higher-ranked source's entry applies whole". Any other layer that carries the key has it
/// dropped with a warning. In a Desktop session that supplies its own servers (`desktopManaged`) no
/// entry is read; `/status` and `claude doctor` "say so when your managed settings carry the key
/// there", which this reports as a warning for each managed layer that carries it.
#[must_use]
pub fn managed_server_entries(
    layers: &[PolicyLayer],
    options: &ManagedServerOptions,
) -> ManagedServerEntries {
    let mut entries = JsObject::new();
    let mut issues = Vec::new();
    for layer in layers {
        let Some(value) = layer
            .settings
            .get(KEY)
            .filter(|value| !value.is_undefined())
        else {
            continue;
        };
        if let Some(dropped) = dropped_reason(layer.scope, *options) {
            issues.push(warning(
                &format!("{}:{KEY}", layer.scope.name()),
                &format!("\"{KEY}\" {dropped}; the key is dropped"),
            ));
            continue;
        }
        let Some(members) = value.as_object() else {
            continue;
        };
        for (name, entry) in property_order(members) {
            if !entries.contains_key(name) {
                entries.insert(name.clone(), entry.clone());
            }
        }
    }
    ManagedServerEntries {
        entries: in_property_order(entries),
        issues,
    }
}

crate::registry::exports! {
    "settings/mcp/managed" {
        "managedServerEntries" => ok |layers: Vec<PolicyLayer>, options: Option<ManagedServerOptions>| managed_server_entries(&layers, &options.unwrap_or_default());
    }
}
