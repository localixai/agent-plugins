//! Port of `src/loading/core/locations.ts`, switched.
//!
//! A fixed location of a core package (core §6): absent is no error (§6.2); present but of the wrong
//! filesystem kind, outside the plugin root (§4.1, boundary 2) or not checkable makes that component
//! type invalid while the others load. Containment is `locate`'s, the one owner of it in the kit.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::locate::{LocateContext, LocateOptions, locate, locate_context};
use crate::format::read_json::{JsonRead, read_json};
use crate::js::JsValue;
use crate::ports::Signal;
use crate::ports::files::{FileStat, PluginFiles};
use crate::registry::Port;

/// Where a fixed location stands.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FixedLocation {
    /// It is not there.
    Absent {
        /// Always true.
        absent: bool,
    },
    /// It disables its component type.
    Issue {
        /// Why.
        issue: PluginIssue,
    },
    /// It is there, of the right kind.
    Present {
        /// Always true.
        present: bool,
    },
}

/// The context every read of a core package's fixed locations shares.
#[must_use]
pub fn core_locate(files: Port, signal: Option<Signal>) -> LocateContext {
    locate_context(
        files,
        LocateOptions {
            signal,
            ..LocateOptions::default()
        },
    )
}

/// The kind a fixed location must be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
}

/// What each expected kind is called.
const fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::File => "a regular file",
        Kind::Directory => "a directory",
    }
}

/// How a stat shows each expected kind.
const fn is(kind: Kind, stat: &FileStat) -> bool {
    match kind {
        Kind::File => stat.is_file,
        Kind::Directory => stat.is_directory,
    }
}

/// Where a fixed location stands, for a component whose location must be a `file` or a `directory`.
pub async fn fixed_location(context: &LocateContext, path: &str, kind: Kind) -> FixedLocation {
    let found = locate(context, path, path, Some(true)).await;
    if let Some(issue) = found.issue {
        return FixedLocation::Issue { issue };
    }
    let Some(stat) = found.stat else {
        let other = context.files.stat(path, context.signal.as_ref()).await;
        return if other.is_none() {
            FixedLocation::Absent { absent: true }
        } else {
            FixedLocation::Issue {
                issue: wrong_kind(path, kind),
            }
        };
    };
    if is(kind, &stat) {
        FixedLocation::Present { present: true }
    } else {
        FixedLocation::Issue {
            issue: wrong_kind(path, kind),
        }
    }
}

fn wrong_kind(path: &str, kind: Kind) -> PluginIssue {
    error(
        path,
        &format!(
            "{path} is not {}: the component type is invalid (core §6.2)",
            label(kind)
        ),
    )
}

/// A fixed JSON file's parsed value, or the issue that disables its component type.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FixedJson {
    /// It is not there.
    Absent {
        /// Always true.
        absent: bool,
    },
    /// It disables its component type.
    Issue {
        /// Why.
        issue: PluginIssue,
    },
    /// Its value.
    Value {
        /// What it holds.
        value: JsValue,
    },
}

/// A fixed JSON file's parsed value, or the issue that disables its component type.
pub async fn fixed_json(context: &LocateContext, path: &str, verdict: &str) -> FixedJson {
    match fixed_location(context, path, Kind::File).await {
        FixedLocation::Absent { absent } => return FixedJson::Absent { absent },
        FixedLocation::Issue { issue } => return FixedJson::Issue { issue },
        FixedLocation::Present { .. } => {}
    }
    match read_json(&context.files, path, context.signal.as_ref()).await {
        JsonRead::Problem { problem, .. } => FixedJson::Issue {
            issue: error(
                path,
                &format!("{verdict}: {path} is not valid JSON: {problem}"),
            ),
        },
        JsonRead::Value { value } => FixedJson::Value { value },
    }
}

crate::registry::exports! {
    "loading/core/locations" {
        "coreLocate" => ok |files: Port, signal: Option<Signal>| core_locate(files, signal).to_js();
    }
}
