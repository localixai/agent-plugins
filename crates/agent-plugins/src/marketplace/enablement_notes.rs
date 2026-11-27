//! Port of `src/marketplace/enablement-notes.ts`, switched.
//!
//! Two questions about `enabledPlugins` that the loading reference answers (plugins/loading.md,
//! "Find where a plugin is enabled"): why a plugin you turned off still loads, and whether Claude
//! Code fetches a plugin only a project enables. Pure over the facts a host read from the settings
//! layers; which layer wins is the settings module's (`enabledPlugin`), so a caller passes its
//! `decidedBy` here, and this module imports nothing from it (same layer).

use serde::{Deserialize, Serialize};

use super::install_errors::{ErrorsTabEntry, enabled_not_installed};
use crate::format::settings_files::SETTINGS_FILE;

/// The sources an `enabledPlugins` entry can come from (the table of "Find where a plugin is
/// enabled").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnableSource {
    /// `--add-dir`.
    AddDir,
    /// User settings.
    User,
    /// The project's `.claude/settings.json`.
    Project,
    /// `.claude/settings.local.json`.
    Local,
    /// `--settings`.
    Flag,
    /// Managed settings.
    Managed,
}

// ---- "Disabled in user settings but still loads" ----

struct Overrider {
    /// How the message names the source that overrode the user setting.
    label: &'static str,
    /// Whether a `false` in `.claude/settings.local.json` can opt out of it.
    opt_out: bool,
}

/// The sources above user settings, with the names the docs give: `project`, `project,
/// gitignored` for `.claude/settings.local.json`, `cli flag` and `managed`. `--add-dir` ranks
/// below user settings and never overrides it.
const fn overriders(source: EnableSource) -> Option<Overrider> {
    match source {
        EnableSource::Project => Some(Overrider {
            label: "project",
            opt_out: true,
        }),
        EnableSource::Local => Some(Overrider {
            label: "project, gitignored",
            opt_out: false,
        }),
        EnableSource::Flag => Some(Overrider {
            label: "cli flag",
            opt_out: false,
        }),
        EnableSource::Managed => Some(Overrider {
            label: "managed",
            opt_out: false,
        }),
        EnableSource::AddDir | EnableSource::User => None,
    }
}

/// `false` for the id in a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OptOut {
    /// The file.
    pub file: &'static str,
    /// Always `false`.
    pub value: bool,
}

/// A user setting a higher source overrode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDisableOverridden {
    /// The row's text in `claude plugin list` and `/plugin`.
    pub message: String,
    /// How to turn it off on this machine: `false` for the id in this file, which outranks the
    /// project file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt_out: Option<OptOut>,
}

/// The settings layer that decided.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Winner {
    /// The user setting.
    #[serde(default)]
    pub user: Option<bool>,
    /// The source that decided.
    pub decided_by: EnableSource,
    /// Whether the plugin is on.
    pub on: bool,
}

/// `Disabled in ~/.claude/settings.json but still loads — project settings enable it, which
/// overrides your user setting`, for a plugin set to `false` in user settings that a `true` in a
/// higher source keeps on (loading.md; troubleshooting.md). Judgment (spec/judgments.md): the
/// docs quote the sentence for `project` only, so another source is named in the same place by
/// its label (`cli flag settings enable it`).
#[must_use]
pub fn overridden_user_disable(winner: &Winner) -> Option<UserDisableOverridden> {
    let overrider = overriders(winner.decided_by);
    if winner.user != Some(false) || !winner.on {
        return None;
    }
    let overrider = overrider?;
    let message = format!(
        "Disabled in {} but still loads — {} settings enable it, which overrides your user setting",
        SETTINGS_FILE.user, overrider.label
    );
    Some(UserDisableOverridden {
        message,
        opt_out: overrider.opt_out.then_some(OptOut {
            file: SETTINGS_FILE.local,
            value: false,
        }),
    })
}

// ---- "Enabled in project settings but not installed" ----

/// One source that sets the plugin to `true`; a `local` file says whether git tracks it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrueIn {
    /// The source.
    pub source: EnableSource,
    /// Whether git tracks the file.
    #[serde(default)]
    pub git_tracked: Option<bool>,
}

/// What a host knows of an enabled plugin.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnabledFacts {
    /// The plugin.
    pub name: String,
    /// Its marketplace.
    pub marketplace: String,
    /// Every source that sets the plugin to `true`.
    pub true_in: Vec<TrueIn>,
    /// An install record exists for it.
    pub installed: bool,
    /// Its marketplace entry has a relative-path source.
    pub relative_path: bool,
    /// A seed directory already holds it.
    pub seeded: bool,
}

/// What Claude Code does with an enabled plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnabledFetchDecision {
    /// Claude Code downloads the plugin's external source.
    pub fetch: bool,
    /// It loads from the marketplace itself, needing no install record.
    pub in_place: bool,
    /// The Errors tab entry, when a project enables a plugin that is not here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorsTabEntry>,
}

/// "Claude Code fetches a plugin with an external source only when one of these sources sets it
/// to `true`."
const fn fetches(source: EnableSource, git_tracked: Option<bool>) -> bool {
    match source {
        EnableSource::User | EnableSource::Flag | EnableSource::Managed => true,
        EnableSource::Local => matches!(git_tracked, Some(false)),
        EnableSource::Project | EnableSource::AddDir => false,
    }
}

const NOTHING: EnabledFetchDecision = EnabledFetchDecision {
    fetch: false,
    in_place: false,
    error: None,
};

/// What Claude Code does with an enabled plugin that has no install record on this machine. A
/// relative-path plugin "needs no install record because it loads from the marketplace itself",
/// and a plugin a seed directory holds is there; otherwise an external source is fetched only
/// when user settings, an untracked `.claude/settings.local.json`, `--settings` or managed
/// settings set it to `true`. When only the project's `.claude/settings.json` does, nothing is
/// fetched and the Errors tab says so. Judgment (spec/judgments.md): a `true` from `--add-dir`
/// alone neither fetches nor reports, since the docs name it in neither list.
#[must_use]
pub fn decide_enabled_fetch(facts: &EnabledFacts) -> EnabledFetchDecision {
    if facts.installed || facts.seeded || facts.true_in.is_empty() {
        return NOTHING;
    }
    if facts.relative_path {
        return EnabledFetchDecision {
            fetch: false,
            in_place: true,
            error: None,
        };
    }
    if facts
        .true_in
        .iter()
        .any(|one| fetches(one.source, one.git_tracked))
    {
        return EnabledFetchDecision {
            fetch: true,
            in_place: false,
            error: None,
        };
    }
    if facts.true_in.iter().any(|one| issues_error(one.source)) {
        EnabledFetchDecision {
            error: Some(enabled_not_installed(&facts.name, &facts.marketplace)),
            ..NOTHING
        }
    } else {
        NOTHING
    }
}

/// Only the project's own file makes the "isn't installed here" entry.
const fn issues_error(source: EnableSource) -> bool {
    match source {
        EnableSource::Project => true,
        EnableSource::User
        | EnableSource::Local
        | EnableSource::Flag
        | EnableSource::Managed
        | EnableSource::AddDir => false,
    }
}

crate::registry::exports! {
    "marketplace/enablement-notes" {
        "overriddenUserDisable" => ok |winner: Winner| overridden_user_disable(&winner);
        "decideEnabledFetch" => ok |facts: EnabledFacts| decide_enabled_fetch(&facts);
    }
}
