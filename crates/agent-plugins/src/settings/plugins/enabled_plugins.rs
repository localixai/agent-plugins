//! Port of `src/settings/plugins/enabled-plugins.ts`, switched.
//!
//! Whether a plugin is on, read from `enabledPlugins` through the host's `PolicyView`.

use serde::{Serialize, Serializer};

use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::view::{KeyHolder, PolicyView};

/// Where an `enabledPlugins` decision came from: a file, or the plugin's own `defaultEnabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecidedBy {
    /// A file of this scope.
    Scope(PolicyScope),
    /// The plugin's own `defaultEnabled`.
    DefaultEnabled,
}

/// Written as the package writes it: the scope's name, or `defaultEnabled`.
impl Serialize for DecidedBy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Scope(scope) => scope.name(),
            Self::DefaultEnabled => "defaultEnabled",
        })
    }
}

/// Whether a plugin is on, and what decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnabledDecision {
    /// Whether the plugin is on.
    pub on: bool,
    /// What decided it.
    pub decided_by: DecidedBy,
    /// A `false` in managed settings: "blocked from installation at every scope".
    pub blocked_by_managed: bool,
    /// The same `false`: "hidden from the marketplace".
    pub hidden_in_marketplace: bool,
}

const KEY: &str = "enabledPlugins";

/// Whether a file's entry for the id counts. A `--add-dir` directory is the lowest source and "Only a
/// `true` value has an effect" there (plugins/loading.md, "Find where a plugin is enabled"), so any
/// other value of its entry is passed over and a lower file, or the manifest, decides.
fn decides(holder: &KeyHolder, id: &str) -> bool {
    holder.value.get(id).is_some()
        && (holder.scope != PolicyScope::AddDir
            || holder.value.get(id) == Some(&JsValue::Bool(true)))
}

/// Whether a plugin is on (settings-reference.md, `enabledPlugins`; plugins/org.md, "Control
/// matrix"). The highest file that has an entry for this id decides, so project beats user, local
/// beats project, and managed beats every other file: a managed `true` cannot be turned off and a
/// managed `false` cannot be turned on. With no entry anywhere the plugin follows its manifest
/// `defaultEnabled`, which is `true` when the manifest does not say (manifest-reference.md).
///
/// A `--add-dir` directory ranks below user settings, and only its `true` counts: it never turns a
/// plugin off and never overrides another file (plugins/loading.md).
///
/// Judgment (spec/judgments.md): the SDK also types a list or an object as an entry, which the
/// settings docs do not describe; only `false` turns a plugin off, so those read as enabled.
pub async fn enabled_plugin(
    view: &dyn PolicyView,
    id: &str,
    manifest_default_enabled: bool,
) -> EnabledDecision {
    let holder = view
        .each_from_highest(KEY)
        .await
        .into_iter()
        .find(|each| decides(each, id));
    let Some(holder) = holder else {
        return EnabledDecision {
            on: manifest_default_enabled,
            decided_by: DecidedBy::DefaultEnabled,
            blocked_by_managed: false,
            hidden_in_marketplace: false,
        };
    };
    let on = holder.value.get(id) != Some(&JsValue::Bool(false));
    let blocked = holder.scope == PolicyScope::Managed && !on;
    EnabledDecision {
        on,
        decided_by: DecidedBy::Scope(holder.scope),
        blocked_by_managed: blocked,
        hidden_in_marketplace: blocked,
    }
}

/// Whether managed settings force-enable the plugin. The id is matched whole, so a plugin of the
/// same name from another marketplace is not matched (settings-reference.md, `allowManagedHooksOnly`).
pub async fn force_enabled_by_managed(view: &dyn PolicyView, id: &str) -> bool {
    let decision = enabled_plugin(view, id, true).await;
    decision.decided_by == DecidedBy::Scope(PolicyScope::Managed) && decision.on
}

crate::registry::exports! {
    "settings/plugins/enabled-plugins" {
        "enabledPlugin" => async ok |view: Port, id: String, manifest_default_enabled: Option<bool>|
            enabled_plugin(&view, &id, manifest_default_enabled.unwrap_or(true)).await;
        "forceEnabledByManaged" => async ok |view: Port, id: String|
            force_enabled_by_managed(&view, &id).await;
    }
}
