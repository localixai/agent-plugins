//! Port of `src/settings/general/plugin-defaults.ts`, switched.
//!
//! A plugin is any object with `name` and `settings`, kept whole (its other members too) where the
//! package keeps it by reference.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue};
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// A plugin as its defaults are merged: its name (for the debug line) and its applied settings,
/// with whatever else the host keeps in it.
pub type PluginWithDefaults = JsValue;

/// The settings a plugin applies (`agent`, `subagentStatusLine`), in the order they were set.
pub type PluginSettings = JsObject;

/// plugins/components.md, "Default settings": last plugin wins, below every settings source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginDefaults {
    /// The merged settings.
    pub settings: PluginSettings,
    /// The plugin each key of `settings` came from: the one loaded last that sets it.
    pub owners: IndexMap<String, PluginWithDefaults>,
    /// The `claude --debug` lines: `Plugin "<b>" overrides setting "<key>" set by plugin "<a>"`.
    pub debug: Vec<String>,
}

const KEYS: [&str; 2] = ["agent", "subagentStatusLine"];

/// A plugin's `name`, as a template writes it.
fn name_of(plugin: &PluginWithDefaults) -> String {
    match plugin.get("name") {
        Some(JsValue::String(name)) => name.clone(),
        Some(other) => crate::js::json_stringify(other).unwrap_or_default(),
        None => "undefined".to_owned(),
    }
}

/// The plugins' defaults merged in the order given (the load order): a later plugin's key wins.
#[must_use]
pub fn plugin_defaults_of(plugins: &[PluginWithDefaults]) -> PluginDefaults {
    let mut owners: IndexMap<String, PluginWithDefaults> = IndexMap::new();
    let mut settings = PluginSettings::new();
    let mut debug: Vec<String> = Vec::new();
    for plugin in plugins {
        for key in KEYS {
            let Some(value) = plugin
                .get("settings")
                .and_then(|own| own.get(key))
                .filter(|value| !value.is_undefined())
            else {
                continue;
            };
            if let Some(previous) = owners.get(key) {
                debug.push(format!(
                    "Plugin \"{}\" overrides setting \"{key}\" set by plugin \"{}\"",
                    name_of(plugin),
                    name_of(previous)
                ));
            }
            settings.insert(key.to_owned(), value.clone());
            owners.insert(key.to_owned(), plugin.clone());
        }
    }
    PluginDefaults {
        settings,
        owners,
        debug,
    }
}

/// A key's value after the plugin defaults: from a settings source, or from the plugin named.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettingWithDefault {
    /// The value.
    pub value: JsValue,
    /// The plugin whose default it is; absent when a settings source sets the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<PluginWithDefaults>,
}

/// What the settings sources themselves set of the two keys.
async fn own_settings(view: &dyn PolicyView) -> PluginSettings {
    let mut own = PluginSettings::new();
    if let Some(agent) = view.get("agent").await {
        own.insert("agent".to_owned(), agent);
    }
    if let Some(line) = view.get("subagentStatusLine").await {
        own.insert("subagentStatusLine".to_owned(), line);
    }
    own
}

/// The value of `agent` or `subagentStatusLine`: "plugin defaults are the lowest layer, so a user's
/// own `agent` in `~/.claude/settings.json` overrides yours". `undefined` when nothing sets it.
pub async fn value_with_plugin_defaults(
    view: &dyn PolicyView,
    key: &str,
    defaults: &PluginDefaults,
) -> Option<SettingWithDefault> {
    if let Some(set) = own_settings(view).await.get(key) {
        return Some(SettingWithDefault {
            value: set.clone(),
            plugin: None,
        });
    }
    let value = defaults.settings.get(key)?;
    let plugin = defaults.owners.get(key)?;
    Some(SettingWithDefault {
        value: value.clone(),
        plugin: Some(plugin.clone()),
    })
}

crate::registry::exports! {
    "settings/general/plugin-defaults" {
        "pluginDefaultsOf" => ok |plugins: Vec<PluginWithDefaults>| plugin_defaults_of(&plugins);
        "valueWithPluginDefaults" => async ok |view: Port, key: String, defaults: PluginDefaults|
            value_with_plugin_defaults(&view, &key, &defaults).await;
    }
}
