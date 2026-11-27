//! Port of `src/cli/uninstall-data.ts`, switched.
//!
//! What `claude plugin uninstall` deletes and keeps (plugins/cli-reference.md, "plugin uninstall",
//! "What an uninstall deletes and keeps"). From the last scope a plugin is installed at, its stored
//! options and secrets and its data directory `~/.claude/plugins/data/<id>/` go, except: `--keep-data`
//! keeps the directory; another installed plugin that uses the same folder keeps it (the folder is
//! `pluginDataId`, and "an ID \[that\] differs from this one only in letter case" names the same
//! folder on a case-insensitive file system, so there folders are compared without regard to case,
//! J3.19); and installed plugins that cannot be read
//! back after the removal keep the options, the secrets and the directory ("the plugin may still be
//! installed at another scope"). Pure over what the host read back.

use serde::{Deserialize, Serialize};

use crate::loading::resolve::data_id::plugin_data_id;
use crate::marketplace::dependency_graph::InstalledRecord;
use crate::marketplace::install_scopes::InstallScope;

/// Why the data directory stayed. `other-scope` is the judgment that a plugin still installed somewhere keeps everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DataKeptBecause {
    /// `keep-data`.
    #[serde(rename = "keep-data")]
    KeepData,
    /// `shared-folder`.
    #[serde(rename = "shared-folder")]
    SharedFolder,
    /// `other-scope`.
    #[serde(rename = "other-scope")]
    OtherScope,
    /// `install_records_unreadable`.
    #[serde(rename = "install_records_unreadable")]
    InstallRecordsUnreadable,
}

/// "With `--json` the result carries `savedKept: \"install_records_unreadable\"`".
pub const SAVED_KEPT_UNREADABLE: &str = "install_records_unreadable";

/// The installed plugins read back after the removal, or `unreadable` when the list could not be
/// read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Remaining {
    /// The records.
    Records(Vec<InstalledRecord>),
    /// `unreadable`.
    Unreadable(String),
}

/// What the host read back.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionFacts {
    /// `name@marketplace` of the plugin just removed from `scope`.
    pub id: String,
    /// The scope it was removed from.
    pub scope: InstallScope,
    /// `--keep-data`.
    #[serde(default)]
    pub keep_data: Option<bool>,
    /// The installed plugins read back after the removal from `scope`; `'unreadable'` when the list could not be read.
    pub remaining: Remaining,
    /// Whether the plugins root's file system ignores letter case (spec §3.1.2 item 4, J3.19): two ids
    /// that differ only in case share one data directory exactly then. Default true, so a host that
    /// cannot tell keeps a folder rather than deleting one another plugin may use.
    #[serde(default)]
    pub case_insensitive_file_system: Option<bool>,
}

/// `delete` or `keep`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Keeping {
    /// `delete`.
    Delete,
    /// `keep`.
    Keep,
}

/// What an uninstall deletes and keeps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Retention {
    /// Its stored options and secrets.
    pub saved: Keeping,
    /// Its data directory.
    pub data: Keeping,
    /// Set when `saved` is `keep` because the records could not be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_kept: Option<&'static str>,
    /// Set when `data` is `keep`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_kept_because: Option<DataKeptBecause>,
    /// The folder name under `~/.claude/plugins/data/`.
    pub data_folder: String,
}

fn folded(id: &str, fold: bool) -> String {
    if fold {
        plugin_data_id(id).to_lowercase()
    } else {
        plugin_data_id(id)
    }
}

fn same_folder(a: &str, b: &str, fold: bool) -> bool {
    folded(a, fold) == folded(b, fold)
}

/// Why the data directory stays, in the order the docs list the exceptions; `None` when it goes.
fn kept_because(facts: &RetentionFacts, others: &[InstalledRecord]) -> Option<DataKeptBecause> {
    if others.iter().any(|record| record.id == facts.id) {
        return Some(DataKeptBecause::OtherScope);
    }
    if facts.keep_data == Some(true) {
        return Some(DataKeptBecause::KeepData);
    }
    let fold = facts.case_insensitive_file_system.unwrap_or(true);
    others
        .iter()
        .any(|record| same_folder(&record.id, &facts.id, fold))
        .then_some(DataKeptBecause::SharedFolder)
}

/// What the uninstall of `facts.id` from `facts.scope` deletes and keeps.
#[must_use]
pub fn uninstall_retention(facts: &RetentionFacts) -> Retention {
    let data_folder = plugin_data_id(&facts.id);
    let Remaining::Records(remaining) = &facts.remaining else {
        return Retention {
            saved: Keeping::Keep,
            data: Keeping::Keep,
            saved_kept: Some(SAVED_KEPT_UNREADABLE),
            data_kept_because: Some(DataKeptBecause::InstallRecordsUnreadable),
            data_folder,
        };
    };
    let because = kept_because(facts, remaining);
    let saved_stays = remaining.iter().any(|record| record.id == facts.id);
    Retention {
        saved: if saved_stays {
            Keeping::Keep
        } else {
            Keeping::Delete
        },
        data: if because.is_none() {
            Keeping::Delete
        } else {
            Keeping::Keep
        },
        saved_kept: None,
        data_kept_because: because,
        data_folder,
    }
}

/// `/plugin` "shows `· data preserved` when it did".
pub const DATA_PRESERVED: &str = "· data preserved";

crate::registry::exports! {
    "cli/uninstall-data" {
        "SAVED_KEPT_UNREADABLE" => ok || SAVED_KEPT_UNREADABLE;
        "uninstallRetention" => ok |facts: RetentionFacts| uninstall_retention(&facts);
        "DATA_PRESERVED" => ok || DATA_PRESERVED;
    }
}
