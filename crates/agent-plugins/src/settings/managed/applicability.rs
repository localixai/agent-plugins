//! Port of `src/settings/managed/applicability.ts`, switched.
//!
//! Where and when a deployed policy applies (managed-settings.md, "Where and when a policy
//! applies"): which surfaces and places read which sources, what reaches a running session, and
//! what waits for the developer. Pure decisions over facts the host copies in.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::settings::general::edit_effect::{Applies, Arrival, SettingsEdit, edit_effect};
use crate::settings::general::reach::FileScope;
use crate::settings::managed::delivery::Mechanism;

/// The surfaces a policy reaches.
pub const POLICY_SURFACES: [&str; 5] = [
    "terminal",
    "vscode",
    "jetbrains",
    "desktopCodeTab",
    "agentSdk",
];

/// The settings files an Agent SDK app chooses among with `settingSources`.
pub const SDK_SETTING_SOURCES: [&str; 3] = ["user", "project", "local"];

/// What a surface loads.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LoadedSources {
    /// Managed settings.
    pub managed: bool,
    /// The files, of `SDK_SETTING_SOURCES`.
    pub files: Vec<String>,
}

/// What a surface loads. "On the developer's machine, the terminal, the VS Code and JetBrains
/// extensions, the desktop app's Code tab, and Agent SDK sessions read all of these sources. Agent
/// SDK sessions load managed settings even when `settingSources` excludes the user, project, and
/// local files."
#[must_use]
pub fn sources_loaded_by(surface: &str, setting_sources: Option<Vec<String>>) -> LoadedSources {
    let every = || SDK_SETTING_SOURCES.map(str::to_owned).to_vec();
    let files = match surface {
        "agentSdk" => setting_sources.unwrap_or_else(every),
        _ => every(),
    };
    LoadedSources {
        managed: true,
        files,
    }
}

/// Where a session runs.
pub const PLACES: [&str; 8] = [
    "machine",
    "cloudHosted",
    "cloudSelfHosted",
    "tagHosted",
    "tagSelfHosted",
    "coworkOnMachine",
    "coworkFullVm",
    "coworkRemote",
];

/// How much of the managed settings file a place applies.
pub const FILE_USES: [&str; 3] = ["all", "crossSourceKeys", "none"];

/// What a place's reach depends on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceFacts {
    /// Server-managed settings deliver at least one policy key to this session.
    pub server_delivers_policy_key: bool,
    /// `managedSourcesBehavior` is `"merge"`.
    pub merging: bool,
}

/// The sources a place reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlaceReach {
    /// The session fetches server-managed settings.
    pub remote: bool,
    /// The session reads the device's MDM profile.
    pub mdm: bool,
    /// One of `FILE_USES`.
    pub file: &'static str,
}

const NOTHING: PlaceReach = reach(false, false, "none");

const fn reach(remote: bool, mdm: bool, file: &'static str) -> PlaceReach {
    PlaceReach { remote, mdm, file }
}

/// A self-hosted environment reads the managed settings file in its runner image, "by default
/// only when server-managed settings deliver no policy key, apart from the keys Claude Code reads
/// from every admin source"; the `"merge"` opt-in applies both.
const fn self_hosted_file(facts: &PlaceFacts) -> &'static str {
    if facts.server_delivers_policy_key && !facts.merging {
        "crossSourceKeys"
    } else {
        "all"
    }
}

/// The sources each place reads. Cloud sessions in an Anthropic-hosted environment read neither
/// device policy; Claude Tag sessions receive no server-managed settings; Cowork never fetches
/// them, and reads the device's policy only when it runs on the user's machine.
#[must_use]
pub fn reach_of_place(place: &str, facts: &PlaceFacts) -> PlaceReach {
    match place {
        "machine" => reach(true, true, "all"),
        "cloudHosted" => reach(true, false, "none"),
        "cloudSelfHosted" => reach(true, false, self_hosted_file(facts)),
        "tagSelfHosted" => reach(false, false, "all"),
        "coworkOnMachine" => reach(false, true, "all"),
        _ => NOTHING,
    }
}

/// Wherever a Cowork session runs, "claude.ai applies the admin console's
/// `strictKnownMarketplaces` and `blockedMarketplaces` lists itself" when someone adds a
/// marketplace from a git repository on claude.ai or from Customize.
const CLAUDE_AI_APPLIES_LISTS: [(&str, bool); 8] = [
    ("machine", false),
    ("cloudHosted", false),
    ("cloudSelfHosted", false),
    ("tagHosted", false),
    ("tagSelfHosted", false),
    ("coworkOnMachine", true),
    ("coworkFullVm", true),
    ("coworkRemote", true),
];

