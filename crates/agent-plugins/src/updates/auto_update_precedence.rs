//! Port of `src/updates/auto-update-precedence.ts`, switched.
//!
//! Which marketplaces auto-update, and whether the pass runs at all (plugins/loading.md, "Which
//! marketplaces and plugins auto-update"; plugins/install.md, "Turn auto-update on or off for a
//! marketplace"; plugins/org.md, "Turn auto-update on or off per marketplace" and "Turn updates off
//! for the whole fleet"; plugins/host-marketplace.md, "Turn on auto-update"). Pure: the host reads
//! the files, the kit decides.

use serde::{Deserialize, Serialize};

use super::env_flags::{DISABLE_NONESSENTIAL_TRAFFIC, is_on, is_set};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::marketplace_name_lists::{OFFICIAL_NAMES, is_official_but_off};
use crate::ports::environment::Environment;
use crate::registry::Port;
use crate::settings::layers::PolicyScope;

/// What the host knows of one marketplace's auto-update setting.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateFacts {
    /// The marketplace.
    pub name: String,
    /// `autoUpdate` on the marketplace's `extraKnownMarketplaces` entry in the settings file that wins; unset is `undefined`.
    #[serde(default)]
    pub settings_entry: Option<bool>,
    /// `autoUpdate` on its `known_marketplaces.json` entry, which the Enable auto-update toggle writes.
    #[serde(default)]
    pub recorded: Option<bool>,
    /// The marketplace was added from claude.ai.
    #[serde(default)]
    pub from_claude_ai: Option<bool>,
    /// A seed directory supplies it: "forces `autoUpdate` off for seed marketplaces" (plugins/org.md).
    #[serde(default)]
    pub seeded: Option<bool>,
}

fn is_official_on(name: &str) -> bool {
    OFFICIAL_NAMES.contains(&name) && !is_official_but_off(name)
}

/// Item 3: on for the official names and for claude.ai marketplaces, off for every other one.
fn default_on(facts: &AutoUpdateFacts) -> bool {
    facts.from_claude_ai == Some(true) || is_official_on(&facts.name)
}

/// Whether the marketplace auto-updates: "the first of these that is set": the settings entry, the
/// `known_marketplaces.json` entry, the default. A seed marketplace is off before any of them
/// (judgment on the order, spec/judgments.md: the docs say the seed "forces" it off).
#[must_use]
pub fn marketplace_auto_update(facts: &AutoUpdateFacts) -> bool {
    if facts.seeded == Some(true) {
        return false;
    }
    facts
        .settings_entry
        .or(facts.recorded)
        .unwrap_or_else(|| default_on(facts))
}

/// The variables that turn the whole pass off ("the whole pass is off and the toggle is hidden").
const PASS_OFF_BY: [&str; 2] = ["DISABLE_UPDATES", "DISABLE_AUTOUPDATER"];

/// Whether the pass runs, and the Enable auto-update toggle shows: `DISABLE_UPDATES=1`,
/// `DISABLE_AUTOUPDATER=1` or any non-empty `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` turns both
/// off "unless you also set `FORCE_AUTOUPDATE_PLUGINS=1`".
pub async fn auto_update_pass_allowed(env: &dyn Environment) -> bool {
    if is_on(env, "FORCE_AUTOUPDATE_PLUGINS").await {
        return true;
    }
    let mut off = false;
    for name in PASS_OFF_BY {
        if is_on(env, name).await {
            off = true;
            break;
        }
    }
    let off = off || is_set(env, DISABLE_NONESSENTIAL_TRAFFIC).await;
    !off
}

/// The labels of the toggle, by whether the marketplace auto-updates now.
const TOGGLE_LABELS: [(&str, &str); 2] =
    [("on", "Disable auto-update"), ("off", "Enable auto-update")];

/// What the Marketplaces tab labels the toggle: "Enable auto-update" when it is off, "Disable auto-update" when it is on.
#[must_use]
pub fn toggle_label(facts: &AutoUpdateFacts) -> &'static str {
    let state = if marketplace_auto_update(facts) {
        "on"
    } else {
        "off"
    };
    TOGGLE_LABELS
        .iter()
        .find(|(name, _)| *name == state)
        .map_or("", |(_, label)| label)
}

