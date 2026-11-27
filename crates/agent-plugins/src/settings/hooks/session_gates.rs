//! Port of `src/settings/hooks/session-gates.ts`, switched.
//!
//! What a hook session needs of the settings, as plain data: the one hook decision
//! `createHookPolicy` also gives, in the shape the hook engine's session takes (hooks.md, "Disable
//! or remove hooks", "Hook locations"). Settings is a lower layer than the engine, so the shape is
//! structural, not a type of `hooks`.

use std::sync::Arc;

use serde::Serialize;

use super::hook_gate::{HookGate, HookLocks, create_hook_gate};
use super::http_policy::{HttpHookUrlPolicy, create_http_hook_policy};
use super::origins::ForceEnabled;
use crate::js::JsValue;
use crate::registry::{Function, Port};
use crate::settings::customization::surfaces::read_surface_locks;
use crate::settings::layers::PolicyScope;
use crate::settings::managed::parent::{ParentSettingsContext, effective_view};
use crate::settings::view::PolicyView;

/// `effective`: `disableAllHooks` left after precedence; `managedLocked`: set in managed settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableAllHooks {
    /// `disableAllHooks` left after precedence.
    pub effective: bool,
    /// Set in managed settings.
    pub managed_locked: bool,
}

/// The merged HTTP hook allowlists; a key no file sets is absent (no limit).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHttpPolicy {
    /// `allowedHttpHookUrls`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_urls: Option<JsValue>,
    /// `httpHookAllowedEnvVars`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_env_vars: Option<JsValue>,
}

/// The `hooks` of one settings file that declares any.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettingsHooks {
    /// The file's scope.
    pub scope: PolicyScope,
    /// Its `hooks`.
    pub hooks: JsValue,
}

/// What a hook session needs of the settings.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSessionGates {
    /// `disableAllHooks`.
    pub disable_all_hooks: DisableAllHooks,
    /// `allowManagedHooksOnly`.
    pub allow_managed_hooks_only: bool,
    /// `strictPluginOnlyCustomization` locks the `hooks` surface.
    pub plugin_only_hooks: bool,
    /// The merged HTTP hook allowlists; a key no file sets is absent (no limit).
    pub http_policy: SessionHttpPolicy,
    /// The `hooks` of each settings file that declares any, highest precedence first.
    pub settings_hooks: Vec<SettingsHooks>,
    /// The plugin is force-enabled in managed `enabledPlugins`, which `allowManagedHooksOnly`
    /// exempts.
    pub force_enabled: Function,
    /// The gate `force_enabled` asks, for the core's own reads (`host/state.rs`).
    #[serde(skip)]
    pub gate: Arc<HookGate>,
}

/// The decision about hooks the settings make, read once from a view the parent rules were
/// applied to.
pub struct HookDecision {
    /// The gate.
    pub gate: Arc<HookGate>,
    /// `strictPluginOnlyCustomization` locks the `hooks` surface.
    pub plugin_only_hooks: bool,
    /// The HTTP hook allowlists.
    pub http: HttpHookUrlPolicy,
    /// The view it was read from.
    pub view: Arc<dyn PolicyView>,
}

/// The decision over `effective`.
pub async fn hook_decision(effective: Arc<dyn PolicyView>) -> HookDecision {
    let locks = effective.get("strictPluginOnlyCustomization").await;
    let plugin_only_hooks = read_surface_locks(&locks.unwrap_or_default())
        .locked
        .members
        .contains(&"hooks");
    HookDecision {
        gate: Arc::new(
            create_hook_gate(
                Arc::clone(&effective),
                HookLocks {
                    hooks: plugin_only_hooks,
                },
            )
            .await,
        ),
        plugin_only_hooks,
        http: create_http_hook_policy(&*effective).await,
        view: effective,
    }
}

/// The gate's `forceEnabled`, as the host calls it.
pub struct ForceEnabledOf(pub Arc<HookGate>);

crate::registry::methods! {
    ForceEnabledOf {
        "call" => sync async ok |this, plugin_id: String| this.0.force_enabled(&plugin_id).await;
    }
}

/// The decision as the hook engine's session takes it.
async fn session_gates_of(decision: HookDecision) -> HookSessionGates {
    let HookDecision {
        gate,
        plugin_only_hooks,
        view,
        ..
    } = decision;
    let urls = view.get("allowedHttpHookUrls").await;
    let variables = view.get("httpHookAllowedEnvVars").await;
    let settings_hooks = view
        .each_from_highest("hooks")
        .await
        .into_iter()
        .map(|holder| SettingsHooks {
            scope: holder.scope,
            hooks: holder.value,
        })
        .collect();
    HookSessionGates {
        disable_all_hooks: DisableAllHooks {
            effective: gate.managed_disable || gate.other_disable,
            managed_locked: gate.managed_disable,
        },
        allow_managed_hooks_only: gate.managed_only,
        plugin_only_hooks,
        http_policy: SessionHttpPolicy {
            allowed_urls: urls,
            allowed_env_vars: variables,
        },
        settings_hooks,
        force_enabled: Function::new(ForceEnabledOf(Arc::clone(&gate))),
        gate,
    }
}

/// The session facts of the hook gates over these settings, under the parent-settings rules.
pub async fn hook_session_gates(
    view: Arc<dyn PolicyView>,
    context: &ParentSettingsContext,
) -> HookSessionGates {
    session_gates_of(hook_decision(effective_view(view, context).await).await).await
}

crate::registry::exports! {
    "settings/hooks/session-gates" {
        "hookSessionGates" => async ok |view: Port, context: ParentSettingsContext|
            hook_session_gates(Arc::new(view), &context).await;
    }
}
