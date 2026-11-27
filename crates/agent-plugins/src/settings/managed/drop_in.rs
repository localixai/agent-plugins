//! Port of `src/settings/managed/drop-in.ts`, switched.
//!
//! A file-based policy split across teams (managed-settings.md, "Split a file-based policy across
//! teams"): `managed-settings.json` first, then every `*.json` of `managed-settings.d/` in
//! alphabetical order, "the later file's value replaces the earlier one". The host lists the folder
//! and reads each file; this module orders them, merges them and reports what is broken.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::issues::PluginIssue;
use crate::format::listing::by_code_unit;
use crate::js::{JsObject, JsValue, in_property_order};
use crate::registry::{Callback, Port};
use crate::settings::canonical::union_of;
use crate::settings::managed::admin_source::{
    AdminDocument, ManagedSource, SourceRead, UnreadableAt, UnreadableCause, UnreadableRead,
    harsher_cause, read_admin_document, unreadable_issue,
};
use crate::settings::merge::MERGES;

/// One file of the folder.
#[derive(Clone, Debug, Deserialize)]
pub struct DropInFile {
    /// Its name.
    pub name: String,
    /// What the host found in it.
    #[serde(default)]
    pub read: Option<SourceRead>,
}

/// The `managed-settings.d/` folder: present and unreadable, or its files (`None` where it is
/// read: absent).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum DropInDirectory {
    /// It exists and could not be listed.
    Unreadable(UnreadableRead),
    /// Its files.
    Files {
        /// The files.
        files: Vec<DropInFile>,
    },
}

/// What a report calls each file and the folder, as data: its path.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLabelNames {
    /// `managed-settings.json`'s.
    #[serde(default)]
    pub base: Option<String>,
    /// The folder's.
    #[serde(default)]
    pub directory: Option<String>,
}

/// The labels as the host gave them: data, or a record that holds `dropIn`, a function from a
/// file's name to its label, read through the port.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum FileLabel {
    /// The host's record with its `dropIn`.
    Host(Port),
    /// The labels as data.
    Data(FileLabelNames),
}

/// `managed-settings.json` and the folder beside it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedFileRead {
    /// `managed-settings.json`.
    #[serde(default)]
    pub base: Option<SourceRead>,
    /// The folder.
    #[serde(default)]
    pub drop_ins: Option<DropInDirectory>,
    /// What a report calls each file and the folder.
    #[serde(default)]
    pub label: Option<FileLabel>,
}

/// The labels read once: the names, and the host's function for a drop-in's.
struct Labels {
    names: FileLabelNames,
    drop_in: Option<Callback>,
}

async fn labels_of(label: Option<&FileLabel>) -> Labels {
    match label {
        None => Labels {
            names: FileLabelNames::default(),
            drop_in: None,
        },
        Some(FileLabel::Data(names)) => Labels {
            names: names.clone(),
            drop_in: None,
        },
        Some(FileLabel::Host(port)) => Labels {
            names: FileLabelNames {
                base: port.clone().member("base").await,
                directory: port.clone().member("directory").await,
            },
            drop_in: port.clone().member("dropIn").await,
        },
    }
}

/// "Claude Code ignores hidden files and files that don't end in `.json`."
#[must_use]
pub fn is_drop_in_name(name: &str) -> bool {
    name.ends_with(".json") && !name.starts_with('.')
}

/// The drop-in names in the order they merge: alphabetical, so numeric prefixes (`10-telemetry.json`,
/// `20-security.json`) control it. Plain code-unit order: the docs say "alphabetical" and no locale
/// (spec/judgments.md).
#[must_use]
pub fn drop_in_order(names: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = names
        .iter()
        .filter(|name| is_drop_in_name(name))
        .cloned()
        .collect();
    kept.sort_by(|a, b| by_code_unit(a, b));
    kept
}

/// Keys where "the later chain/lineup replaces the earlier one whole".
const REPLACED_WHOLE: [&str; 2] = ["fallbackModel", "modelPicker"];
/// Keys where "a later entry with the same name replaces the earlier one whole".
const REPLACED_BY_NAME: [&str; 2] = ["extraKnownMarketplaces", "managedMcpServers"];

