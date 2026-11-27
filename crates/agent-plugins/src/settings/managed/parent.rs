//! Port of `src/settings/managed/parent.ts`, switched.
//!
//! The parent settings an embedding host passes through the Agent SDK's `managedSettings` option
//! (managed-settings.md, "Let an embedding host add policy"; claude-apps-gateway.md, "How hook and
//! skill keys in parent settings take effect"). One rule, applied where the layers are read
//! (`parentSettingsOf`) and again over any view a host built itself (`applyParentRules`):
//!
//! - "By default, Claude Code ignores parent settings whenever an admin source is present";
//!   `parentSettingsBehavior: "merge"` in the highest-priority managed source keeps them.
//! - The checks Claude Code applies "to parent-supplied values on their own": a value of the
//!   managed settings it applies blocks the parent's for the keys below.
//! - An empty parent allowlist is ignored when the administrator "defines `hooks` or enables a
//!   plugin".
//!
//! Which keys a parent may carry at all is the key table's (`KEY_SPECS`).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::value::JsObject;
use crate::protocol::is_record::is_record;
use crate::registry::Port;
use crate::settings::layers::{PolicyLayer, PolicyScope};
use crate::settings::paths::{SettingsPath, value_at, with_value_at};
use crate::settings::view::{HostView, PolicyView, create_policy_view, host_view};

/// What the host says about the machine.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentSettingsContext {
    /// An administrator deploys managed settings on this machine, whether or not the host passed
    /// them.
    pub has_managed_settings: bool,
    /// The administrator set `parentSettingsBehavior: "merge"`.
    pub parent_merge: bool,
}

/// What the admin sources say, which decides what of the parent settings stays.
struct ParentChecks<'a> {
    /// The managed settings Claude Code applies.
    applied: &'a JsValue,
    /// The settings of every admin source that delivers a policy key, highest first.
    admins: &'a [JsValue],
    /// `managedSourcesBehavior` is `"merge"`.
    merging: bool,
}

fn is_set(settings: &JsValue, path: &str) -> bool {
    !value_at(settings, &SettingsPath::Dotted(path.to_owned())).is_undefined()
}

/// The condition under which the admin tier blocks a parent key.
#[derive(Clone, Copy)]
enum Block {
    /// "Claude Code enforces the value in the managed settings it applies and blocks a
    /// parent-supplied one."
    Applied(&'static str),
    /// `allowedProviders`: "a list in the selected managed source blocks a parent-supplied one,
    /// and under the `managedSourcesBehavior` `"merge"` opt-in a list in any admin source does."
    AllowedProviders,
    /// "When any admin source sets `allowManagedPermissionRulesOnly`, Claude Code drops
    /// parent-supplied permission allow rules and `additionalDirectories` as it reads them, even
    /// when a higher-priority source leaves the key unset."
    PermissionLock,
    /// The two allowlists a parent can carry, and only when empty: managed settings define a
    /// hook, or turn a plugin on.
    ShippedHooks,
}

/// Managed settings define a hook, or turn a plugin on.
fn ships_hooks(admins: &[JsValue]) -> bool {
    admins.iter().any(|doc| {
        let hooks = doc.get("hooks").filter(|hooks| is_record(hooks));
        let plugins = doc.get("enabledPlugins");
        hooks.is_some_and(|hooks| matches!(hooks, JsValue::Object(members) if !members.is_empty()))
            || matches!(plugins, Some(JsValue::Object(members))
                if members.values().any(|on| *on != JsValue::Bool(false)))
    })
}

fn blocks(block: Block, checks: &ParentChecks<'_>) -> bool {
    match block {
        Block::Applied(path) => is_set(checks.applied, path),
        Block::AllowedProviders => {
            if checks.merging {
                checks
                    .admins
                    .iter()
                    .any(|doc| is_set(doc, "allowedProviders"))
            } else {
                checks
                    .admins
                    .first()
                    .is_some_and(|doc| is_set(doc, "allowedProviders"))
            }
        }
        Block::PermissionLock => checks.admins.iter().any(|doc| {
            value_at(
                doc,
                &SettingsPath::Dotted("allowManagedPermissionRulesOnly".to_owned()),
            ) == JsValue::Bool(true)
        }),
        Block::ShippedHooks => ships_hooks(checks.admins),
    }
}

/// The parent keys the admin tier blocks, each with the condition. A parent
/// `blockedMarketplaces` is never blocked: it adds.
const BLOCKS: [(&str, Block); 9] = [
    ("permissions.allow", Block::PermissionLock),
    ("permissions.additionalDirectories", Block::PermissionLock),
    ("forceLoginOrgUUID", Block::Applied("forceLoginOrgUUID")),
    ("allowedMcpServers", Block::Applied("allowedMcpServers")),
    ("availableModels", Block::Applied("availableModels")),
    (
        "strictKnownMarketplaces",
        Block::Applied("strictKnownMarketplaces"),
    ),
    ("allowedProviders", Block::AllowedProviders),
    ("allowedHttpHookUrls", Block::ShippedHooks),
    ("httpHookAllowedEnvVars", Block::ShippedHooks),
];

/// What of the parent settings stays.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParentOutcome {
    /// The parent settings that count; `undefined` when an admin source is present and does not
    /// opt in.
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub settings: JsValue,
    /// The keys of the host dropped, by dotted path.
    pub dropped: Vec<String>,
}

