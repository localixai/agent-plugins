//! Port of `src/rules/decide/hook-permission-updates.ts`, switched.
//!
//! The permission update entries of a hook's answer as they take effect in this session (hooks.md,
//! "Permission update entries"): "`setMode` with `bypassPermissions` only takes effect if you
//! launched the session with bypass mode already available ... Otherwise the update is a no-op. The
//! update is also a no-op when `permissions.disableBypassPermissionsMode` disables the mode, or when
//! the session starts in restricted mode", and "`bypassPermissions` is never persisted as
//! `defaultMode` regardless of `destination`". `bypassAvailable` already folds in the launch, the
//! setting and `--restricted` (`startPermissions`). An entry that takes effect only in memory keeps
//! its place with `destination: "session"`.

use serde::{Deserialize, Serialize};

use super::context::PermissionMode;
use crate::rules::types::{PermissionBehavior, PermissionRuleValue};

/// The SDK's `PermissionUpdateDestination`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// `userSettings`.
    UserSettings,
    /// `projectSettings`.
    ProjectSettings,
    /// `localSettings`.
    LocalSettings,
    /// `session`.
    Session,
    /// `cliArg`.
    CliArg,
}

/// The SDK's `PermissionUpdate`, one entry of a hook's `updatedPermissions`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[expect(missing_docs, reason = "each variant and member is the SDK's own")]
pub enum PermissionUpdate {
    AddRules {
        rules: Vec<PermissionRuleValue>,
        behavior: PermissionBehavior,
        destination: PermissionUpdateDestination,
    },
    ReplaceRules {
        rules: Vec<PermissionRuleValue>,
        behavior: PermissionBehavior,
        destination: PermissionUpdateDestination,
    },
    RemoveRules {
        rules: Vec<PermissionRuleValue>,
        behavior: PermissionBehavior,
        destination: PermissionUpdateDestination,
    },
    SetMode {
        mode: PermissionMode,
        destination: PermissionUpdateDestination,
    },
    AddDirectories {
        directories: Vec<String>,
        destination: PermissionUpdateDestination,
    },
    RemoveDirectories {
        directories: Vec<String>,
        destination: PermissionUpdateDestination,
    },
}

/// `Pick<DecideContext, 'bypassAvailable'>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Bypass mode is available to this session.
    pub bypass_available: bool,
}

/// The modes the bypass rule is about.
const fn is_bypass(mode: PermissionMode) -> bool {
    match mode {
        PermissionMode::BypassPermissions => true,
        PermissionMode::Default
        | PermissionMode::AcceptEdits
        | PermissionMode::Plan
        | PermissionMode::DontAsk
        | PermissionMode::Auto => false,
    }
}

fn set_mode(
    mode: PermissionMode,
    destination: PermissionUpdateDestination,
    session: Session,
) -> Option<PermissionUpdate> {
    if !is_bypass(mode) {
        return Some(PermissionUpdate::SetMode { mode, destination });
    }
    session
        .bypass_available
        .then_some(PermissionUpdate::SetMode {
            mode,
            destination: PermissionUpdateDestination::Session,
        })
}

/// `IN_EFFECT[update.type](update, session)`: every entry but `setMode` is `unchanged`.
fn in_effect(update: PermissionUpdate, session: Session) -> Option<PermissionUpdate> {
    match update {
        PermissionUpdate::SetMode { mode, destination } => set_mode(mode, destination, session),
        PermissionUpdate::AddRules { .. }
        | PermissionUpdate::ReplaceRules { .. }
        | PermissionUpdate::RemoveRules { .. }
        | PermissionUpdate::AddDirectories { .. }
        | PermissionUpdate::RemoveDirectories { .. } => Some(update),
    }
}

/// The entries to apply, in order: a `setMode` to bypass the session cannot take is left out.
#[must_use]
pub fn permission_updates_in_effect(
    updates: Vec<PermissionUpdate>,
    session: Session,
) -> Vec<PermissionUpdate> {
    updates
        .into_iter()
        .filter_map(|update| in_effect(update, session))
        .collect()
}

crate::registry::exports! {
    "rules/decide/hook-permission-updates" {
        "permissionUpdatesInEffect" => ok |updates: Vec<PermissionUpdate>, session: Session| permission_updates_in_effect(updates, session);
    }
}