/// `CLAUDE_AI_APPLIES_LISTS[place]`.
#[must_use]
pub fn claude_ai_applies_marketplace_lists(place: &str) -> bool {
    CLAUDE_AI_APPLIES_LISTS
        .iter()
        .any(|(one, applies)| *one == place && *applies)
}

/// What decides whether remote settings are fetched.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the facts of the session the docs' fetch rule reads"
)]
pub struct FetchFacts {
    /// The session authenticates to Anthropic's API directly with an eligible credential.
    pub eligible_credential: bool,
    /// The provider is Anthropic's API, not Bedrock, Vertex, Foundry or another.
    pub anthropic_provider: bool,
    /// `ANTHROPIC_BASE_URL` points somewhere other than Anthropic's API.
    pub base_url_elsewhere: bool,
    /// The developer signed in to a gateway with `/login`.
    pub gateway_login: bool,
    /// Claude Desktop's Code tab runs behind a gateway that serves that session settings.
    pub desktop_gateway_session: bool,
}

/// Whether remote settings, the top-ranked managed source, are fetched at all
/// (managed-settings.md, "How Claude Code combines managed sources", rank 1): "only when the
/// session authenticates to Anthropic's API directly with an eligible credential, signs in to a
/// gateway with `/login`, or runs in Claude Desktop's Code tab behind a gateway that serves that
/// session settings. On other providers, or when `ANTHROPIC_BASE_URL` points somewhere other than
/// Anthropic's API, it starts at the next source."
#[must_use]
pub const fn fetches_remote_settings(facts: &FetchFacts) -> bool {
    facts.gateway_login
        || facts.desktop_gateway_session
        || (facts.eligible_credential && facts.anthropic_provider && !facts.base_url_elsewhere)
}

/// `ARRIVALS[mechanism]`.
const fn arrival_of(mechanism: Mechanism) -> Arrival {
    match mechanism {
        Mechanism::ServerManaged => Arrival::ServerManaged,
        Mechanism::Mdm | Mechanism::Hkcu => Arrival::Mdm,
        Mechanism::File => Arrival::File,
    }
}

/// When a change to `key` reaches a running session: "most changes reach a running session on the
/// schedule in the delivery mechanism table, without a restart"; `forceRemoteSettingsRefresh`,
/// `requiredMinimumVersion` and the keys read once "take effect at the next session start".
#[must_use]
pub fn reaches_running_session(mechanism: Mechanism, key: Option<String>) -> Applies {
    edit_effect(&SettingsEdit {
        scope: FileScope::Managed,
        arrival: arrival_of(mechanism),
        key,
        removed: None,
    })
    .applies
}

/// The members of `HelperPlan` this module reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperPlan {
    /// The helper the highest source that carries a policy key names; absent when none, or its
    /// value is invalid.
    #[serde(default)]
    pub helper: JsValue,
    /// The helper runs at launch: its source is an MDM policy or a managed file.
    pub runs_at_launch: bool,
    /// Server-managed settings are the policy, so the helper waits for a fetch that reports them
    /// removed.
    pub shadowed_by_remote: bool,
}

/// What the last fetch reported.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperFetch {
    /// Server-managed settings still deliver a policy.
    pub remote_still_delivers: bool,
}

/// Whether the policy helper runs now: a new or changed one "takes effect at the next launch", and
/// one server-managed settings shadow at launch "runs as soon as a fetch reports those settings
/// removed".
#[must_use]
pub fn helper_runs_now(plan: &HelperPlan, fetch: &HelperFetch) -> bool {
    if plan.helper.is_undefined() {
        return false;
    }
    plan.runs_at_launch || (plan.shadowed_by_remote && !fetch.remote_still_delivers)
}

/// A server-managed change.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerChange {
    /// The change sets something "that needs approval", such as a hook or an `env` variable.
    pub needs_approval: bool,
    /// One of "the updates that wait for the next launch".
    pub waits_for_next_launch: bool,
    /// An IDE extension or the Agent SDK hosts the session, so no dialog can be shown to the
    /// developer.
    pub hosted: bool,
}

/// When a server-managed change applies.
pub const SERVER_CHANGE_TIMINGS: [&str; 4] = [
    "next-launch",
    "awaits-approval",
    "applies-for-current-run",
    "next-poll",
];

