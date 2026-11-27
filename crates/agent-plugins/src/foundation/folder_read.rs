//! Port of `src/foundation/folder-read.ts`, switched.
//!
//! A file of a scope folder read through the host's port, for the skills and the agents scans.
//! A port may refuse a file by rejecting (over a size limit, text that is not UTF-8, a file that
//! changed while it was read): the scan skips that file with a warning that carries the port's
//! message and goes on. An aborted `signal` still rejects, so a cancelled scan never turns into a
//! quiet partial answer (judgment, spec/judgments.md, "metadata-first catalog").

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::ports::Signal;
use crate::ports::folder_scan::ScanFolder;
use crate::registry::{Port, catch_host};

/// A file's text, or why it was skipped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TextRead {
    /// `None` when the file is absent or the port refused it (then `issue` says why).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Why the port refused it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<PluginIssue>,
}

/// `path` of `folder` read; a refusal of the port's is a warning, unless `signal` aborted.
pub async fn read_text_of(
    folder: &dyn ScanFolder,
    path: &str,
    signal: Option<&Signal>,
) -> TextRead {
    let read = catch_host(async { folder.files().await.read_text(path, signal).await }).await;
    match read {
        Ok(text) => TextRead { text, issue: None },
        Err(cause) => {
            if signal.is_some_and(Signal::is_aborted) {
                return cause.rethrow().await;
            }
            let reason = cause.message;
            TextRead {
                text: None,
                issue: Some(warning(
                    &folder.host_path(path).await,
                    &format!("cannot be read: {reason}; skipped"),
                )),
            }
        }
    }
}

crate::registry::exports! {
    "foundation/folder-read" {
        "readTextOf" => async ok |folder: Port, path: String, signal: Option<Signal>|
            read_text_of(&folder, &path, signal.as_ref()).await;
    }
}
