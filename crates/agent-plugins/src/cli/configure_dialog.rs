//! Port of `src/cli/configure-dialog.ts`, switched.
//!
//! When the configuration dialog appears and what it starts with (components.md, "When the
//! configuration dialog appears").

use serde::{Deserialize, Serialize};

use super::user_config_input::unset_keys;
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::loading::resolve::user_config::{ResolvedUserConfig, user_config_value};

/// Which options an event's dialog asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selection {
    Unset,
    All,
    None,
}

/// The events that may open the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigurationEvent {
    /// `session-install`.
    SessionInstall,
    /// `session-enable`.
    SessionEnable,
    /// `session-configure`.
    SessionConfigure,
    /// `vscode-install`.
    VscodeInstall,
    /// `vscode-configure`.
    VscodeConfigure,
    /// `shell-install`.
    ShellInstall,
}

const fn selection_of(event: ConfigurationEvent) -> Selection {
    match event {
        ConfigurationEvent::SessionInstall
        | ConfigurationEvent::SessionEnable
        | ConfigurationEvent::VscodeInstall => Selection::Unset,
        ConfigurationEvent::SessionConfigure | ConfigurationEvent::VscodeConfigure => {
            Selection::All
        }
        ConfigurationEvent::ShellInstall => Selection::None,
    }
}

/// The dialog a host renders.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfigurationDialog {
    /// Whether it opens.
    pub open: bool,
    /// The options it asks for.
    pub keys: Vec<String>,
    /// Sensitive starting values never leave the secure store.
    pub inputs: JsObject,
}

fn select(selection: Selection, config: &ResolvedUserConfig) -> Vec<String> {
    match selection {
        Selection::Unset => unset_keys(config),
        Selection::All => property_order(&config.declared)
            .into_iter()
            .map(|(key, _)| key.clone())
            .collect(),
        Selection::None => Vec::new(),
    }
}

/// components.md § When the configuration dialog appears. The host renders this plan, never shell install.
#[must_use]
pub fn configuration_dialog(
    event: ConfigurationEvent,
    config: &ResolvedUserConfig,
) -> ConfigurationDialog {
    let keys = select(selection_of(event), config);
    let mut inputs = JsObject::new();
    for key in &keys {
        let sensitive = config
            .declared
            .get(key)
            .and_then(|option| option.get("sensitive"))
            == Some(&JsValue::Bool(true));
        if let Some(value) = user_config_value(config, key).filter(|_| !sensitive) {
            inputs.insert(key.clone(), value);
        }
    }
    // Judgment: a plugin with no options needs no empty dialog, even for an explicit configure.
    ConfigurationDialog {
        open: !keys.is_empty(),
        keys,
        inputs: in_property_order(inputs),
    }
}

crate::registry::exports! {
    "cli/configure-dialog" {
        "configurationDialog" => ok |event: ConfigurationEvent, config: ResolvedUserConfig| configuration_dialog(event, &config);
    }
}
