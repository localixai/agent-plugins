//! Port of `src/protocol/settings-keys.ts`, switched.
//!
//! The settings keys the kit resolves as policy, and the value lists of the settings it reads. In
//! the TypeScript each name must be a key the official `Settings` declares, which `satisfies`
//! checks at compile time; the wrapper keeps those checks.

use indexmap::IndexMap;

/// The top-level settings keys the kit resolves as policy: every key of the settings index
/// (settings-reference.md, "Settings index") that plugins, hooks, skills, workflows and channels
/// depend on, and the managed-only and fail-closed keys of managed-settings.md ("Keys only a
/// managed source can set", "Keys that fail closed"), in alphabetical order. The four `strictPluginOnlyCustomization.*` ids are surfaces
/// of the one key, not keys.
pub const POLICY_KEYS: &[&str] = &[
    "allowAllClaudeAiMcps",
    "allowClaudeInChromeWithManagedMcp",
    "allowManagedHooksOnly",
    "allowManagedMcpServersOnly",
    "allowManagedPermissionRulesOnly",
    "allowedChannelPlugins",
    "allowedHttpHookUrls",
    "allowedMcpServers",
    "allowedProviders",
    "appendPlugins",
    "availableModelsMatch",
    "autoMode",
    "blockedMarketplaces",
    "channelsEnabled",
    "deniedMcpServers",
    "deniedModels",
    "disableAllHooks",
    "disableAutoMode",
    "disableBundledSkills",
    "disableClaudeAiConnectors",
    "disableCommandPluginSources",
    "disableSideloadFlags",
    "disableSkillShellExecution",
    "disableWorkflows",
    "disabledMcpjsonServers",
    "enableAllProjectMcpServers",
    "enableWorkflows",
    "enabledMcpjsonServers",
    "enabledPlugins",
    "extraKnownMarketplaces",
    "forceLoginOrgUUID",
    "forceRemoteSettingsRefresh",
    "gatewayInternalNetworks",
    "hooks",
    "httpHookAllowedEnvVars",
    "managedMcpServers",
    "managedSourcesBehavior",
    "parentSettingsBehavior",
    "permissions",
    "pluginConfigs",
    "pluginSuggestionMarketplaces",
    "pluginTrustMessage",
    "policyHelper",
    "prependPlugins",
    "skillOverrides",
    "skipAutoPermissionPrompt",
    "skipDangerousModePermissionPrompt",
    "strictKnownMarketplaces",
    "strictPluginOnlyCustomization",
    "syncClaudeAiPlugins",
    "syncClaudeAiSkills",
    "useAutoModeDuringPlan",
    "workflowKeywordTriggerEnabled",
    "workflowSizeGuideline",
    "wslInheritsWindowsSettings",
];

/// The kinds of customization `strictPluginOnlyCustomization` can lock, in the order the docs list
/// them ("`skills`, `agents`, `hooks`, and `mcp`").
pub const PLUGIN_ONLY_SURFACES: &[&str] = &["skills", "agents", "hooks", "mcp"];

/// The states a `skillOverrides` entry takes (skills.md, "Override skill visibility from settings").
pub const SKILL_OVERRIDE_STATES: &[&str] = &["on", "name-only", "user-invocable-only", "off"];

/// The values of `workflowSizeGuideline` (settings-reference.md, `workflowSizeGuideline`).
pub const WORKFLOW_SIZES: &[&str] = &["unrestricted", "small", "medium", "large"];

/// Every `source` kind of a marketplace source object. The docs' "Allowed source types" table names
/// eight; the SDK also types `settings` and `npm` (spec/judgments.md).
pub const MARKETPLACE_SOURCE_KINDS: &[&str] = &[
    "directory",
    "file",
    "git",
    "github",
    "hostPattern",
    "npm",
    "pathPattern",
    "settings",
    "skills-dir",
    "url",
];

/// Keys Claude Code accepts under another spelling (settings-reference.md, "Marketplace key
/// aliases"): the alias on the left reads exactly as the canonical key on the right.
#[must_use]
pub fn marketplace_key_aliases() -> IndexMap<&'static str, &'static str> {
    IndexMap::from([
        ("allowedMarketplaces", "strictKnownMarketplaces"),
        ("additionalMarketplaces", "extraKnownMarketplaces"),
    ])
}

/// The services `allowedProviders` names (settings-reference.md, `allowedProviders`).
pub const API_PROVIDERS: &[&str] = &[
    "anthropic",
    "customEndpoint",
    "bedrock",
    "vertex",
    "foundry",
    "anthropicAws",
    "mantle",
    "gateway",
];

/// The values of `availableModelsMatch` (settings-reference.md, `availableModelsMatch`).
pub const MODEL_MATCH_MODES: &[&str] = &["prefix", "exact"];

/// The values of `crossSessionInbound` from the least to the most strict (settings-reference.md, `crossSessionInbound`).
pub const CROSS_SESSION_MODES: &[&str] = &["accept", "hold", "refuse"];

crate::registry::exports! {
    "protocol/settings-keys" {
        "POLICY_KEYS" => ok || POLICY_KEYS;
        "PLUGIN_ONLY_SURFACES" => ok || PLUGIN_ONLY_SURFACES;
        "SKILL_OVERRIDE_STATES" => ok || SKILL_OVERRIDE_STATES;
        "WORKFLOW_SIZES" => ok || WORKFLOW_SIZES;
        "MARKETPLACE_SOURCE_KINDS" => ok || MARKETPLACE_SOURCE_KINDS;
        "MARKETPLACE_KEY_ALIASES" => ok marketplace_key_aliases;
        "API_PROVIDERS" => ok || API_PROVIDERS;
        "MODEL_MATCH_MODES" => ok || MODEL_MATCH_MODES;
        "CROSS_SESSION_MODES" => ok || CROSS_SESSION_MODES;
    }
}
