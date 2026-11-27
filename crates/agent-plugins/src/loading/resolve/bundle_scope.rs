//! Ported from `src/loading/resolve/bundle-scope.ts`; only the core uses it now, so it has no wrapper.

use serde::Deserialize;

use super::substitute::scope::SubstitutionScope;
use super::user_config::ResolvedUserConfig;

use crate::format::issues::{PluginIssue, error};
use crate::format::mcpb_manifest::{
    OptionState, RequiredInput, bundle_setting_key, missing_bundle_settings,
    required_bundle_settings,
};
use crate::js::{JsObject, JsValue};

/// What `bundleScope` reads of a bundle (`McpBundleConfig`): the settings it declares.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleConfig {
    /// The settings by key, each as the bundle declares it.
    pub user_config: JsValue,
}

/// "A bundled server with a required setting that has no saved value doesn't start" (plugins/components.md).
fn needs_configuration(server: &str) -> PluginIssue {
    error(
        "mcpServers",
        &format!("Bundled MCP server \"{server}\" was not started: it needs configuration"),
    )
}

/// A saved value a reference can stand for: text, a number or a Boolean.
const fn is_plain(value: Option<&JsValue>) -> bool {
    matches!(
        value,
        Some(JsValue::String(_) | JsValue::Number(_) | JsValue::Bool(_))
    )
}

/// The settings a bundle declares, by key.
fn declared_of(bundle: &BundleConfig) -> JsObject {
    bundle.user_config.as_object().cloned().unwrap_or_default()
}

/// What `requiredBundleSettings` reads of one declared setting.
fn option_state_of(setting: &JsValue) -> OptionState {
    OptionState {
        required: setting.get("required").cloned().unwrap_or_default(),
        default: setting.get("default").cloned().unwrap_or_default(),
    }
}

/// The plain values saved for one bundled server: the plugin's values under `<server>.<key>`.
fn saved_of<L, E>(scope: &SubstitutionScope<L, E>, server: &str, declared: &JsObject) -> JsObject {
    let values = &scope.user_config.values;
    declared
        .keys()
        .filter_map(|key| {
            let value = values.get(&bundle_setting_key(server, key));
            is_plain(value).then(|| value.map(|plain| (key.clone(), plain.clone())))?
        })
        .collect()
}

/// The scope of a bundled server, or why it does not start.
#[derive(Clone, Debug)]
pub enum BundleScope<L, E> {
    /// The scope the server's references read.
    Scope {
        /// The plugin's scope with the bundle's own settings in place of its `userConfig`.
        scope: SubstitutionScope<L, E>,
    },
    /// Why the server does not start.
    Issue {
        /// The issue.
        issue: PluginIssue,
    },
}

/// The scope of a bundled server, or why it does not start (plugins/components.md, "Include a packaged
/// MCPB server"). Its `${user_config.KEY}` reads its bundle's own settings, never the plugin's
/// `userConfig`: the values saved under `<server>.<key>` (`pluginConfigs.<id>.mcpServers.<server>`, the
/// host's store over it; the `--config` form), else the setting's default, answered in the same one
/// pass as every other reference, so a value holding `${...}` stays that text. The verdict is made here,
/// when the server is resolved for a session, so one loaded plugin serves sessions with different
/// values; a value that is not text, a number or a Boolean is none (judgments, spec/judgments.md).
#[must_use]
pub fn bundle_scope<L: Clone, E: Clone>(
    scope: &SubstitutionScope<L, E>,
    server: &str,
    bundle: &BundleConfig,
) -> BundleScope<L, E> {
    let declared = declared_of(bundle);
    let saved = saved_of(scope, server, &declared);
    let flat: JsObject = saved
        .iter()
        .map(|(key, value)| (bundle_setting_key(server, key), value.clone()))
        .collect();
    let required = required_bundle_settings(&RequiredInput {
        name: server.to_owned(),
        options: declared
            .iter()
            .map(|(key, setting)| (key.clone(), option_state_of(setting)))
            .collect(),
    });
    if !missing_bundle_settings(&required, &flat).is_empty() {
        return BundleScope::Issue {
            issue: needs_configuration(server),
        };
    }
    BundleScope::Scope {
        scope: SubstitutionScope {
            user_config: ResolvedUserConfig {
                declared,
                values: saved,
            },
            ..scope.clone()
        },
    }
}
