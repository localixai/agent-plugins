//! Port of `src/loading/resolve/user-config.ts`, switched.
//!
//! What the user (or the host) saved for a plugin's `userConfig` options
//! (plugins/manifest-reference.md, "User configuration"). The kit never stores or prompts: the
//! host reads `pluginConfigs` and its secure store and hands the values over as plain data.

use serde::{Deserialize, Serialize};

use crate::format::user_config_refs::{USER_CONFIG_KEY, user_config_references};
use crate::js::{JsObject, JsValue, array_join, regex_test, string_of};

/// The saved values of a plugin's options, and what the manifest declares (`ResolvedUserConfig`).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ResolvedUserConfig {
    /// The manifest's `userConfig`.
    #[serde(default)]
    pub declared: JsObject,
    /// The saved values by key, sensitive ones included; a bundled server's settings under
    /// `<server>.<key>`.
    #[serde(default)]
    pub values: JsObject,
}

/// The process variable a hook reads an option from: `api_token` is `CLAUDE_PLUGIN_OPTION_API_TOKEN`.
#[must_use]
pub fn option_env_name(key: &str) -> String {
    format!("CLAUDE_PLUGIN_OPTION_{}", key.to_uppercase())
}

/// The value `${user_config.KEY}` stands for: the saved one, else the declared `default` ("Value
/// used when the user provides nothing"), else `None`. Judgment: a key the manifest does not
/// declare has no value, whatever the host supplied.
#[must_use]
pub fn user_config_value(config: &ResolvedUserConfig, key: &str) -> Option<JsValue> {
    let declared = config.declared.get(key)?;
    match config.values.get(key) {
        Some(saved) => Some(saved.clone()),
        None => declared.get("default").cloned(),
    }
    .filter(|value| !value.is_undefined())
}

/// A value as text. The docs show strings only; judgment: a number or boolean is written as
/// `String(value)`, a list joined by commas (one argument, no spaces to split on).
#[must_use]
pub fn user_config_text(value: &JsValue) -> String {
    match value {
        JsValue::Array(items) => array_join(items, ","),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Object(_) => string_of(value),
    }
}

/// True when a template references `${user_config.*}`. It reads the template, never a value, so it
/// fires when nothing is configured yet (errors.md, "plugin-command-references-user-config"): a
/// shell field is refused for what it says, not for what is saved.
#[must_use]
pub fn references_user_config(template: &str) -> bool {
    user_config_references(template)
        .iter()
        .any(|key| regex_test(&USER_CONFIG_KEY, key))
}

/// The variables a hook process receives for the options (plugins/manifest-reference.md,
/// "Reference a saved value"): one per option, sensitive ones included. Judgment: an option
/// with neither a saved value nor a default has nothing to export and is left out.
#[must_use]
pub fn option_environment(config: &ResolvedUserConfig) -> JsObject {
    let mut exported = JsObject::new();
    for key in config.declared.keys() {
        if let Some(value) = user_config_value(config, key) {
            exported.insert(option_env_name(key), user_config_text(&value).into());
        }
    }
    exported
}

crate::registry::exports! {
    "loading/resolve/user-config" {
        "optionEnvName" => ok |key: String| option_env_name(&key);
        "userConfigValue" => ok |config: ResolvedUserConfig, key: String| user_config_value(&config, &key);
        "userConfigText" => ok |value: JsValue| user_config_text(&value);
        "referencesUserConfig" => ok |template: String| references_user_config(&template);
        "optionEnvironment" => ok |config: ResolvedUserConfig| option_environment(&config);
    }
}
