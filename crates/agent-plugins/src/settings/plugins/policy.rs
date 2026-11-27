//! Port of `src/settings/plugins/policy.ts`, switched.
//!
//! The plugin keys of the settings as one object over a view (docs/API.md, "Settings").

use std::sync::Arc;

use super::enabled_plugins::{enabled_plugin, force_enabled_by_managed};
use super::mod_order::{ModOrderContext, mod_order};
use super::plugin_config::plugin_config;
use super::suggestions::suggestion_marketplaces;
use super::sync_plugins::sync_decision;
use super::trust_warning::{STANDARD_TRUST_WARNING, trust_warning};
use crate::js::JsValue;
use crate::registry::{Handle, Port, Set};
use crate::settings::view::PolicyView;

/// The plugin keys of the settings as one object over a view.
pub struct PluginSettingsPolicy {
    view: Arc<dyn PolicyView>,
    context: ModOrderContext,
}

/// The policy over `view`.
#[must_use]
pub fn create_plugin_settings_policy(
    view: Arc<dyn PolicyView>,
    context: ModOrderContext,
) -> PluginSettingsPolicy {
    PluginSettingsPolicy { view, context }
}

crate::registry::methods! {
    PluginSettingsPolicy {
        "enabled" => sync async ok |this, id: String, manifest_default_enabled: Option<bool>|
            enabled_plugin(&*this.view, &id, manifest_default_enabled.unwrap_or(true)).await;
        "forceEnabledByManaged" => sync async ok |this, id: String|
            force_enabled_by_managed(&*this.view, &id).await;
        "config" => sync async ok |this, id: String| plugin_config(&*this.view, &id).await;
        "modOrder" => sync async ok |this, org_mods: Set<String>|
            mod_order(&*this.view, &this.context, &org_mods.members).await;
        "syncClaudeAiPlugins" => sync async ok |this| sync_decision(&*this.view).await;
        "trustWarning" => sync async ok |this, standard: Option<String>|
            trust_warning(&*this.view, standard.as_deref().unwrap_or(STANDARD_TRUST_WARNING)).await;
        "suggestionMarketplaces" => sync async ok |this, registered: JsValue|
            suggestion_marketplaces(&*this.view, &registered).await;
    }
}

crate::registry::exports! {
    "settings/plugins/policy" {
        "createPluginSettingsPolicy" => ok |view: Port, context: ModOrderContext|
            Handle::new(create_plugin_settings_policy(Arc::new(view), context));
    }
}