/// The one rule: what of `parent` stays beside the admin sources.
fn kept_of(
    parent: &JsValue,
    checks: &ParentChecks<'_>,
    context: &ParentSettingsContext,
) -> ParentOutcome {
    if context.has_managed_settings && !context.parent_merge {
        let dropped = if let JsValue::Object(members) = parent {
            crate::js::property_order(members)
                .into_iter()
                .map(|(name, _)| name.clone())
                .collect()
        } else {
            Vec::new()
        };
        return ParentOutcome {
            settings: JsValue::Undefined,
            dropped,
        };
    }
    let dropped: Vec<String> = BLOCKS
        .iter()
        .filter(|(path, block)| is_set(parent, path) && blocks(*block, checks))
        .map(|(path, _)| (*path).to_owned())
        .collect();
    let settings = dropped.iter().fold(parent.clone(), |kept, path| {
        with_value_at(
            &kept,
            &SettingsPath::Dotted(path.clone()),
            &JsValue::Undefined,
        )
    });
    ParentOutcome { settings, dropped }
}

/// The members of `ManagedComposition` the rule reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentComposition {
    /// The managed tier.
    pub layer: PolicyLayer,
    /// `managedSourcesBehavior`.
    pub behavior: String,
    /// An administrator deploys managed settings here.
    pub has_managed_settings: bool,
    /// `parentSettingsBehavior` is `"merge"` in the highest-priority admin source.
    pub parent_merge: bool,
    /// The repaired settings of every admin source that delivers a policy key, highest first.
    pub admins: Vec<JsValue>,
}

/// What a host's parent settings come to beside the managed tier the kit composed.
#[must_use]
pub fn parent_settings_of(composition: &ParentComposition, parent: &JsValue) -> ParentOutcome {
    kept_of(
        parent,
        &ParentChecks {
            applied: &composition.layer.settings,
            admins: &composition.admins,
            merging: composition.behavior == "merge",
        },
        &ParentSettingsContext {
            has_managed_settings: composition.has_managed_settings,
            parent_merge: composition.parent_merge,
        },
    )
}

fn is_parent(layer: &PolicyLayer) -> bool {
    layer.scope == PolicyScope::Parent
}

/// The view `applyParentRules` answers: the one given, or a new one over the kept layers.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum RuledView {
    /// Nothing to drop: the view as it was given.
    Same(Port),
    /// The core's view over the kept layers.
    Made(HostView),
}

/// The view every area reads: the parent layers under the same rule, over the managed layers of
/// the view. A view with nothing to drop is returned as it is, so applying the rule again costs
/// nothing; a kept parent layer keeps its `invalid` keys, whose fail-closed floors still apply.
/// `None` is the view as given.
pub async fn apply_parent_rules(
    view: &dyn PolicyView,
    context: &ParentSettingsContext,
) -> Option<Vec<PolicyLayer>> {
    let given = view.layers().await;
    if !given.iter().any(is_parent) {
        return None;
    }
    let admins: Vec<JsValue> = given
        .iter()
        .filter(|layer| layer.scope == PolicyScope::Managed)
        .map(|one| one.settings.clone())
        .collect();
    let empty = JsValue::Object(JsObject::new());
    let checks = ParentChecks {
        applied: admins.first().unwrap_or(&empty),
        admins: &admins,
        merging: false,
    };
    let present = ParentSettingsContext {
        has_managed_settings: context.has_managed_settings || !admins.is_empty(),
        ..context.clone()
    };
    let mut changed = false;
    let mut layers = Vec::new();
    for layer in &given {
        if !is_parent(layer) {
            layers.push(layer.clone());
            continue;
        }
        let ParentOutcome { settings, dropped } = kept_of(&layer.settings, &checks, &present);
        if !settings.is_undefined() && dropped.is_empty() {
            layers.push(layer.clone());
            continue;
        }
        changed = true;
        if !settings.is_undefined() {
            layers.push(PolicyLayer {
                settings,
                ..layer.clone()
            });
        }
    }
    changed.then_some(layers)
}

async fn ruled_view(view: Port, context: ParentSettingsContext) -> RuledView {
    match apply_parent_rules(&view, &context).await {
        None => RuledView::Same(view),
        Some(layers) => {
            let given: Vec<JsValue> = layers
                .iter()
                .filter_map(|layer| crate::js::value::to_js_value(layer).ok())
                .collect();
            RuledView::Made(host_view(&Arc::new(create_policy_view(&given))))
        }
    }
}

crate::registry::exports! {
    "settings/managed/parent" {
        "parentSettingsOf" => ok |composition: ParentComposition, parent: JsValue| parent_settings_of(&composition, &parent);
        "applyParentRules" => async ok |view: Port, context: ParentSettingsContext| ruled_view(view, context).await;
    }
}
