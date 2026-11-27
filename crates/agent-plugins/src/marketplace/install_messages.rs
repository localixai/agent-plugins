//! Port of `src/marketplace/install-messages.ts`, switched.
//!
//! The exact texts of the install and loading problems the docs quote (plugins/troubleshooting.md,
//! plugins/loading.md, plugins/cli-reference.md): pure builders, each named for the heading it
//! answers. The join between a quoted message and a quoted hint or fix, which the docs do not show,
//! is a row in spec/judgments.md.

use serde::{Deserialize, Serialize};

use super::store_layout::{StoreEntry, classify_store_entry};
use crate::js::json_stringify::json_stringify;
use crate::js::value::to_js_value;
use crate::js::{JsValue, number_to_string};
use crate::registry::Port;

fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---- installed_plugins.json ----

const OPTIONS: &str = "Update Claude Code with \"claude update\". If you cannot update, uninstall the plugin with the version of Claude Code that wrote the record. If neither helps, delete the record from installed_plugins.json by hand, then restart Claude Code or run /reload-plugins.";

/// "`installed_plugins.json holds a record under "<id>" that this version of Claude Code cannot
/// read`"; with several, "holds records under" (troubleshooting.md). The options after it are the
/// three steps of that section, in order.
#[must_use]
pub fn unreadable_records_message(ids: &[String]) -> String {
    let subject = if ids.len() == 1 {
        "a record"
    } else {
        "records"
    };
    format!(
        "installed_plugins.json holds {subject} under {} that this version of Claude Code cannot read. {OPTIONS}",
        quoted(ids)
    )
}

/// "The whole file declares a format this version doesn't know" (troubleshooting.md).
#[must_use]
pub fn unknown_format_message(version: f64) -> String {
    format!(
        "installed_plugins.json is in a format (version {}) that this version of Claude Code does not know",
        number_to_string(version)
    )
}

// ---- the commands that refuse, and the notes `claude plugin list` prints ----

/// The three commands that refuse while a record cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefusingCommand {
    /// `claude plugin install`.
    Install,
    /// `claude plugin uninstall`.
    Uninstall,
    /// `claude plugin update`.
    Update,
}

impl RefusingCommand {
    /// The command's name, as its result names it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Update => "update",
        }
    }

    /// The word its refusal uses.
    const fn refused(self) -> &'static str {
        match self {
            Self::Install => "installed",
            Self::Uninstall => "uninstalled",
            Self::Update => "updated",
        }
    }
}

/// `failureCode` of `--json` on those three commands when a record cannot be read.
pub const RECORDS_UNREADABLE_CODE: &str = "install_records_unreadable";

/// A refusal of those three commands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordsRefusal {
    /// `Plugin "<name>" was not installed:` (or `uninstalled`, `updated`) and the same text
    /// `claude plugin list` prints.
    pub message: String,
    /// Always [`RECORDS_UNREADABLE_CODE`].
    pub failure_code: String,
}

/// How `claude plugin install`, `uninstall` and `update` refuse (troubleshooting.md): `Plugin
/// "<name>" was not installed:`, `was not uninstalled:` or `was not updated:`, followed by the
/// unreadable-record text; with `--json` the result carries that `message` and `failureCode`.
#[must_use]
pub fn refuse_for_unreadable_records(
    command: RefusingCommand,
    name: &str,
    reason: &str,
) -> RecordsRefusal {
    RecordsRefusal {
        message: format!("Plugin \"{name}\" was not {}: {reason}", command.refused()),
        failure_code: RECORDS_UNREADABLE_CODE.to_owned(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultLine<'a> {
    command: RefusingCommand,
    outcome: &'static str,
    message: &'a str,
    failure_code: &'a str,
}

/// The last line of stdout of `claude plugin install|uninstall|update --json` for that refusal:
/// "one JSON object" with `command`, `outcome` and `message`, and `failureCode` where it applies
/// (cli-reference.md, "JSON result format"; troubleshooting.md: "the result line carries the same
/// `message` and `failureCode`").
#[must_use]
pub fn refusal_result_line(command: RefusingCommand, refusal: &RecordsRefusal) -> String {
    let line = ResultLine {
        command,
        outcome: "failed",
        message: &refusal.message,
        failure_code: &refusal.failure_code,
    };
    to_js_value(&line)
        .ok()
        .and_then(|value| json_stringify(&value))
        .unwrap_or_default()
}

/// `claude plugin list` prints the unreadable-record text as a `Note:`.
#[must_use]
pub fn list_note(text: &str) -> String {
    format!("Note: {text}")
}

/// The note naming a dated copy that sits beside `installed_plugins.json`. Judgment
/// (spec/judgments.md): the words around the quoted heading and the path.
#[must_use]
pub fn rebuilt_note(kept_path: &str) -> String {
    list_note(&format!(
        "installed_plugins.json could not be read and was rebuilt. The old contents are kept in {kept_path}."
    ))
}

/// "The note ends `Nothing needs doing about these copies.`" (troubleshooting.md).
#[must_use]
pub fn set_aside_note(copy_path: &str) -> String {
    list_note(&format!(
        "install records under names that no version of Claude Code can use were removed from installed_plugins.json. A copy of them is kept in {copy_path}. Nothing needs doing about these copies."
    ))
}

fn copy_note(entry: StoreEntry) -> Option<fn(&str) -> String> {
    match entry {
        StoreEntry::UnreadableCopy => Some(rebuilt_note),
        StoreEntry::SetAsideCopy => Some(set_aside_note),
        StoreEntry::Cache
        | StoreEntry::Data
        | StoreEntry::Marketplaces
        | StoreEntry::Synced
        | StoreEntry::Trash
        | StoreEntry::InstalledPlugins
        | StoreEntry::KnownMarketplaces
        | StoreEntry::KnownMarketplacesClaudeAi
        | StoreEntry::FlaggedPlugins
        | StoreEntry::Unknown => None,
    }
}

/// The notes `claude plugin list` prints "for as long as" a dated copy "sits beside
/// `installed_plugins.json`" (troubleshooting.md): one per `.unreadable` copy and one per
/// `.set-aside` copy found among `names`, the files directly under the plugins root. `layout` is
/// the store's (`StoreLayout`), as the host holds it: its `file` names each copy's path.
pub async fn recovery_notes(names: &[String], layout: &Port) -> Vec<String> {
    let mut notes = vec![];
    for name in names {
        if let Some(note) = copy_note(classify_store_entry(name)) {
            let path = layout
                .method::<String>("file", vec![JsValue::from(name.as_str())])
                .await;
            notes.push(note(&path));
        }
    }
    notes
}

crate::registry::exports! {
    "marketplace/install-messages" {
        "unreadableRecordsMessage" => ok |ids: Vec<String>| unreadable_records_message(&ids);
        "unknownFormatMessage" => ok |version: f64| unknown_format_message(version);
        "RECORDS_UNREADABLE_CODE" => ok || RECORDS_UNREADABLE_CODE;
        "refuseForUnreadableRecords" => ok |command: RefusingCommand, name: String, reason: String|
            refuse_for_unreadable_records(command, &name, &reason);
        "refusalResultLine" => ok |command: RefusingCommand, refusal: RecordsRefusal|
            refusal_result_line(command, &refusal);
        "listNote" => ok |text: String| list_note(&text);
        "rebuiltNote" => ok |kept_path: String| rebuilt_note(&kept_path);
        "setAsideNote" => ok |copy_path: String| set_aside_note(&copy_path);
        "recoveryNotes" => async ok |names: Vec<String>, layout: Port| recovery_notes(&names, &layout).await;
    }
}