fn member(part: &JsValue, key: &str) -> JsValue {
    part.get(key).cloned().unwrap_or_default()
}

fn later_set(parts: &[JsValue], key: &str) -> JsValue {
    parts
        .iter()
        .map(|part| member(part, key))
        .find(|value| !value.is_undefined())
        .unwrap_or_default()
}

/// A later file's value over an earlier one's: blocks merge key by key, lists concatenate in file
/// order with a later duplicate dropped (each entry stays where it first occurred, Judgment J14.1),
/// any other value replaces.
fn later_over(earlier: JsValue, later: &JsValue) -> JsValue {
    match (earlier, later) {
        (earlier, JsValue::Undefined) => earlier,
        (JsValue::Array(earlier), JsValue::Array(later)) => {
            JsValue::Array(union_of(&[earlier, later.clone()]))
        }
        (JsValue::Object(mut merged), JsValue::Object(later)) => {
            for (name, value) in later {
                let before = merged.get(name).cloned().unwrap_or_default();
                merged.insert(name.clone(), later_over(before, value));
            }
            JsValue::Object(in_property_order(merged))
        }
        (_, later) => later.clone(),
    }
}

/// The files merged in file order: `managed-settings.json`, then the drop-ins alphabetically.
fn in_file_order(parts: &[JsValue]) -> JsValue {
    parts
        .iter()
        .rev()
        .fold(JsValue::Object(JsObject::new()), later_over)
}

/// The files' settings as one, later files first in `parts`: single values replaced, lists combined
/// with duplicates removed, nested blocks merged key by key with each key under these rules, and the
/// keys above replaced whole.
fn merge_drop_ins(parts: &[JsValue]) -> JsValue {
    let JsValue::Object(mut merged) = in_file_order(parts) else {
        return JsValue::Object(JsObject::new());
    };
    let whole = REPLACED_WHOLE.map(|key| (key, later_set(parts, key)));
    let by_name = REPLACED_BY_NAME.map(|key| {
        let records: Vec<JsValue> = parts
            .iter()
            .map(|part| member(part, key))
            .filter(JsValue::is_object)
            .collect();
        let value = if records.is_empty() {
            JsValue::Undefined
        } else {
            (MERGES.per_key_highest_wins.combine)(&records)
        };
        (key, value)
    });
    for (key, value) in whole.into_iter().chain(by_name) {
        if !value.is_undefined() {
            merged.insert(key.to_owned(), value);
        }
    }
    JsValue::Object(in_property_order(merged))
}

/// The shapes the file source takes in `/status`: `(file)`, `(drop-ins)` or `(file + drop-ins)`.
pub const FILE_SHAPES: [&str; 3] = ["file", "drop-ins", "file + drop-ins"];

const fn shape_of(base: bool, drop_ins: bool) -> Option<&'static str> {
    match (base, drop_ins) {
        (true, false) => Some("file"),
        (false, true) => Some("drop-ins"),
        (true, true) => Some("file + drop-ins"),
        (false, false) => None,
    }
}

/// The read that decides the source's outcome: where it failed, why, and the cause.
#[derive(Clone, Debug)]
struct Failure {
    cause: UnreadableCause,
    label: String,
    reason: String,
}

struct Parts {
    documents: Vec<AdminDocument>,
    failure: Option<Failure>,
    issues: Vec<PluginIssue>,
}

/// The failure that counts when several parts fail: a cause that exits wins over one that starts the
/// session without the source (managed-settings.md, "Find entries Claude Code dropped": any other
/// failure exits), the first of equal causes otherwise (spec/judgments.md).
fn decisive(current: Option<Failure>, next: Failure) -> Failure {
    match current {
        Some(current) if harsher_cause(current.cause, next.cause) == current.cause => current,
        _ => next,
    }
}

const DIRECTORY_FAILED: &str = "Managed settings drop-in directory could not be read";