/// When a server-managed change applies. "Apart from the updates that wait for the next launch, a
/// server-managed change to a setting that needs approval waits for the developer to accept the
/// dialog in an interactive session, and applies for the current run in a session an IDE extension
/// or the Agent SDK hosts. Other server-managed changes apply on the next poll."
#[must_use]
pub const fn server_change_timing(change: &ServerChange) -> &'static str {
    if change.waits_for_next_launch {
        return "next-launch";
    }
    if !change.needs_approval {
        return "next-poll";
    }
    if change.hosted {
        "applies-for-current-run"
    } else {
        "awaits-approval"
    }
}

/// What `requiredMinimumVersion` depends on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MinimumVersionFacts {
    /// The binary is older than `requiredMinimumVersion`.
    pub outdated: bool,
    /// The session is already running.
    pub running: bool,
}

/// What `requiredMinimumVersion` does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimumVersionEffect {
    /// It blocks the start.
    pub blocks_start: bool,
    /// It ends the session.
    pub ends_session: bool,
}

/// "`requiredMinimumVersion` blocks an outdated binary from starting and doesn't end a session
/// that's already running."
#[must_use]
pub const fn minimum_version_effect(facts: &MinimumVersionFacts) -> MinimumVersionEffect {
    MinimumVersionEffect {
        blocks_start: facts.outdated && !facts.running,
        ends_session: false,
    }
}

/// What startup's fetch depends on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the facts of the launch the docs' startup fetch reads"
)]
pub struct StartupFetchFacts {
    /// `forceRemoteSettingsRefresh`.
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
}

/// What startup does about the fetch.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupFetch {
    /// Startup waits for the fetch.
    pub blocks_startup: bool,
    /// The session exits when the fetch fails.
    pub exits_on_failure: bool,
    /// Seconds startup waits for the fetch before it opens the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waits_up_to_seconds: Option<f64>,
}

const SIGN_IN_WAIT_SECONDS: f64 = 5.0;
const OPEN: StartupFetch = StartupFetch {
    blocks_startup: false,
    exits_on_failure: false,
    waits_up_to_seconds: None,
};
const BLOCKING: StartupFetch = StartupFetch {
    blocks_startup: true,
    exits_on_failure: true,
    waits_up_to_seconds: None,
};

/// What startup does about the fetch (settings-reference.md, `forceRemoteSettingsRefresh`):
/// `true` blocks until a fresh fetch and exits on failure; unset does not block, though a sign-in
/// startup waits up to five seconds; "Anthropic can require the settings", which blocks a start
/// without them; a Cloud gateway session waits and exits when the gateway cannot be reached. The
/// `claude auth` subcommands are exempt, and a session that does not fetch starts without waiting.
#[must_use]
pub const fn startup_fetch_of(facts: &StartupFetchFacts) -> StartupFetch {
    if facts.auth_subcommand || !facts.fetches_remote {
        return OPEN;
    }
    if facts.force_remote_settings_refresh || facts.gateway_session || facts.anthropic_requires {
        return BLOCKING;
    }
    if facts.signs_in_at_startup {
        StartupFetch {
            waits_up_to_seconds: Some(SIGN_IN_WAIT_SECONDS),
            ..OPEN
        }
    } else {
        OPEN
    }
}

crate::registry::exports! {
    "settings/managed/applicability" {
        "POLICY_SURFACES" => ok || POLICY_SURFACES;
        "SDK_SETTING_SOURCES" => ok || SDK_SETTING_SOURCES;
        "sourcesLoadedBy" => ok |surface: String, setting_sources: Option<Vec<String>>| sources_loaded_by(&surface, setting_sources);
        "PLACES" => ok || PLACES;
        "FILE_USES" => ok || FILE_USES;
        "reachOfPlace" => ok |place: String, facts: PlaceFacts| reach_of_place(&place, &facts);
        "claudeAiAppliesMarketplaceLists" => ok |place: String| claude_ai_applies_marketplace_lists(&place);
        "fetchesRemoteSettings" => ok |facts: FetchFacts| fetches_remote_settings(&facts);
        "reachesRunningSession" => ok |mechanism: Mechanism, key: Option<String>| reaches_running_session(mechanism, key);
        "helperRunsNow" => ok |plan: HelperPlan, fetch: HelperFetch| helper_runs_now(&plan, &fetch);
        "SERVER_CHANGE_TIMINGS" => ok || SERVER_CHANGE_TIMINGS;
        "serverChangeTiming" => ok |change: ServerChange| server_change_timing(&change);
        "minimumVersionEffect" => ok |facts: MinimumVersionFacts| minimum_version_effect(&facts);
        "startupFetchOf" => ok |facts: StartupFetchFacts| startup_fetch_of(&facts);
    }
}
