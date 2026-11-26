//! Port of `src/updates/private-marketplace.ts`, switched.
//!
//! Background auto-update of a private marketplace (plugins/host-marketplace.md, "What background
//! auto-update does with credentials"; plugins/install.md, "Auto-update from a private
//! marketplace"). The check "uses the user's configured git credential helpers and never prompts".
//! Pure: the host runs git; the kit says whether the check can authenticate and what follows it.

use serde::{Deserialize, Serialize};

use super::env_flags::is_on;
use crate::ports::environment::Environment;
use crate::registry::Port;

/// What the HTTPS credential helper does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Helper {
    /// Git Credential Manager, the macOS Keychain helper or `git-credential-store` holding a credential.
    #[serde(rename = "supplies-stored-credential")]
    SuppliesStoredCredential,
    /// Cannot answer unattended.
    #[serde(rename = "must-prompt")]
    MustPrompt,
    /// No helper.
    #[serde(rename = "none")]
    None,
}

/// What authenticates a remote in the background (`GITHUB_TOKEN`, `GH_TOKEN` or another provider
/// token in the environment never counts toward authenticating, so it is not read).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(
    tag = "remote",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum Credentials {
    /// An SSH remote.
    Ssh {
        /// A key is in the agent.
        key_in_agent: bool,
    },
    /// An HTTPS remote.
    Https {
        /// What the credential helper does.
        helper: Helper,
    },
}

/// Whether the unattended check authenticates. "If a user sets `GITHUB_TOKEN` or another provider
/// token in the environment, that alone doesn't authenticate the background check": the function
/// reads only the key in the agent and the helper, so a token in the environment cannot count. An SSH remote with no
/// key in the agent and an HTTPS remote with no helper are judgments (spec/judgments.md).
#[must_use]
pub const fn background_check_authenticates(credentials: &Credentials) -> bool {
    match credentials {
        Credentials::Ssh { key_in_agent } => *key_in_agent,
        Credentials::Https { helper } => matches!(helper, Helper::SuppliesStoredCredential),
    }
}

/// What the host sees of the remote when the check runs.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteState {
    /// The remote answers at all.
    pub reachable: bool,
    /// Its default branch has commits the checkout lacks; only meaningful once the check authenticated.
    pub has_new_commits: bool,
}

/// What the check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckResult {
    /// Nothing new.
    #[serde(rename = "up-to-date")]
    UpToDate,
    /// The remote has new commits.
    #[serde(rename = "new-commits")]
    NewCommits,
    /// The check could not reach or authenticate.
    #[serde(rename = "cannot-reach-or-authenticate")]
    CannotReachOrAuthenticate,
}

/// What follows the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CheckFollowUp {
    /// Leave the checkout.
    #[serde(rename = "leave")]
    Leave,
    /// Clone again.
    #[serde(rename = "reclone")]
    Reclone,
    /// Keep the existing checkout.
    #[serde(rename = "keep-existing-checkout")]
    KeepExistingCheckout,
}

/// What the unattended check finds: a remote it cannot reach, or cannot authenticate to (a helper
/// that must prompt "fails quietly"), is `cannot-reach-or-authenticate`; otherwise whether the remote
/// has new commits.
#[must_use]
pub const fn background_check_result(
    credentials: &Credentials,
    remote: &RemoteState,
) -> CheckResult {
    if !remote.reachable || !background_check_authenticates(credentials) {
        return CheckResult::CannotReachOrAuthenticate;
    }
    if remote.has_new_commits {
        CheckResult::NewCommits
    } else {
        CheckResult::UpToDate
    }
}

const KEEP_ON_FAILURE: &str = "CLAUDE_CODE_PLUGIN_KEEP_MARKETPLACE_ON_FAILURE";

/// "After the check, Claude Code does one of the following." `checkoutHoldsCatalog` is whether the
/// existing checkout contains `.claude-plugin/marketplace.json`: the variable "skips the re-clone
/// only for a checkout that already contains" it, so a marketplace never cloned, or whose clone
/// stopped partway, still gets the clone attempt (plugins/troubleshooting.md, "Marketplace updates
/// keep failing offline").
pub async fn after_background_check(
    result: CheckResult,
    env: &dyn Environment,
    checkout_holds_catalog: bool,
) -> CheckFollowUp {
    match result {
        CheckResult::UpToDate => CheckFollowUp::Leave,
        CheckResult::NewCommits => CheckFollowUp::Reclone,
        CheckResult::CannotReachOrAuthenticate => {
            if is_on(env, KEEP_ON_FAILURE).await && checkout_holds_catalog {
                CheckFollowUp::KeepExistingCheckout
            } else {
                CheckFollowUp::Reclone
            }
        }
    }
}

/// What follows a clone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RecloneFollowUp {
    /// The new clone takes the place.
    #[serde(rename = "replace-checkout")]
    ReplaceCheckout,
    /// "If that clone fails, the existing checkout stays in place."
    #[serde(rename = "keep-existing-checkout")]
    KeepExistingCheckout,
}

/// "If that clone fails, the existing checkout stays in place."
#[must_use]
pub const fn after_reclone(clone_succeeded: bool) -> RecloneFollowUp {
    if clone_succeeded {
        RecloneFollowUp::ReplaceCheckout
    } else {
        RecloneFollowUp::KeepExistingCheckout
    }
}

crate::registry::exports! {
    "updates/private-marketplace" {
        "backgroundCheckAuthenticates" => ok |credentials: Credentials|
            background_check_authenticates(&credentials);
        "backgroundCheckResult" => ok |credentials: Credentials, remote: RemoteState|
            background_check_result(&credentials, &remote);
        "afterBackgroundCheck" => async ok |result: CheckResult, env: Port, checkout_holds_catalog: bool|
            after_background_check(result, &env, checkout_holds_catalog).await;
        "afterReclone" => ok |clone_succeeded: bool| after_reclone(clone_succeeded);
    }
}
