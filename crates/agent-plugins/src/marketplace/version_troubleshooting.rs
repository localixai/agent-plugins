//! Port of `src/marketplace/version-troubleshooting.ts`, switched.
//!
//! The way out of `Version: unknown` for a plugin installed from a git repository (plugins/
//! troubleshooting.md, "A plugin installed from a git repository shows version `unknown`"): Claude
//! Code could not read the commit it uses as the plugin's version. The four steps are the section's,
//! with the commands as it writes them; the plan is data a host shows or runs.

use serde::{Deserialize, Serialize};

use super::install_scopes::{InstallScope, is_writable_scope};

/// One step of the way out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionStep {
    /// What to run.
    pub command: String,
    /// Set on the step that needs a Claude Code release: the first release that clones in a readable format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_claude_code: Option<String>,
}

/// A rerun of the update for one scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Rerun {
    /// The scope.
    pub scope: InstallScope,
    /// What to run.
    pub command: String,
}

/// The steps and the reruns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnknownVersionPlan {
    /// The steps.
    pub steps: Vec<VersionStep>,
    /// `claude plugin update` "changes one scope's block per run": one rerun for each block still showing `unknown`.
    pub rerun: Vec<Rerun>,
}

/// "v2.1.302 or later, which clones in a format it can read the commit from".
const MINIMUM_CLAUDE_CODE: &str = "2.1.302";

/// A plugin as `<name>@<marketplace>`: the two parts the commands use, never split out of one string.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginRef {
    /// Its name.
    pub name: String,
    /// Its marketplace.
    pub marketplace: String,
}

/// The steps for a plugin and the scopes of the blocks that still show `unknown` afterwards. The
/// plugin is given as its two parts, so the marketplace of step 2 is always the plugin's own.
/// Judgments: a managed scope gets no rerun, as the user cannot update it; a scope listed twice is
/// rerun once.
#[must_use]
pub fn unknown_version_steps(
    plugin: &PluginRef,
    unknown_scopes: &[InstallScope],
) -> UnknownVersionPlan {
    let id = format!("{}@{}", plugin.name, plugin.marketplace);
    let mut once: Vec<InstallScope> = Vec::new();
    for scope in unknown_scopes {
        if !once.contains(scope) {
            once.push(*scope);
        }
    }
    UnknownVersionPlan {
        steps: vec![
            VersionStep {
                command: "claude update".to_owned(),
                minimum_claude_code: Some(MINIMUM_CLAUDE_CODE.to_owned()),
            },
            VersionStep {
                command: format!("claude plugin marketplace update {}", plugin.marketplace),
                minimum_claude_code: None,
            },
            VersionStep {
                command: format!("claude plugin update {id}"),
                minimum_claude_code: None,
            },
            VersionStep {
                command: "claude plugin list".to_owned(),
                minimum_claude_code: None,
            },
        ],
        rerun: once
            .into_iter()
            .filter(|scope| is_writable_scope(*scope))
            .map(|scope| Rerun {
                scope,
                command: format!("claude plugin update {id} --scope {}", scope.as_str()),
            })
            .collect(),
    }
}

crate::registry::exports! {
    "marketplace/version-troubleshooting" {
        "unknownVersionSteps" => ok |plugin: PluginRef, unknown_scopes: Vec<InstallScope>| unknown_version_steps(&plugin, &unknown_scopes);
    }
}
