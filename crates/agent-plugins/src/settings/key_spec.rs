//! Port of `src/settings/key-spec.ts`, switched.
//!
//! One row per policy key. A row's functions (`merge.combine`, `admin.pinnedWhen`) are Rust ones
//! the core's view calls (`settings/view.rs`); the package holds the rows as data (`KEY_SPECS`,
//! `SPECS`, below).

use std::collections::HashMap;
use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::settings::general::exceptions::{
    FALSE_FROM_ANY_SCOPE, ManagedException, TRUE_FROM_ANY_SCOPE,
};
use crate::settings::general::global_config::GLOBAL_CONFIG_KEYS;
use crate::settings::general::rows::GENERAL_SPECS;
use crate::settings::layers::PolicyScope;
use crate::settings::merge::{MERGES, MergeKind};
use crate::settings::value_spec::{AdminTier, ValueSpec, spec};

/// A policy key's row: its value row, and how an invalid value is repaired.
#[derive(Debug)]
pub struct KeySpec {
    /// The value row.
    pub spec: ValueSpec,
    /// The repair of `settings::repair::kinds` the key's invalid value gets.
    pub repair: RepairKind,
}

/// `RepairKind` of `settings::repair::kinds`: what to do with a value that does not have the
/// type its key documents. Declared here, its first use in the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    missing_docs,
    reason = "each variant is the repair of its name in repair/kinds.ts"
)]
pub enum RepairKind {
    Boolean,
    LockBoolean,
    HooksSwitch,
    String,
    StringList,
    ClosedStringList,
    ChannelAllowlist,
    AllowedMcpServers,
    DeniedMcpServers,
    StrictSources,
    BlockedSources,
    EnabledPlugins,
    MarketplaceMap,
    Record,
    SkillOverrides,
    WorkflowSize,
    PluginOnly,
    Hooks,
    AsWritten,
    ProviderAllowlist,
    MatchMode,
    ModelDenylist,
    OrgUuid,
    GatewayNetworks,
    Permissions,
    AutoMode,
    DisableMode,
    ConsentBoolean,
}

/// The files a key may come from. The docs' scope names are `Any file`, `Managed`, `User or
/// managed` and `User, local, or managed` (plus `--settings`); the rest are the parent settings an
/// embedding host passes (agent-sdk/typescript.md, `managedSettings`), which only the keys the
/// gateway page names accept (claude-apps-gateway.md, "How hook and skill keys in parent settings
/// take effect"). `userManagedFlag` is "User or managed" plus `--settings`, for the keys whose
/// entry says so, and `repositoryCannotElevate` admits any file except a repository's with an
/// elevating value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    missing_docs,
    reason = "each variant is the scope line of its name (above)"
)]
pub enum ScopeRule {
    Any,
    AnyAndParent,
    AnyAndEmptyListFromParent,
    Managed,
    ManagedAndParent,
    User,
    UserOrManaged,
    UserManagedFlag,
    UserLocalManagedFlag,
    RepositoryCannotElevate,
}

type Admits = Box<dyn Fn(PolicyScope, &JsValue) -> bool + Send + Sync>;

fn in_scopes(scopes: Vec<PolicyScope>) -> Admits {
    Box::new(move |scope, _| scopes.contains(&scope))
}

const FILES: [PolicyScope; 6] = [
    PolicyScope::Managed,
    PolicyScope::Flag,
    PolicyScope::Local,
    PolicyScope::Project,
    PolicyScope::AddDir,
    PolicyScope::User,
];
/// The files a checked-out repository controls, and the `permissions.defaultMode` values that do
/// not take effect from them: "`auto` and `bypassPermissions` don't take effect from project or
/// local settings; set them in user or managed settings instead" (settings.md, "A value you set is
/// ignored").
pub const REPOSITORY_FILES: [PolicyScope; 2] = [PolicyScope::Project, PolicyScope::Local];
fn is_elevated(value: &JsValue) -> bool {
    matches!(value.as_str(), Some("auto" | "bypassPermissions"))
}
fn is_empty_list(value: &JsValue) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
}
fn from_files_or_parent(parent_value: fn(&JsValue) -> bool) -> Admits {
    Box::new(move |scope, value| {
        FILES.contains(&scope) || (scope == PolicyScope::Parent && parent_value(value))
    })
}

