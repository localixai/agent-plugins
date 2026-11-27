//! Ported from `src/marketplace/command-directory.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a `command` source's printed directory must hold and stay within (plugins/marketplace-
//! reference.md, "Output that fails the install or update", "Copy mode and link mode"), judged
//! through a read-only view rooted at that directory.

use super::plugin_content::PLUGIN_CONTENT_DIRECTORIES;
use crate::format::issues::{PluginIssue, error};
use crate::format::listing::list_sorted;
use crate::format::path_text::{PathPlatform, leaves_folder};
use crate::js::number_to_string;
use crate::ports::Signal;
use crate::ports::files::PluginFiles;

const MIB: u64 = 1024 * 1024;

/// "In copy mode, the directory is larger than 256 MiB or has more than 20,000 entries."
const MAX_BYTES: u64 = 256 * MIB;
const MAX_ENTRIES: u64 = 20_000;

/// The limits of a copy (`CopyLimits`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyLimits {
    /// The most bytes.
    pub max_bytes: f64,
    /// The most entries.
    pub max_entries: f64,
}

/// The two ways a directory is too large to copy: the verdict, once a limit is broken.
#[must_use]
pub fn too_many_entries(max_entries: f64) -> PluginIssue {
    error(
        "source",
        &format!(
            "The printed directory has more than {} entries, too many to copy",
            number_to_string(max_entries)
        ),
    )
}

/// `256 MiB` for a whole number of MiB (the docs' limit), else the bytes.
fn size_text(bytes: f64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a mebibyte is far below 2^53, as JavaScript's number holds it"
    )]
    let mib = MIB as f64;
    if bytes % mib == 0.0 {
        format!("{} MiB", number_to_string(bytes / mib))
    } else {
        format!("{} bytes", number_to_string(bytes))
    }
}

/// The verdict for a directory over the byte limit.
#[must_use]
pub fn too_large(max_bytes: f64) -> PluginIssue {
    error(
        "source",
        &format!(
            "The printed directory is larger than {}, too large to copy",
            size_text(max_bytes)
        ),
    )
}

fn problem(message: &str) -> PluginIssue {
    error("source", message)
}

/// The printed directory holds plugin content at its top level.
pub async fn content_issue(files: &dyn PluginFiles, signal: &Signal) -> Option<PluginIssue> {
    let top = list_sorted(files, ".", Some(signal))
        .await
        .unwrap_or_default();
    let has = top.iter().any(|entry| {
        entry.is_directory && PLUGIN_CONTENT_DIRECTORIES.contains(&entry.name.as_str())
    });
    if has {
        None
    } else {
        Some(problem(
            "The printed directory has no plugin content at its top level (a .claude-plugin/ directory, or skills/, commands/, agents/ or hooks/)",
        ))
    }
}

struct Tally {
    entries: u64,
    bytes: u64,
    pending: Vec<String>,
}

fn child(directory: &str, name: &str) -> String {
    if directory == "." {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}

/// Counts one entry; the verdict when it breaks a limit. Symlinks count as one entry and are not followed.
async fn count_entry(
    files: &dyn PluginFiles,
    path: &str,
    tally: &mut Tally,
    signal: &Signal,
) -> Option<PluginIssue> {
    tally.entries += 1;
    if tally.entries > MAX_ENTRIES {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the entry limit is far below 2^53, as JavaScript's number holds it"
        )]
        return Some(too_many_entries(MAX_ENTRIES as f64));
    }
    let stat = files.stat(path, Some(signal)).await?;
    if stat.is_symlink {
        return None;
    }
    if stat.is_directory {
        tally.pending.push(path.to_owned());
    }
    tally.bytes = tally
        .bytes
        .saturating_add(if stat.is_file { stat.size } else { 0 });
    if tally.bytes > MAX_BYTES {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the byte limit is far below 2^53, as JavaScript's number holds it"
        )]
        return Some(too_large(MAX_BYTES as f64));
    }
    None
}

/// In copy mode: the directory is at most 256 MiB with at most 20,000 entries.
pub async fn copy_limit_issue(files: &dyn PluginFiles, signal: &Signal) -> Option<PluginIssue> {
    let mut tally = Tally {
        entries: 0,
        bytes: 0,
        pending: vec![".".to_owned()],
    };
    while let Some(directory) = tally.pending.pop() {
        for entry in list_sorted(files, &directory, Some(signal))
            .await
            .unwrap_or_default()
        {
            let issue =
                count_entry(files, &child(&directory, &entry.name), &mut tally, signal).await;
            if issue.is_some() {
                return issue;
            }
        }
    }
    None
}

/// In link mode: "the install fails if a top-level entry is a symlink that points outside the printed directory".
pub async fn link_issue(files: &dyn PluginFiles, signal: &Signal) -> Option<PluginIssue> {
    for entry in list_sorted(files, ".", Some(signal))
        .await
        .unwrap_or_default()
    {
        let stat = files.stat(&entry.name, Some(signal)).await;
        if stat.as_ref().is_some_and(|stat| {
            stat.is_symlink
                && leaves_folder(
                    stat.real_path.as_deref().unwrap_or(""),
                    PathPlatform::Windows,
                )
        }) {
            return Some(problem(&format!(
                "The top-level entry \"{}\" is a symlink that points outside the printed directory",
                entry.name
            )));
        }
    }
    None
}
