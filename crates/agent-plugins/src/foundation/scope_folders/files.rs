//! The files of one scope folder, from `src/foundation/scope-folders.ts`: the files below a base
//! with their text, and the folder a scope watches.

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::foundation::folder_read::read_text_of;
use crate::foundation::folder_walk::{WalkOptions, Walked, WalkedFile, list_children, walk_files};
use crate::js::promise_all;
use crate::ports::folder_scan::ScanFolder;
use crate::ports::{BoxFuture, Signal};

/// One file of a scope folder, with its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeFile {
    /// Relative to the base opened.
    pub path: String,
    /// The names below the folder: `frontend`, `component.md`.
    pub segments: Vec<String>,
    /// Where the file really lives, when the host could say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
    /// The path as the host names it, for messages and records.
    pub host_path: String,
    /// The file's text.
    pub text: String,
}

/// The files of a scope folder and what was met reading them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeFiles {
    /// The files read.
    pub files: Vec<ScopeFile>,
    /// A link that leads nowhere, a file the port refused.
    pub issues: Vec<PluginIssue>,
}

/// How deep `read_scope_files` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Depth {
    /// Every depth ("recursively, so you can organize definitions into subfolders").
    Tree,
    /// The folder alone.
    Flat,
}

/// What `read_scope_files` takes besides the base and the container.
#[derive(Clone, Debug, Deserialize)]
pub struct ScopeFilesOptions {
    /// The file extension, case ignored: `.md`.
    pub extension: String,
    /// `tree`: every depth; `flat`: the folder alone.
    pub depth: Depth,
    /// Ends the read early.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The files below `container` each depth names, without their text (`LISTERS[depth]`).
async fn lister(
    base: &dyn ScanFolder,
    container: &str,
    options: &ScopeFilesOptions,
) -> Walked<WalkedFile> {
    let ScopeFilesOptions {
        extension, signal, ..
    } = options;
    match options.depth {
        Depth::Tree => {
            let walk = WalkOptions {
                extension: extension.clone(),
                signal: signal.clone(),
            };
            walk_files(base, container, &walk).await
        }
        Depth::Flat => {
            let children = list_children(base, container, signal.as_ref()).await;
            let wanted = extension.to_lowercase();
            let found = children
                .found
                .into_iter()
                .filter(|child| child.is_file && child.name.to_lowercase().ends_with(&wanted))
                .map(|child| WalkedFile {
                    path: child.path,
                    segments: vec![child.name],
                    canonical: child.canonical,
                })
                .collect();
            Walked {
                found,
                issues: children.issues,
            }
        }
    }
}

/// The files of one folder below an opened base, with their text. A file the port refuses is
/// skipped with a warning (`read_text_of`); an aborted signal still rejects.
pub async fn read_scope_files(
    base: &dyn ScanFolder,
    container: &str,
    options: &ScopeFilesOptions,
) -> ScopeFiles {
    let walked = lister(base, container, options).await;
    let reads = promise_all(
        walked
            .found
            .iter()
            .map(|file| -> BoxFuture<'_, _> {
                Box::pin(read_text_of(base, &file.path, options.signal.as_ref()))
            })
            .collect(),
    )
    .await;
    let mut files = Vec::new();
    let mut issues = walked.issues.clone();
    for (file, read) in walked.found.into_iter().zip(&reads) {
        if let Some(text) = &read.text {
            let host_path = base.host_path(&file.path).await;
            files.push(ScopeFile {
                path: file.path,
                segments: file.segments,
                canonical: file.canonical,
                host_path,
                text: text.clone(),
            });
        }
    }
    issues.extend(reads.into_iter().filter_map(|read| read.issue));
    ScopeFiles { files, issues }
}

/// What `watched_folder` takes besides the base and the container.
#[derive(Clone, Debug, Deserialize)]
pub struct WatchOptions {
    /// Whether the scope watches the folder.
    pub watched: bool,
    /// Ends the check early.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The folder to watch below an opened base: its host path when the scope watches it and it
/// exists.
pub async fn watched_folder(
    base: &dyn ScanFolder,
    container: &str,
    options: &WatchOptions,
) -> Vec<String> {
    if !options.watched {
        return Vec::new();
    }
    if base
        .files()
        .await
        .exists(container, options.signal.as_ref())
        .await
    {
        vec![base.host_path(container).await]
    } else {
        Vec::new()
    }
}