fn with(scopes: &[PolicyScope], more: &[PolicyScope]) -> Vec<PolicyScope> {
    scopes.iter().chain(more).copied().collect()
}

/// Which scopes (and, for one rule, which parent values) each rule admits.
static ADMITS: LazyLock<HashMap<ScopeRule, Admits>> = LazyLock::new(|| {
    use PolicyScope::{Flag, Local, Managed, Parent, User};
    HashMap::from([
        (ScopeRule::Any, in_scopes(FILES.to_vec())),
        (ScopeRule::AnyAndParent, in_scopes(with(&FILES, &[Parent]))),
        (
            ScopeRule::AnyAndEmptyListFromParent,
            from_files_or_parent(is_empty_list),
        ),
        (ScopeRule::Managed, in_scopes(vec![Managed])),
        (
            ScopeRule::ManagedAndParent,
            in_scopes(vec![Managed, Parent]),
        ),
        (ScopeRule::User, in_scopes(vec![User])),
        (ScopeRule::UserOrManaged, in_scopes(vec![User, Managed])),
        (
            ScopeRule::UserManagedFlag,
            in_scopes(vec![User, Managed, Flag]),
        ),
        (
            ScopeRule::RepositoryCannotElevate,
            Box::new(|scope, value: &JsValue| {
                FILES.contains(&scope) && !(REPOSITORY_FILES.contains(&scope) && is_elevated(value))
            }) as Admits,
        ),
        (
            ScopeRule::UserLocalManagedFlag,
            in_scopes(vec![User, Local, Managed, Flag]),
        ),
    ])
});

/// The only keys a `--add-dir` directory's settings files carry (permissions.md, "Additional
/// directories grant file access, not configuration"): every other key is not loaded from there.
fn only_keys(scope: PolicyScope) -> Option<&'static [&'static str]> {
    (scope == PolicyScope::AddDir).then_some(&["enabledPlugins", "extraKnownMarketplaces"][..])
}

/// Whether a scope carries any value of `key` at all, whatever the value: a `--add-dir` directory
/// carries two keys (permissions.md), and a global config key is carried by no settings file, only
/// by `~/.claude.json` (settings.md, "Change a setting for one session": `--settings` "can't set
/// `Managed` or `Global config` keys").
#[must_use]
pub fn scope_carries(scope: PolicyScope, key: &str) -> bool {
    !GLOBAL_CONFIG_KEYS.contains(&key) && only_keys(scope).is_none_or(|keys| keys.contains(&key))
}

/// Whether `key` read from `scope` with `value` is taken into account. A key with no row is not:
/// the package's `SPECS[key]` would throw, and every caller passes a key of the table.
#[must_use]
pub fn admits(key: &str, scope: PolicyScope, value: &JsValue) -> bool {
    scope_carries(scope, key)
        && SPECS
            .get(key)
            .and_then(|row| ADMITS.get(&row.scopes))
            .is_some_and(|rule| rule(scope, value))
}

/// "When `true`, only `allowedMcpServers` from managed settings are respected."
fn pinned_by_managed_mcp_only(managed: &[JsValue]) -> bool {
    managed
        .iter()
        .any(|settings| settings.get("allowManagedMcpServersOnly") == Some(&JsValue::Bool(true)))
}

/// The table as the TypeScript writes it: `key: scopes, merge, repair`, then the row's other
/// members (`exception`, `admin`), each as written there.
macro_rules! key_specs {
    ($($key:literal: $scopes:ident, $merge:ident, $repair:ident $(, $field:ident: $value:expr)*;)*) => {
        IndexMap::from([$((
            $key,
            KeySpec {
                spec: ValueSpec {
                    $($field: $value,)*
                    ..spec(ScopeRule::$scopes, MERGES.$merge)
                },
                repair: RepairKind::$repair,
            },
        ),)*])
    };
}

