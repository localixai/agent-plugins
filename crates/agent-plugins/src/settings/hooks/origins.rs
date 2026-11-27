//! Port of `src/settings/hooks/origins.ts`, switched.
//!
//! Where a hook comes from, as a host reports it. What `allowManagedHooksOnly`, `disableAllHooks`
//! and the `hooks` surface of `strictPluginOnlyCustomization` stop is a property of the origin
//! (settings-reference.md, `disableAllHooks` "The reach depends on which file carries the key" and
//! "What runs under `allowManagedHooksOnly`"), so the host names the origin and the policy answers.

use serde::{Deserialize, Serialize};

/// A plugin that ships hooks, with the facts the locks look at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's PluginHookSource holds these four facts, each its own"
)]
pub struct PluginHookSource {
    /// `plugin@marketplace`, matched whole against the plugins managed settings force-enable.
    pub id: String,
    /// A folder the user keeps under `skills/`: user customization to
    /// `strictPluginOnlyCustomization`.
    pub skills_dir_plugin: bool,
    /// A plugin whose directory a marketplace-declared command produces.
    pub command_source: bool,
    /// A plugin whose code registers hooks.
    pub is_mod: bool,
    /// A mod that counts as the organization's (plugins/mods/admin.md, "Install your
    /// organization's mods").
    pub is_org_mod: bool,
}

/// Where `settings` hooks are read from: the `--settings` file and the SDK `settings` option count
/// as the user's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsHookScope {
    /// The user's file.
    User,
    /// The project's shared file.
    Project,
    /// The project's local file.
    Local,
    /// The `--settings` file.
    Flag,
    /// The SDK `settings` option.
    SdkSettingsOption,
}

/// What declares frontmatter hooks (hooks.md, "Hooks in skills and agents").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookFrontmatterOwner {
    /// A skill.
    Skill,
    /// A command.
    Command,
    /// An agent.
    Agent,
}

/// The sources a skill, command or agent can come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookFrontmatterSource {
    /// A personal file.
    User,
    /// A project file.
    Project,
    /// An added directory.
    AddDir,
    /// Synced from claude.ai.
    Synced,
    /// Deployed through managed settings.
    Managed,
    /// Bundled with Claude Code.
    Bundled,
}

/// Where a hook comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyHookSource {
    /// Managed settings.
    ManagedSettings,
    /// An SDK callback.
    SdkCallback,
    /// A settings file.
    Settings {
        /// Which one.
        scope: SettingsHookScope,
    },
    /// A plugin's hooks.
    Plugin(PluginHookSource),
    /// Frontmatter of a skill, command or agent that is not a plugin's.
    Frontmatter {
        /// What declares it.
        owner: HookFrontmatterOwner,
        /// Where the file comes from.
        from: HookFrontmatterSource,
    },
    /// Frontmatter of a plugin's skill, command or agent.
    PluginFrontmatter {
        /// What declares it.
        owner: HookFrontmatterOwner,
        /// The plugin.
        plugin: PluginHookSource,
    },
}

/// Why a hook does not run.
pub const HOOK_BLOCK_REASONS: [HookBlockReason; 4] = [
    HookBlockReason::CommandSourceDisabled,
    HookBlockReason::DisableAllHooks,
    HookBlockReason::AllowManagedHooksOnly,
    HookBlockReason::StrictPluginOnlyCustomizationHooks,
];

/// One of `HOOK_BLOCK_REASONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookBlockReason {
    /// `disableCommandPluginSources`.
    #[serde(rename = "commandSourceDisabled")]
    CommandSourceDisabled,
    /// `disableAllHooks`.
    #[serde(rename = "disableAllHooks")]
    DisableAllHooks,
    /// `allowManagedHooksOnly`.
    #[serde(rename = "allowManagedHooksOnly")]
    AllowManagedHooksOnly,
    /// The `hooks` surface of `strictPluginOnlyCustomization`.
    #[serde(rename = "strictPluginOnlyCustomization.hooks")]
    StrictPluginOnlyCustomizationHooks,
}

/// Whether a hook runs, and why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HookVerdict {
    /// Whether it runs.
    pub run: bool,
    /// Why it does not; absent when it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<HookBlockReason>,
}

/// Which of the locks stops a hook of one origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's Stops holds one flag per lock"
)]
pub struct Stops {
    /// `disableAllHooks` in managed settings.
    pub managed_disable: bool,
    /// `disableAllHooks` in any other file, or in the SDK's `managedSettings`.
    pub other_disable: bool,
    /// `allowManagedHooksOnly`.
    pub managed_only: bool,
    /// The `hooks` surface of `strictPluginOnlyCustomization`.
    pub plugin_only: bool,
    /// `disableCommandPluginSources`, or `allowManagedHooksOnly` standing in for it.
    pub command_source: bool,
}

crate::registry::exports! {
    "settings/hooks/origins" {
        "HOOK_BLOCK_REASONS" => ok || HOOK_BLOCK_REASONS;
    }
}
