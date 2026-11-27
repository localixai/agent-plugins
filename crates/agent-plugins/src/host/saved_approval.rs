//! Port of `src/host/saved-approval.ts`, switched.
//!
//! What "Yes, and don't ask again" does for one tool in a session, and where it is saved.

use serde::{Deserialize, Serialize};

use crate::registry::Port;
use crate::rules::decide::approval_memory::{
    ApprovalLifetime, ApprovalMemory, ApprovalOptions, approval_memory,
};
use crate::settings::locations::{SettingsLocationsInput, settings_locations};

/// What "Yes, and don't ask again" does for one tool in a session, and where it is saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedApproval {
    /// What the approval remembers.
    pub memory: ApprovalMemory,
    /// The file the rule is saved to, for an approval that lasts permanently: "`.claude/settings.local.json`
    /// at the root of the git repository, resolved through worktrees to the main checkout", or the
    /// working directory's where Claude Code does not use the root (permissions.md, "Permission system").
    /// Absent when nothing is saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_file: Option<String>,
}

/// `Pick<PermissionsPolicy, 'managedRulesOnly'>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesPolicy {
    /// `allowManagedPermissionRulesOnly` is on.
    pub managed_rules_only: bool,
}

/// The effect of "Yes, and don't ask again" for `tool_name` under the policy (`allowManagedPermissionRulesOnly`
/// stops saving new rules) and the host's paths (its record, read only when a rule is saved); `undefined` for a tool the table does not list.
pub async fn saved_approval(
    tool_name: &str,
    policy: RulesPolicy,
    locations: Port,
) -> Option<SavedApproval> {
    let memory = approval_memory(
        tool_name,
        ApprovalOptions {
            rules_locked: Some(policy.managed_rules_only),
        },
    )?;
    Some(if memory.lifetime == ApprovalLifetime::Repository {
        SavedApproval {
            memory,
            rule_file: Some(
                settings_locations(&SettingsLocationsInput::read(locations).await)
                    .await
                    .local,
            ),
        }
    } else {
        SavedApproval {
            memory,
            rule_file: None,
        }
    })
}

crate::registry::exports! {
    "host/saved-approval" {
        "savedApproval" => async ok |tool_name: String, policy: RulesPolicy, locations: Port|
            saved_approval(&tool_name, policy, locations).await;
    }
}