/// One row per policy key, in the TypeScript's order.
pub static KEY_SPECS: LazyLock<IndexMap<&'static str, KeySpec>> = LazyLock::new(|| {
    key_specs! {
        // A `true` from any admin source that sets these applies (spec/judgments.md).
        "allowClaudeInChromeWithManagedMcp": Managed, any_true, Boolean;
        "allowAllClaudeAiMcps": Managed, any_true, Boolean;
        "allowManagedHooksOnly": Managed, highest_wins, LockBoolean;
        "allowManagedMcpServersOnly": Managed, highest_wins, LockBoolean;
        "allowManagedPermissionRulesOnly": Managed, highest_wins, LockBoolean;
        "allowedChannelPlugins": Managed, highest_wins, ChannelAllowlist;
        "allowedHttpHookUrls": AnyAndEmptyListFromParent, union_arrays, ClosedStringList;
        "allowedMcpServers": AnyAndParent, union_arrays, AllowedMcpServers, admin: Some(AdminTier {
            blocks_parent: Some(true),
            // "When `true`, only `allowedMcpServers` from managed settings are respected."
            pinned_when: Some(Box::new(pinned_by_managed_mcp_only)),
            owns: None,
        });
        "allowedProviders": ManagedAndParent, highest_wins, ProviderAllowlist;
        "appendPlugins": UserOrManaged, highest_wins, StringList;
        "availableModelsMatch": Managed, highest_wins, MatchMode;
        "autoMode": UserManagedFlag, auto_mode_blocks, AutoMode;
        "blockedMarketplaces": ManagedAndParent, union_arrays, BlockedSources;
        "channelsEnabled": Managed, highest_wins, Boolean;
        "deniedMcpServers": AnyAndParent, union_arrays, DeniedMcpServers;
        "deniedModels": Managed, highest_wins, ModelDenylist;
        "disableAllHooks": AnyAndParent, highest_wins, HooksSwitch;
        "disableAutoMode": Any, highest_wins, DisableMode;
        "disableBundledSkills": Any, highest_wins, LockBoolean;
        // "Scope: Any file ... A `true` in any settings file applies"; a boolean lock key fails closed in the
        // policy tier (managed-settings.md), which `lockBoolean` does. The MCP load settings read every holder.
        "disableClaudeAiConnectors": Any, highest_wins, LockBoolean,
            exception: Some(&TRUE_FROM_ANY_SCOPE);
        "disableCommandPluginSources": Managed, highest_wins, LockBoolean;
        "disableSideloadFlags": Managed, highest_wins, LockBoolean;
        "disableSkillShellExecution": AnyAndParent, highest_wins, LockBoolean;
        "disableWorkflows": Any, highest_wins, LockBoolean;
        // A rejection "in any settings file applies" (settings-reference.md); a bad entry is dropped, which
        // fails open for that one name, as every non-restrictive list does (managed-settings.md).
        "disabledMcpjsonServers": Any, union_arrays, StringList;
        "enableAllProjectMcpServers": Any, highest_wins, Boolean;
        "enableWorkflows": Any, highest_wins, Boolean;
        "enabledMcpjsonServers": Any, union_arrays, StringList;
        "enabledPlugins": Any, per_key_highest_wins, EnabledPlugins;
        "extraKnownMarketplaces": Any, per_key_highest_wins, MarketplaceMap;
        "forceLoginOrgUUID": AnyAndParent, highest_wins, OrgUuid;
        "forceRemoteSettingsRefresh": Managed, highest_wins, LockBoolean;
        "gatewayInternalNetworks": Managed, highest_wins, GatewayNetworks;
        "hooks": Any, hook_groups, Hooks;
        "httpHookAllowedEnvVars": AnyAndEmptyListFromParent, union_arrays, ClosedStringList;
        "managedMcpServers": Managed, per_key_highest_wins, Record;
        "managedSourcesBehavior": Managed, highest_wins, AsWritten;
        "parentSettingsBehavior": Managed, highest_wins, AsWritten;
        "permissions": AnyAndParent, permission_blocks, Permissions;
        "pluginConfigs": UserOrManaged, per_key_highest_wins, Record;
        "pluginSuggestionMarketplaces": Managed, highest_wins, StringList;
        "pluginTrustMessage": Managed, highest_wins, String;
        "policyHelper": Managed, highest_wins, AsWritten;
        "prependPlugins": UserOrManaged, highest_wins, StringList;
        "skillOverrides": Any, per_key_highest_wins, SkillOverrides;
        "skipAutoPermissionPrompt": UserOrManaged, any_true, ConsentBoolean;
        "skipDangerousModePermissionPrompt": UserLocalManagedFlag, any_true, ConsentBoolean;
        "strictKnownMarketplaces": ManagedAndParent, highest_wins, StrictSources;
        "strictPluginOnlyCustomization": ManagedAndParent, highest_wins, PluginOnly;
        "syncClaudeAiPlugins": UserLocalManagedFlag, highest_wins, Boolean,
            exception: Some(&FALSE_FROM_ANY_SCOPE);
        "syncClaudeAiSkills": UserLocalManagedFlag, highest_wins, Boolean,
            exception: Some(&FALSE_FROM_ANY_SCOPE);
        "useAutoModeDuringPlan": UserLocalManagedFlag, highest_wins, ConsentBoolean,
            exception: Some(&FALSE_FROM_ANY_SCOPE);
        "workflowKeywordTriggerEnabled": Any, highest_wins, Boolean;
        "workflowSizeGuideline": Any, highest_wins, WorkflowSize;
        "wslInheritsWindowsSettings": Managed, highest_wins, AsWritten;
    }
});

