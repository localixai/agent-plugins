//! Port of `src/settings/managed/limits.ts`, switched.
//!
//! What managed settings cannot enforce, and how to check a policy that does not behave
//! (managed-settings.md "What a developer can change"; plugins/org.md "Plan for what managed settings
//! can't enforce" and "Troubleshoot policy"). Pure: each function answers from facts the host gives.
//! The docs give the facts; the keys and the returned shape are the kit's (spec/judgments.md).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Who acts, as far as the managed settings are concerned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedActor {
    /// An administrator on the machine can edit the managed source itself.
    #[serde(default)]
    pub local_admin: Option<bool>,
    /// The developer calls the API from a tool other than Claude Code.
    #[serde(default)]
    pub other_tool: Option<bool>,
    /// The edit is to the local copy of server-managed settings.
    #[serde(default)]
    pub edits_server_cache: Option<bool>,
}

/// Whether the managed settings bind, and why not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManagedBinding {
    /// Whether they bind.
    pub bound: bool,
    /// Why not, or the remedy the docs name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

/// "Local admin rights": an administrator can edit the managed source, so MDM redeploys it; "The
/// server-managed cache": an edit "lasts only until the next successful fetch"; "Other tools":
/// "managed settings bind Claude Code only".
#[must_use]
pub fn managed_binding(actor: &ManagedActor) -> ManagedBinding {
    if actor.other_tool == Some(true) {
        return ManagedBinding {
            bound: false,
            note: Some(
                "managed settings bind Claude Code only; a developer's other tools aren't under them",
            ),
        };
    }
    if actor.edits_server_cache == Some(true) {
        return ManagedBinding {
            bound: true,
            note: Some("the edit lasts only until the next successful fetch"),
        };
    }
    if actor.local_admin == Some(true) {
        return ManagedBinding {
            bound: false,
            note: Some("redeploy the profile or file on a schedule with MDM tooling"),
        };
    }
    ManagedBinding {
        bound: true,
        note: None,
    }
}

/// The requests of security reviews that have no dedicated key.
pub const UNENFORCEABLE_REQUESTS: [&str; 5] = [
    "per-user-targeting",
    "entries-inside-allowed-marketplace",
    "hide-plugin-command",
    "gate-plugin-dir-through-allowlist",
    "enforce-claude-ai-toggles",
];

/// The nearest existing controls per request ("These requests ... have no dedicated key").
pub const NEAREST_CONTROLS: [(&str, &[&str]); 5] = [
    (
        "per-user-targeting",
        &["endpoint-managed settings", "gateway policies"],
    ),
    ("entries-inside-allowed-marketplace", &["enabledPlugins"]),
    (
        "hide-plugin-command",
        &[
            "strictKnownMarketplaces",
            "enabledPlugins",
            "disableSideloadFlags",
        ],
    ),
    (
        "gate-plugin-dir-through-allowlist",
        &["disableSideloadFlags"],
    ),
    ("enforce-claude-ai-toggles", &["synced plugins controls"]),
];

/// The symptoms "Troubleshoot policy" lists.
pub const POLICY_SYMPTOMS: [&str; 4] = [
    "managed-file-not-parsed",
    "managed-source-not-loaded",
    "blocked-by-enterprise-policy",
    "disabled-plugin-still-loads",
];

/// What to check first, per symptom.
pub const SYMPTOM_CHECKS: [(&str, &str); 4] = [
    (
        "managed-file-not-parsed",
        "managed-settings.json is not valid JSON: Claude Code refuses to start and prints an error naming the file; a file with one invalid entry keeps the rest of its policy",
    ),
    (
        "managed-source-not-loaded",
        "run /status and look for `Enterprise managed settings` in the `Setting sources` line; if it is missing, the source did not load",
    ),
    (
        "blocked-by-enterprise-policy",
        "the message names the marketplace or its source and, for an allowlist, lists the allowed sources",
    ),
    (
        "disabled-plugin-still-loads",
        "another settings source re-enabled it, such as a managed `enabledPlugins` entry; /plugin and `claude plugin list` show `Disabled in ~/.claude/settings.json but still loads` with that source",
    ),
];

/// Whether the `Setting sources` line of `/status` lists the managed source ("If it's missing, the source didn't load").
#[must_use]
pub fn managed_source_loaded(setting_sources_line: &str) -> bool {
    setting_sources_line.contains("Enterprise managed settings")
}

