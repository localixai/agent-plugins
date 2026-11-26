//! The core's counterpart of `src/hooks/contracts.ts` at sha256:6224da49bc98, which stays TypeScript:
//! the shapes the hook engine's stages share, only types. This file holds the fields the ported
//! modules read; a field no ported module reads is left out (a struct reads what it names and
//! ignores the rest).

use serde::Deserialize;

use crate::registry::Set;

/// Where a registered handler was declared (hooks.md, "Hook locations").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookSourceKind {
    /// A settings file.
    Settings,
    /// A plugin's hooks file.
    Plugin,
    /// A skill's frontmatter.
    Skill,
    /// An agent's frontmatter.
    Agent,
    /// A session registered it.
    Session,
}

/// Where a registered handler was declared.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSource {
    /// The kind of declaration site.
    pub kind: HookSourceKind,
    /// What an ask prompt and the `/hooks` list call it: `[settings]`, `[plugin:<name>]`, `[skill]`.
    #[serde(default)]
    pub label: String,
    /// The plugin's manifest name.
    #[serde(default)]
    pub plugin_name: Option<String>,
    /// Which settings file, skill folder or agent folder it came from (official).
    #[serde(default)]
    pub scope: Option<String>,
    /// Declared in managed policy settings (hooks.md, "Disable or remove hooks").
    #[serde(default)]
    pub managed: Option<bool>,
}

/// A handler a registry holds: where it came from, and its plugin's full id.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredHandler {
    /// Where it was declared.
    pub source: HookSource,
    /// The `plugin@marketplace` id of the plugin it came from.
    #[serde(default)]
    pub plugin_id: Option<String>,
}

/// `disableAllHooks` as the session carries it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableAllHooks {
    /// The `disableAllHooks` left after settings precedence, which turns off every hook but the
    /// managed ones.
    #[serde(default)]
    pub effective: bool,
    /// `disableAllHooks` set at the managed level, which turns off those too.
    #[serde(default)]
    pub managed_locked: bool,
}

/// What the host knows about the running session that the lock gates read (`HookSession`).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSession {
    /// Settings-file hooks may run: an interactive session after the trust dialog, a `-p` or SDK
    /// session always (hooks.md, "Workspace trust").
    #[serde(default)]
    pub trusted: bool,
    /// The trust dialog was accepted for the exact folder (hooks.md, "Hooks in skills and
    /// agents").
    #[serde(default)]
    pub folder_trust_accepted: Option<bool>,
    /// `disableAllHooks` left after settings precedence, and set at the managed level (hooks.md,
    /// "Disable or remove hooks").
    #[serde(default)]
    pub disable_all_hooks: DisableAllHooks,
    /// The plugins that are enabled; absent means the registry holds only enabled plugins' hooks.
    #[serde(default)]
    pub enabled_plugins: Option<Set<String>>,
    /// `allowManagedHooksOnly` is set in managed settings.
    #[serde(default)]
    pub allow_managed_hooks_only: Option<bool>,
    /// The full `plugin@marketplace` ids force-enabled in managed `enabledPlugins`.
    #[serde(default)]
    pub force_enabled_plugins: Option<Set<String>>,
    /// `strictPluginOnlyCustomization` locks the `hooks` surface.
    #[serde(default)]
    pub plugin_only_hooks: Option<bool>,
}
