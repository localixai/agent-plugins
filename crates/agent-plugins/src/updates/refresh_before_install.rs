//! Port of `src/updates/refresh-before-install.ts`, switched.
//!
//! When Claude Code reads a marketplace again before it looks a plugin up for an install
//! (plugins/loading.md, "When Claude Code refreshes a marketplace before an install"). Pure: the
//! host fetches; the kit says which marketplaces, which are skipped and why, and what a failure
//! leaves. Every fact comes in as data, the clock and the environment through their ports.

use serde::{Deserialize, Serialize};

use super::env_flags::{DISABLE_NONESSENTIAL_TRAFFIC, is_set};
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::marketplace::ids::parse_plugin_ref;
use crate::format::marketplace::types::MarketplaceSourceKind;
use crate::ports::clock::Clock;
use crate::ports::environment::Environment;
use crate::registry::Port;

/// "You can run `/plugin install` in a session or `claude plugin install` in your shell."
pub const INSTALL_COMMANDS: [InstallCommand; 2] = [
    InstallCommand::PluginInstall,
    InstallCommand::ClaudePluginInstall,
];

/// One of `INSTALL_COMMANDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallCommand {
    /// `/plugin install`.
    #[serde(rename = "/plugin install")]
    PluginInstall,
    /// `claude plugin install`.
    #[serde(rename = "claude plugin install")]
    ClaudePluginInstall,
}

/// "Claude Code refreshed the marketplace within the last 30 seconds."
const REFRESH_WINDOW_MS: f64 = 30_000.0;

/// What the kit needs to know about a marketplace the user has registered.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownMarketplace {
    /// Its name.
    pub name: String,
    /// How it was added: `file` and `directory` are local paths, `settings` is defined inline.
    pub source: MarketplaceSourceKind,
    /// Its resolved auto-update setting (`marketplaceAutoUpdate`).
    pub auto_update: bool,
    /// A seed directory supplies it (`CLAUDE_CODE_PLUGIN_SEED_DIR`).
    #[serde(default)]
    pub seeded: Option<bool>,
    /// When the host last refreshed it, in `Clock.now()` milliseconds.
    #[serde(default)]
    pub last_refreshed_at: Option<f64>,
    /// The marketplace gate's message when managed settings block it ("is blocked by enterprise policy").
    #[serde(default)]
    pub blocked: Option<String>,
}

/// What an install asks the refresh.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    /// `name` or `name@marketplace`, as the user typed it.
    #[serde(rename = "ref")]
    pub reference: String,
    /// The command.
    pub command: InstallCommand,
    /// The marketplaces the user registered.
    pub marketplaces: Vec<KnownMarketplace>,
    /// The lookup of a bare name in the cached catalogs found no plugin.
    pub lookup_missed: bool,
    /// The environment.
    pub env: Port,
    /// The clock.
    pub clock: Port,
}

/// Why nothing is refreshed.
pub const SKIP_REASONS: [SkipReason; 10] = [
    SkipReason::InvalidReference,
    SkipReason::MarketplaceUnknown,
    SkipReason::LocalOrInlineSource,
    SkipReason::SeedDirectory,
    SkipReason::RefreshedWithin30Seconds,
    SkipReason::NonessentialTrafficDisabled,
    SkipReason::CliReadsCachedCatalogs,
    SkipReason::LookupFoundThePlugin,
    SkipReason::NoAutoUpdateMarketplace,
    SkipReason::EveryCandidateSkipped,
];

/// One of `SKIP_REASONS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// The reference does not parse.
    #[serde(rename = "invalid-reference")]
    InvalidReference,
    /// No such marketplace.
    #[serde(rename = "marketplace-unknown")]
    MarketplaceUnknown,
    /// A local path or an inline definition.
    #[serde(rename = "local-or-inline-source")]
    LocalOrInlineSource,
    /// A seed directory supplies it.
    #[serde(rename = "seed-directory")]
    SeedDirectory,
    /// Refreshed within the last 30 seconds.
    #[serde(rename = "refreshed-within-30-seconds")]
    RefreshedWithin30Seconds,
    /// Nonessential traffic is disabled.
    #[serde(rename = "nonessential-traffic-disabled")]
    NonessentialTrafficDisabled,
    /// The CLI reads cached catalogs.
    #[serde(rename = "cli-reads-cached-catalogs")]
    CliReadsCachedCatalogs,
    /// The lookup found the plugin.
    #[serde(rename = "lookup-found-the-plugin")]
    LookupFoundThePlugin,
    /// No marketplace has auto-update on.
    #[serde(rename = "no-auto-update-marketplace")]
    NoAutoUpdateMarketplace,
    /// Every candidate is skipped.
    #[serde(rename = "every-candidate-skipped")]
    EveryCandidateSkipped,
}

/// What to refresh.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum RefreshDecision {
    /// Refresh these marketplaces.
    Refresh {
        /// Always `refresh`.
        outcome: &'static str,
        /// Their names.
        marketplaces: Vec<String>,
    },
    /// Refresh nothing.
    Skip {
        /// Always `skip`.
        outcome: &'static str,
        /// Why.
        reason: SkipReason,
    },
    /// "Managed settings block the marketplace, in which case Claude Code also refuses the install."
    Refuse {
        /// Always `refuse`.
        outcome: &'static str,
        /// Always `blocked-by-managed-settings`.
        reason: &'static str,
        /// The refusal.
        issue: PluginIssue,
    },
}

const fn skip(reason: SkipReason) -> RefreshDecision {
    RefreshDecision::Skip {
        outcome: "skip",
        reason,
    }
}

