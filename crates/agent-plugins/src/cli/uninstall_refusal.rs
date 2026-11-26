//! Port of `src/cli/uninstall-refusal.ts`, switched.
//!
//! The text after `"<name>" was not uninstalled: ` when a settings file of the scope still switches,
//! or may still switch, the plugin on (errors.md, "Plugin was not uninstalled").

use serde::Deserialize;

/// Why a settings file of the scope may still switch the plugin on (errors.md, "Plugin was not
/// uninstalled"): `still-on` the write reported success but the entry is still there; `change-failed`
/// the file could not be saved, for `error`; `unreadable` it exists but is not readable as settings;
/// `not-read` it is on a network path or a link to one, or its path could not be examined.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "cause", rename_all = "kebab-case")]
pub enum StillSwitchedOn {
    /// The write reported success but the entry is still there.
    StillOn {
        /// The settings file.
        file: String,
    },
    /// It exists but is not readable as settings.
    Unreadable {
        /// The settings file.
        file: String,
    },
    /// It is on a network path or a link to one, or its path could not be examined.
    NotRead {
        /// The settings file.
        file: String,
    },
    /// The file could not be saved.
    ChangeFailed {
        /// The settings file.
        file: String,
        /// Why.
        error: String,
    },
}

const TAKE_IT_OUT: &str = "It is still installed. Take it out of \"enabledPlugins\" in that file yourself, then uninstall it again.";

// Judgment: for a file that was not read the last sentence also offers repairing it (errors.md,
// "What to do"); the docs quote only the switched-on form's (spec/judgments.md).
const REPAIR_IT: &str = "It is still installed. Repair or replace that file, or take the plugin out of \"enabledPlugins\" in it yourself, then uninstall it again.";

/// The cause and the sentence that says what to do, for one settings file.
#[must_use]
pub fn still_on_reason(fact: &StillSwitchedOn) -> String {
    match fact {
        StillSwitchedOn::StillOn { file } => format!(
            "it is still switched on in {file}, although the settings change reported no error. {TAKE_IT_OUT}"
        ),
        StillSwitchedOn::ChangeFailed { file, error } => format!(
            "it is still switched on in {file}, and the settings change failed ({error}). {TAKE_IT_OUT}"
        ),
        StillSwitchedOn::Unreadable { file } => {
            format!("{file} is there and could not be read. {REPAIR_IT}")
        }
        StillSwitchedOn::NotRead { file } => format!(
            "{file} (not read: it is on a network path or is a link to one, or could not be checked). {REPAIR_IT}"
        ),
    }
}

crate::registry::exports! {
    "cli/uninstall-refusal" {
        "stillOnReason" => ok |fact: StillSwitchedOn| still_on_reason(&fact);
    }
}
