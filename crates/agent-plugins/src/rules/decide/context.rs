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