/// Local paths and inline definitions are read where they are, so a refresh has nothing to fetch.
fn read_in_place(kind: &str) -> bool {
    matches!(kind, "file" | "directory" | "settings")
}

/// The docs' list of skips after the managed block, in the docs' order.
async fn skip_of(
    marketplace: &KnownMarketplace,
    env: &dyn Environment,
    clock: &dyn Clock,
) -> Option<SkipReason> {
    if read_in_place(&marketplace.source) {
        return Some(SkipReason::LocalOrInlineSource);
    }
    if marketplace.seeded == Some(true) {
        return Some(SkipReason::SeedDirectory);
    }
    // Judgment: exactly 30 seconds ago is no longer "within the last 30 seconds".
    if let Some(refreshed) = marketplace.last_refreshed_at
        && clock.now().await - refreshed < REFRESH_WINDOW_MS
    {
        return Some(SkipReason::RefreshedWithin30Seconds);
    }
    if is_set(env, DISABLE_NONESSENTIAL_TRAFFIC).await {
        return Some(SkipReason::NonessentialTrafficDisabled);
    }
    None
}

fn refuse(message: &str) -> RefreshDecision {
    RefreshDecision::Refuse {
        outcome: "refuse",
        reason: "blocked-by-managed-settings",
        issue: error("marketplace", message),
    }
}

async fn named(
    marketplace: Option<&KnownMarketplace>,
    env: &dyn Environment,
    clock: &dyn Clock,
) -> RefreshDecision {
    let Some(marketplace) = marketplace else {
        return skip(SkipReason::MarketplaceUnknown);
    };
    if let Some(blocked) = &marketplace.blocked {
        return refuse(blocked);
    }
    match skip_of(marketplace, env, clock).await {
        None => RefreshDecision::Refresh {
            outcome: "refresh",
            marketplaces: vec![marketplace.name.clone()],
        },
        Some(reason) => skip(reason),
    }
}

/// A bare name through `/plugin install`: "Only marketplaces that have auto-update on, and only
/// after the lookup misses". Judgment (spec/judgments.md): each candidate is held to the skip
/// conditions of the named install, and a candidate managed settings block is dropped, not refused,
/// since no marketplace was named.
async fn bare_through_session(
    request: &RefreshRequest,
    env: &dyn Environment,
    clock: &dyn Clock,
) -> RefreshDecision {
    if !request.lookup_missed {
        return skip(SkipReason::LookupFoundThePlugin);
    }
    let candidates: Vec<&KnownMarketplace> = request
        .marketplaces
        .iter()
        .filter(|marketplace| marketplace.auto_update)
        .collect();
    if candidates.is_empty() {
        return skip(SkipReason::NoAutoUpdateMarketplace);
    }
    let mut refreshable: Vec<String> = Vec::new();
    for marketplace in candidates {
        if marketplace.blocked.is_none() && skip_of(marketplace, env, clock).await.is_none() {
            refreshable.push(marketplace.name.clone());
        }
    }
    if refreshable.is_empty() {
        skip(SkipReason::EveryCandidateSkipped)
    } else {
        RefreshDecision::Refresh {
            outcome: "refresh",
            marketplaces: refreshable,
        }
    }
}

/// What to refresh before the lookup of `request.ref`. The refresh of a named marketplace depends
/// neither on its `autoUpdate` nor on `DISABLE_AUTOUPDATER`: only the five skips and the managed
/// block above apply.
pub async fn refresh_before_install(request: &RefreshRequest) -> RefreshDecision {
    let Some(reference) = parse_plugin_ref(&request.reference) else {
        return skip(SkipReason::InvalidReference);
    };
    let (env, clock): (&dyn Environment, &dyn Clock) = (&request.env, &request.clock);
    let Some(marketplace) = reference.marketplace else {
        return match request.command {
            InstallCommand::PluginInstall => bare_through_session(request, env, clock).await,
            // A bare name through the CLI: "Nothing. It reads the cached catalogs without refreshing."
            InstallCommand::ClaudePluginInstall => skip(SkipReason::CliReadsCachedCatalogs),
        };
    };
    named(
        request
            .marketplaces
            .iter()
            .find(|candidate| candidate.name == marketplace),
        env,
        clock,
    )
    .await
}

/// What a failed refresh leaves.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshFailure {
    /// Always `cached catalog`.
    pub install_from: &'static str,
    /// The CLI's report; the docs name none for `/plugin install`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<PluginIssue>,
}

/// What `claude plugin install` reports when the refresh failed.
pub const NOT_REFRESHED: &str = "marketplace not refreshed";

/// "When the refresh fails, the install proceeds from the cached catalog and `claude plugin install`
/// reports `marketplace not refreshed`."
#[must_use]
pub fn refresh_failed(command: InstallCommand) -> RefreshFailure {
    let report = match command {
        InstallCommand::PluginInstall => None,
        InstallCommand::ClaudePluginInstall => Some(warning("marketplace", NOT_REFRESHED)),
    };
    RefreshFailure {
        install_from: "cached catalog",
        report,
    }
}

crate::registry::exports! {
    "updates/refresh-before-install" {
        "INSTALL_COMMANDS" => ok || INSTALL_COMMANDS;
        "SKIP_REASONS" => ok || SKIP_REASONS;
        "refreshBeforeInstall" => async ok |request: RefreshRequest| refresh_before_install(&request).await;
        "NOT_REFRESHED" => ok || NOT_REFRESHED;
        "refreshFailed" => ok |command: InstallCommand| refresh_failed(command);
    }
}
