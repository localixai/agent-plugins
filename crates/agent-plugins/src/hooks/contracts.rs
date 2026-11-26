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

/// What the host knows about the running session that the lock gates read (`HookSession`).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSession {
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