/// Where the cause of a skipped skills-directory plugin lies (plugins/loading.md, "Skills-directory plugins blocked by managed settings").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillsDirCause {
    /// Ask your administrator.
    AskAdministrator,
    /// Check the repository's plugin causes.
    CheckRepositoryPluginCauses,
}

/// "Run `/status` ... If the `Setting sources` line lists `Enterprise managed settings`, ask your administrator."
#[must_use]
pub fn skills_dir_cause(setting_sources_line: &str) -> SkillsDirCause {
    if managed_source_loaded(setting_sources_line) {
        SkillsDirCause::AskAdministrator
    } else {
        SkillsDirCause::CheckRepositoryPluginCauses
    }
}

/// What is known about loading a whole plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholePluginFacts {
    /// The folder is in a marketplace the organization allows and installed from there.
    pub allowed_marketplace_available: bool,
    /// The administrator turned off the `--plugin-dir` flag (`disableSideloadFlags`).
    pub sideload_flags_disabled: bool,
}

/// "Load the whole plugin": through an allowed marketplace, or for one session with `--plugin-dir` "unless your administrator has also turned off that flag".
#[must_use]
pub fn whole_plugin_routes(facts: &WholePluginFacts) -> Vec<&'static str> {
    let mut routes = Vec::new();
    if facts.allowed_marketplace_available {
        routes.push("marketplace");
    }
    if !facts.sideload_flags_disabled {
        routes.push("plugin-dir");
    }
    routes
}

/// An entry of a delivered payload after validation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadEntry {
    /// The setting.
    pub key: String,
    /// Whether its value passed validation.
    pub valid: bool,
    /// The key fails closed: an invalid value falls back to a stricter one.
    #[serde(default)]
    pub falls_back_stricter: Option<bool>,
}

/// What a delivered payload comes to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadOutcome {
    /// Whether the valid entries apply.
    pub apply: bool,
    /// The cache is replaced only when the payload is applied.
    pub update_cache: bool,
    /// The failed-load cause shown by the startup notice, `/status` and `claude doctor`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<&'static str>,
}

const RETENTION_KEYS: [&str; 2] = ["cleanupPeriodDays", "desktopSessionCleanupPeriodDays"];

/// "Claude Code applies nothing from a payload and leaves the cache unchanged when" every setting
/// fails validation, none falls back to a stricter value, and the payload holds a key other than the
/// two retention keys. Otherwise the valid entries apply.
#[must_use]
pub fn delivered_payload_outcome(entries: &[PayloadEntry]) -> PayloadOutcome {
    let all_invalid = !entries.is_empty() && entries.iter().all(|entry| !entry.valid);
    let no_stricter = entries
        .iter()
        .all(|entry| entry.falls_back_stricter != Some(true));
    let has_other_key = entries
        .iter()
        .any(|entry| !RETENTION_KEYS.contains(&entry.key.as_str()));
    if all_invalid && no_stricter && has_other_key {
        return PayloadOutcome {
            apply: false,
            update_cache: false,
            failure: Some("no setting in the server response could be applied as written"),
        };
    }
    PayloadOutcome {
        apply: true,
        update_cache: true,
        failure: None,
    }
}

/// At startup on the cache: an invalid retention value "stays in the cached copy and is never applied".
#[must_use]
pub fn retention_value_applies(key: &str, valid: bool) -> bool {
    valid || !RETENTION_KEYS.contains(&key)
}

crate::registry::exports! {
    "settings/managed/limits" {
        "managedBinding" => ok |actor: ManagedActor| managed_binding(&actor);
        "UNENFORCEABLE_REQUESTS" => ok || UNENFORCEABLE_REQUESTS;
        "NEAREST_CONTROLS" => ok || NEAREST_CONTROLS.iter().copied().collect::<IndexMap<&str, &[&str]>>();
        "POLICY_SYMPTOMS" => ok || POLICY_SYMPTOMS;
        "SYMPTOM_CHECKS" => ok || SYMPTOM_CHECKS.iter().copied().collect::<IndexMap<&str, &str>>();
        "managedSourceLoaded" => ok |setting_sources_line: String| managed_source_loaded(&setting_sources_line);
        "skillsDirCause" => ok |setting_sources_line: String| skills_dir_cause(&setting_sources_line);
        "wholePluginRoutes" => ok |facts: WholePluginFacts| whole_plugin_routes(&facts);
        "deliveredPayloadOutcome" => ok |entries: Vec<PayloadEntry>| delivered_payload_outcome(&entries);
        "retentionValueApplies" => ok |key: String, valid: bool| retention_value_applies(&key, valid);
    }
}
