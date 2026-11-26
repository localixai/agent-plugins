//! Port of `src/marketplace/missing-plugin-diagnosis.ts`, switched.
//!
//! "Plugin doesn't appear or its skills don't show up" (plugins/troubleshooting.md): the three checks, in
//! the docs' order, and what to do next. Pure over what the host can read.

use serde::{Deserialize, Serialize};

/// What the host can read about a plugin that does not show up.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's interface: one flag per fact the host reads"
)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingPluginFacts {
    /// It is installed.
    pub installed: bool,
    /// It is enabled.
    pub enabled: bool,
    /// The Errors tab holds entries for the plugin.
    pub errors: Vec<String>,
    /// Installed during this session, not yet reloaded.
    pub installed_this_session: bool,
    /// Whose plugin it is, for the last step.
    pub own_plugin: bool,
}

/// What to do next.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "step")]
pub enum Step {
    /// `confirm-installed-and-enabled`.
    #[serde(rename = "confirm-installed-and-enabled")]
    ConfirmInstalledAndEnabled {
        /// Where to look.
        r#where: String,
    },
    /// `read-errors-tab`.
    #[serde(rename = "read-errors-tab")]
    ReadErrorsTab {
        /// The entries.
        errors: Vec<String>,
    },
    /// `reload`.
    #[serde(rename = "reload")]
    Reload {
        /// The command.
        command: String,
    },
    /// `own-plugin`.
    #[serde(rename = "own-plugin")]
    OwnPlugin {
        /// Where to read on.
        see: String,
    },
    /// `someone-elses-plugin`.
    #[serde(rename = "someone-elses-plugin")]
    SomeoneElsesPlugin {
        /// Where to look.
        look: String,
    },
}

/// The next step: the first check that fails, else what to do when the plugin loads with no error and still shows no skills.
#[must_use]
pub fn next_step(facts: &MissingPluginFacts) -> Step {
    if !facts.installed || !facts.enabled {
        return Step::ConfirmInstalledAndEnabled {
            r#where: "/plugin Installed tab, or `claude plugin list` (Status: \u{2714} enabled)"
                .to_owned(),
        };
    }
    if !facts.errors.is_empty() {
        return Step::ReadErrorsTab {
            errors: facts.errors.clone(),
        };
    }
    if facts.installed_this_session {
        return Step::Reload {
            command: "/reload-plugins".to_owned(),
        };
    }
    if facts.own_plugin {
        Step::OwnPlugin {
            see: "Plugin loads but its skills are missing".to_owned(),
        }
    } else {
        Step::SomeoneElsesPlugin {
            look:
                "the plugin's details pane under Installed in /plugin, which lists what it contains"
                    .to_owned(),
        }
    }
}

crate::registry::exports! {
    "marketplace/missing-plugin-diagnosis" {
        "nextStep" => ok |facts: MissingPluginFacts| next_step(&facts);
    }
}
