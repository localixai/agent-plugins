//! Port of `src/settings/managed/startup-failure.ts`, switched.
//!
//! What a failed startup fetch of server-managed settings does (server-managed-settings.md, "Enforce
//! fail-closed startup"; errors.md, "Cloud gateway session expired"). Pure over the facts the host
//! knows; the host prints the line, exits or opens the session.

use serde::{Deserialize, Serialize};

/// The line a gateway session that answered `401` prints before it opens signed out.
pub const GATEWAY_SESSION_EXPIRED: &str =
    "Cloud gateway session expired — run /login to reconnect.";

/// `StartupFetchFacts` of `applicability.ts`, with what a failure adds.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "it mirrors the interface of the package, whose members are independent facts"
)]
pub struct StartupFailureFacts {
    /// `forceRemoteSettingsRefresh` is set.
    pub force_remote_settings_refresh: bool,
    /// The session fetches server-managed settings at all.
    pub fetches_remote: bool,
    /// The developer signs in at startup (first run, after `/logout`).
    pub signs_in_at_startup: bool,
    /// Anthropic requires the organization's settings before a session starts.
    pub anthropic_requires: bool,
    /// A Cloud gateway session.
    pub gateway_session: bool,
    /// A `claude auth` subcommand.
    pub auth_subcommand: bool,
    /// An attended interactive launch: `claude` in a terminal, not `-p`, the SDK or a background run.
    pub attended_interactive: bool,
    /// The HTTP status the gateway answered with; absent when it could not be reached.
    #[serde(default)]
    pub gateway_status: Option<f64>,
}

/// What the failed fetch does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome")]
pub enum StartupFailure {
    /// The session starts anyway, on a cached or absent policy.
    #[serde(rename = "starts")]
    Starts,
    /// The session exits with an error.
    #[serde(rename = "exits")]
    Exits,
    /// The host prints `message` and opens the session signed out of the gateway until `/login`.
    #[serde(rename = "opens-signed-out")]
    OpensSignedOut {
        /// The line to print.
        message: &'static str,
    },
}

const UNAUTHORIZED: f64 = 401.0;

/// "If the gateway answers an attended interactive launch with a `401` and this setting is off, the
/// gateway has ended that sign-in": the line, then a session signed out of the gateway. "When the fetch
/// fails in any other way, or in any other kind of launch except a `claude auth` subcommand, the client
/// exits with an error." Outside a gateway, `forceRemoteSettingsRefresh` or Anthropic's requirement
/// exits and anything else starts.
#[must_use]
pub fn startup_fetch_failure(facts: &StartupFailureFacts) -> StartupFailure {
    if facts.auth_subcommand || !facts.fetches_remote {
        return StartupFailure::Starts;
    }
    let expired = facts.gateway_session
        && facts.attended_interactive
        && !facts.force_remote_settings_refresh
        && facts.gateway_status == Some(UNAUTHORIZED);
    if expired {
        return StartupFailure::OpensSignedOut {
            message: GATEWAY_SESSION_EXPIRED,
        };
    }
    let exits =
        facts.gateway_session || facts.force_remote_settings_refresh || facts.anthropic_requires;
    if exits {
        StartupFailure::Exits
    } else {
        StartupFailure::Starts
    }
}

crate::registry::exports! {
    "settings/managed/startup-failure" {
        "GATEWAY_SESSION_EXPIRED" => ok || GATEWAY_SESSION_EXPIRED;
        "startupFetchFailure" => ok |facts: StartupFailureFacts| startup_fetch_failure(&facts);
    }
}