/// The drop-in directory that cannot be listed: the failure that counts and the issue to report.
fn directory_failure(directory: &UnreadableRead, label: Option<&str>) -> (Failure, PluginIssue) {
    (
        Failure {
            cause: directory.unreadable,
            label: label.unwrap_or("managed-settings.d").to_owned(),
            reason: directory.reason.clone(),
        },
        unreadable_issue(
            directory.unreadable,
            "managed:managed-settings.d",
            &format!("{DIRECTORY_FAILED}: {}", directory.reason),
        ),
    )
}

fn take(parts: &mut Parts, source: Option<&SourceRead>, label: Option<&str>) {
    let document = read_admin_document(ManagedSource::File, source, label);
    if let (Some(cause), Some(at)) = (document.unreadable, &document.unreadable_at) {
        let next = Failure {
            cause,
            label: at.label.clone(),
            reason: at.reason.clone(),
        };
        parts.failure = Some(decisive(parts.failure.take(), next));
    }
    parts.documents.push(document);
}

async fn read_parts(read: &ManagedFileRead, labels: &Labels) -> Parts {
    let mut parts = Parts {
        documents: Vec::new(),
        failure: None,
        issues: Vec::new(),
    };
    take(&mut parts, read.base.as_ref(), labels.names.base.as_deref());
    match &read.drop_ins {
        Some(DropInDirectory::Unreadable(directory)) => {
            let (failed, issue) = directory_failure(directory, labels.names.directory.as_deref());
            parts.failure = Some(decisive(parts.failure.take(), failed));
            parts.issues.push(issue);
        }
        Some(DropInDirectory::Files { files }) => {
            let by_name: IndexMap<&str, Option<&SourceRead>> = files
                .iter()
                .map(|file| (file.name.as_str(), file.read.as_ref()))
                .collect();
            let names: Vec<String> = by_name.keys().map(|name| (*name).to_owned()).collect();
            for name in drop_in_order(&names) {
                let label: Option<String> = match &labels.drop_in {
                    Some(drop_in) => {
                        drop_in
                            .clone()
                            .call(vec![JsValue::from(name.as_str())])
                            .await
                    }
                    None => None,
                };
                let source = by_name.get(name.as_str()).copied().flatten();
                take(&mut parts, source, label.as_deref());
            }
        }
        None => {}
    }
    parts
}

/// The file source as one document: the base then the drop-ins merged; a part that is not a JSON
/// object fails the whole source (its issue names that part); a part that cannot be read leaves the
/// source present.
pub async fn read_file_document(read: &ManagedFileRead) -> AdminDocument {
    let labels = labels_of(read.label.as_ref()).await;
    let Parts {
        documents,
        failure,
        issues,
    } = read_parts(read, &labels).await;
    let broken = documents.iter().any(|document| document.broken);
    let raws: Vec<JsValue> = documents
        .iter()
        .filter(|document| !document.broken)
        .map(|document| JsValue::Object(document.raw.clone()))
        .rev()
        .collect();
    let merged = SourceRead::Value {
        value: merge_drop_ins(&raws),
    };
    let document = read_admin_document(
        ManagedSource::File,
        Some(&merged),
        labels.names.base.as_deref(),
    );
    let shape = shape_of(read.base.is_some(), documents.len() > 1);
    let mut all: Vec<PluginIssue> = documents.into_iter().flat_map(|part| part.issues).collect();
    all.extend(issues);
    all.extend(document.issues.iter().cloned());
    AdminDocument {
        present: document.present || failure.is_some() || broken,
        broken,
        unreadable: failure
            .as_ref()
            .map(|failure| failure.cause)
            .or(document.unreadable),
        unreadable_at: failure
            .map(|failure| UnreadableAt {
                label: failure.label,
                reason: failure.reason,
            })
            .or_else(|| document.unreadable_at.clone()),
        shape: shape.map(str::to_owned).or_else(|| document.shape.clone()),
        issues: all,
        ..document
    }
}

crate::registry::exports! {
    "settings/managed/drop-in" {
        "isDropInName" => ok |name: String| is_drop_in_name(&name);
        "dropInOrder" => ok |names: Vec<String>| drop_in_order(&names);
        "FILE_SHAPES" => ok || FILE_SHAPES;
        "readFileDocument" => async ok |read: ManagedFileRead| read_file_document(&read).await;
    }
}
