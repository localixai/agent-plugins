//! The core's counterpart of `src/rules/decide/context.ts` at sha256:cbff0800c37b, which stays TypeScript: it declares types only, so it has nothing to run (docs/RUST.md, "What stays TypeScript").
//!
//! What `decide` reads beside the call. Of its types the ported modules read only the SDK's two it
//! imports, declared here: the session's permission mode and a hook's permission decision.

use serde::{Deserialize, Serialize};

/// The SDK's `PermissionMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// `default`.
    Default,
    /// `acceptEdits`.
    AcceptEdits,
    /// `bypassPermissions`.
    BypassPermissions,
    /// `plan`.
    Plan,
    /// `dontAsk`.
    DontAsk,
    /// `auto`.
    Auto,
}

/// The SDK's `HookPermissionDecision`: a PreToolUse hook's `permissionDecision`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookPermissionDecision {
    /// `allow`.
    Allow,
    /// `deny`.
    Deny,
    /// `ask`.
    Ask,
    /// `defer`.
    Defer,
}

/// The paths beside the protected lists that a session protects (`ProtectedPathOptions`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedPathOptions {
    /// Files passed to `--settings`, `--mcp-config` or `--agents`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// The files that a symlinked settings file points to. Unlike `files`, a write to one prompts
    /// in the modes that route protected-path writes to the classifier, "even when an allow rule
    /// matches".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symlink_targets: Option<Vec<String>>,
    /// The directory `CLAUDE_CONFIG_DIR` points to, in place of `~/.claude` (default
    /// `~/.claude`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_directory: Option<String>,
    /// Directories loaded with `--plugin-dir`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// The `plansDirectory` setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plans_directory: Option<String>,
    /// The signature, sync and consent files the host keeps beside `remote-settings.json`, by
    /// path. The docs name them by role only, so the kit protects these and guesses no name (Agent
    /// Plugins Extended J13.35).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_settings_files: Option<Vec<String>>,
    /// A session started with `--restricted`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restricted: Option<bool>,
}