/// Every key the layers combine by name, with the row that says how.
pub static SPECS: LazyLock<IndexMap<&'static str, &'static ValueSpec>> = LazyLock::new(|| {
    let policy = KEY_SPECS.iter().map(|(key, row)| (*key, &row.spec));
    let general = GENERAL_SPECS.iter().map(|(key, row)| (*key, row));
    policy.chain(general).collect()
});

/// A row as the package holds it: its data, without the functions the core calls (`combine`,
/// `pinnedWhen`, `read`), which stay in the core.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpecRow {
    scopes: ScopeRule,
    merge: MergeRow,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair: Option<RepairKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exception: Option<&'static ManagedException>,
    #[serde(skip_serializing_if = "Option::is_none")]
    admin: Option<AdminRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'static [&'static str]>,
}

#[derive(Serialize)]
struct MergeRow {
    kind: MergeKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    blocks_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owns: Option<bool>,
}

fn row_of(spec: &'static ValueSpec, repair: Option<RepairKind>) -> SpecRow {
    SpecRow {
        scopes: spec.scopes,
        merge: MergeRow {
            kind: spec.merge.kind,
        },
        repair,
        exception: spec.exception.map(|rule| &**rule),
        admin: spec.admin.as_ref().map(|admin| AdminRow {
            blocks_parent: admin.blocks_parent,
            owns: admin.owns,
        }),
        path: spec.path,
    }
}

fn key_spec_rows() -> IndexMap<&'static str, SpecRow> {
    KEY_SPECS
        .iter()
        .map(|(key, row)| (*key, row_of(&row.spec, Some(row.repair))))
        .collect()
}

fn spec_rows() -> IndexMap<&'static str, SpecRow> {
    let general = GENERAL_SPECS
        .iter()
        .map(|(key, row)| (*key, row_of(row, None)));
    key_spec_rows().into_iter().chain(general).collect()
}

crate::registry::exports! {
    "settings/key-spec" {
        "admits" => ok |key: String, scope: PolicyScope, value: JsValue| admits(&key, scope, &value);
        "KEY_SPECS" => ok || key_spec_rows().into_iter().collect::<IndexMap<_, _>>();
        "SPECS" => ok || spec_rows().into_iter().collect::<IndexMap<_, _>>();
    }
}
