//! Ported from `src/settings/hooks/hook-gate.ts`; only the core uses it now, so it has no wrapper.
//!
//! The three decisions about hooks that the settings make, read once from the view, and the
//! verdict per origin.

use std::sync::Arc;

use super::origin_stops::stops_of;
use super::origins::{
    ForceEnabled, HOOK_BLOCK_REASONS, HookBlockReason, HookVerdict, PolicyHookSource, Stops,
};
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::settings::layers::{KeyHolder, PolicyScope};
use crate::settings::marketplaces::command_sources::create_command_source_policy;
use crate::settings::plugins::enabled_plugins::force_enabled_by_managed;
use crate::settings::view::PolicyView;

/// What the customization package decides and this one only consumes: whether the `hooks` surface
/// of `strictPluginOnlyCustomization` is locked (settings-reference.md,
/// `strictPluginOnlyCustomization.hooks`). Taking it as an input keeps the hook locks from depending
/// on the package that owns that key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookLocks {
    /// The `hooks` surface is locked.
    pub hooks: bool,
}

/// The three decisions about hooks that the settings make, read once from the view.
#[derive(Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's HookGate holds one flag per decision"
)]
pub struct HookGate {
    /// `disableAllHooks` is `true` after precedence, in managed settings.
    pub managed_disable: bool,
    /// `disableAllHooks` is `true` after precedence, in any other file, or the SDK's
    /// `managedSettings` carries it as `true`, which is a lock and not a ranked value.
    pub other_disable: bool,
    /// `allowManagedHooksOnly` is `true`, or `managedSettings` carries `disableAllHooks: true`,
    /// which binds as it.
    pub managed_only: bool,
    /// The view, which `forceEnabled` reads.
    pub view: Arc<dyn PolicyView>,
    /// The customization locks it was made with.
    pub locks: HookLocks,
    /// Command sources are blocked.
    pub commands: bool,
}

impl std::fmt::Debug for HookGate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HookGate")
            .field("managed_disable", &self.managed_disable)
            .field("other_disable", &self.other_disable)
            .field("managed_only", &self.managed_only)
            .finish_non_exhaustive()
    }
}

/// `forceEnabled`: whether managed settings force-enable the plugin of this whole
/// `plugin@marketplace` id.
impl ForceEnabled for HookGate {
    fn force_enabled<'a>(&'a self, id: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(force_enabled_by_managed(&*self.view, id))
    }
}

/// Whether each lock stops a hook with these stops, in the order the reason is reported.
type Applies = fn(&Stops, &HookGate, HookLocks, bool) -> bool;

/// The reason named when several locks stop one hook. The docs do not rank them
/// (spec/judgments.md): a plugin that is never loaded comes first, then the widest setting, then
/// the narrower ones.
fn applies(reason: HookBlockReason) -> Applies {
    match reason {
        HookBlockReason::CommandSourceDisabled => {
            |stops, _gate, _locks, commands| commands && stops.command_source
        }
        HookBlockReason::DisableAllHooks => |stops, gate, _locks, _commands| {
            (gate.managed_disable && stops.managed_disable)
                || (gate.other_disable && stops.other_disable)
        },
        HookBlockReason::AllowManagedHooksOnly => {
            |stops, gate, _locks, _commands| gate.managed_only && stops.managed_only
        }
        HookBlockReason::StrictPluginOnlyCustomizationHooks => {
            |stops, _gate, locks, _commands| locks.hooks && stops.plugin_only
        }
    }
}

/// `disableAllHooks: true` in the SDK's `managedSettings` "binds as `allowManagedHooksOnly: true`"
/// (the SDK's `managedSettings` option), so it is a lock that no `disableAllHooks` of another file
/// can lift. A `false` there is an ordinary value and ranks like any other.
fn binds_as_lock(holder: &KeyHolder) -> bool {
    holder.scope == PolicyScope::Parent && holder.value == JsValue::Bool(true)
}

/// The gate over `view`.
pub async fn create_hook_gate(view: Arc<dyn PolicyView>, locks: HookLocks) -> HookGate {
    let holders = view.each_from_highest("disableAllHooks").await;
    let ranked = holders.iter().find(|holder| !binds_as_lock(holder));
    let parent_lock = holders.iter().any(binds_as_lock);
    let disabled = ranked.is_some_and(|holder| holder.value == JsValue::Bool(true));
    let managed = ranked.is_some_and(|holder| holder.scope == PolicyScope::Managed);
    let managed_only =
        parent_lock || view.get("allowManagedHooksOnly").await == Some(JsValue::Bool(true));
    let commands = create_command_source_policy(&*view).await.disabled;
    HookGate {
        managed_disable: disabled && managed,
        other_disable: parent_lock || (disabled && !managed),
        managed_only,
        view,
        locks,
        commands,
    }
}

impl HookGate {
    /// Whether a hook of `origin` runs, and the first lock that stops it.
    pub async fn runs(&self, origin: &PolicyHookSource) -> HookVerdict {
        let stops = stops_of(origin, self).await;
        let reason = HOOK_BLOCK_REASONS
            .into_iter()
            .find(|each| applies(*each)(&stops, self, self.locks, self.commands));
        HookVerdict {
            run: reason.is_none(),
            reason,
        }
    }
}