/// The settings file that declares the marketplace under `extraKnownMarketplaces`, as the registry plans it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    /// The file's scope.
    pub scope: PolicyScope,
    /// Its `autoUpdate`, when set.
    #[serde(default)]
    pub auto_update: Option<bool>,
}

/// The user's toggle.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleRequest {
    /// The marketplace.
    pub name: String,
    /// What the user chose: Enable (true) or Disable (false).
    pub enable: bool,
    /// The session's environment.
    pub env: Port,
    /// The settings file that declares it, if any.
    #[serde(default)]
    pub declared: Option<Declaration>,
}

/// A file the toggle writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "file")]
pub enum ToggleWrite {
    /// `known_marketplaces.json`.
    #[serde(rename = "known_marketplaces.json")]
    KnownMarketplaces,
    /// The settings file of that scope.
    #[serde(rename = "settings")]
    Settings {
        /// Its scope.
        scope: PolicyScope,
    },
}

/// What the toggle does.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum ToggleDecision {
    /// It is not shown.
    Hidden,
    /// It is refused.
    Refused {
        /// Why.
        issue: PluginIssue,
    },
    /// It writes these files.
    Write {
        /// The value written.
        #[serde(rename = "autoUpdate")]
        auto_update: bool,
        /// The files.
        writes: Vec<ToggleWrite>,
    },
}

/// The files a user can edit: user, project and local settings. A managed, `--settings` or `--add-dir` file is not written.
const fn user_writable(scope: PolicyScope) -> bool {
    match scope {
        PolicyScope::User | PolicyScope::Project | PolicyScope::Local => true,
        PolicyScope::Managed | PolicyScope::Parent | PolicyScope::Flag | PolicyScope::AddDir => {
            false
        }
    }
}

/// "Claude Code refuses the user's `/plugin` toggle with an error that starts `Auto-update for '<name>' is set by`."
fn refusal(name: &str) -> PluginIssue {
    error(
        &format!("extraKnownMarketplaces.{name}.autoUpdate"),
        &format!(
            "Auto-update for '{name}' is set by your organization's managed settings and cannot be changed here"
        ),
    )
}

fn writes_for(declared: Option<&Declaration>) -> Vec<ToggleWrite> {
    let recorded = ToggleWrite::KnownMarketplaces;
    match declared {
        Some(declared) if user_writable(declared.scope) => vec![
            recorded,
            ToggleWrite::Settings {
                scope: declared.scope,
            },
        ],
        Some(_) | None => vec![recorded],
    }
}

/// The Enable or Disable auto-update toggle: hidden when the pass is off; refused when a managed
/// entry sets the field; otherwise it writes `known_marketplaces.json`, and the settings entry too
/// "when a settings file also declares the marketplace under `extraKnownMarketplaces`" (only a file
/// the user can write, judgment). A managed entry that leaves the field unset keeps the user toggle.
pub async fn toggle_decision(request: &ToggleRequest) -> ToggleDecision {
    if !auto_update_pass_allowed(&request.env).await {
        return ToggleDecision::Hidden;
    }
    let declared = request.declared.as_ref();
    if declared.is_some_and(|declared| {
        declared.scope == PolicyScope::Managed && declared.auto_update.is_some()
    }) {
        return ToggleDecision::Refused {
            issue: refusal(&request.name),
        };
    }
    ToggleDecision::Write {
        auto_update: request.enable,
        writes: writes_for(declared),
    }
}

crate::registry::exports! {
    "updates/auto-update-precedence" {
        "marketplaceAutoUpdate" => ok |facts: AutoUpdateFacts| marketplace_auto_update(&facts);
        "autoUpdatePassAllowed" => async ok |env: Port| auto_update_pass_allowed(&env).await;
        "toggleLabel" => ok |facts: AutoUpdateFacts| toggle_label(&facts);
        "toggleDecision" => async ok |request: ToggleRequest| toggle_decision(&request).await;
    }
}
