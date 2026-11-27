//! Ported from `src/settings/general/global-config.ts`; only the core uses it now, so it has no wrapper.
//!
//! The keys whose Scope is `Global config` in the settings index (settings-reference.md,
//! "Settings index"): they "apply only from `~/.claude.json`", so a settings file, `--settings`
//! included, cannot set them. The index of Claude Code 2.1.319 has these twelve;
//! `permissionExplainerEnabled` is kept because the index still lists it as removed rather than
//! unknown.

/// The keys of the global config.
pub const GLOBAL_CONFIG_KEYS: [&str; 12] = [
    "autoConnectIde",
    "autoInstallIdeExtension",
    "claudeInChromeDefaultEnabled",
    "copyFullResponse",
    "copyOnSelect",
    "defaultToAgentsView",
    "diffTool",
    "externalEditorContext",
    "leftArrowOpensAgents",
    "permissionExplainerEnabled",
    "prStatusFooterEnabled",
    "teammateDefaultModel",
];
