//! Port of `src/foundation/folder-walk.ts`, switched.
//!
//! The walk of a scope folder, shared by the skills and the agents scans: the entries of a directory
//! with symbolic links followed, and the files of a tree. A link to a directory elsewhere on disk is
//! followed (skills.md, "Choose where skills load", "Symlinked folders"); one that dangles is skipped
//! with a warning, and a tree is walked at most `MAX_DEPTH` levels deep and each real directory once,
//! so a link loop ends (judgment, spec/judgments.md).

use std::collections::HashSet;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::format::listing::list_sorted;
use crate::js::promise_all;
use crate::ports::folder_scan::ScanFolder;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// The deepest level a walk goes.
pub const MAX_DEPTH: usize = 16;

/// One entry of a directory, classified through its link.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Child {
    /// The entry's name.
    pub name: String,
    /// Relative to the folder opened.
    pub path: String,
    /// A regular file, through its link.
    pub is_file: bool,
    /// A directory, through its link.
    pub is_directory: bool,
    /// Where the entry really is; absent only when the host could not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// What a walk found and the issues it met.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Walked<T> {
    /// What the walk found.
    pub found: Vec<T>,
    /// What it met on the way.
    pub issues: Vec<PluginIssue>,
}

fn join(directory: &str, name: &str) -> String {
    if directory == "." {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}

/// One entry after its check: the child, or the issue that skipped it.
enum Checked {
    Issue(PluginIssue),
    Child(Child),
}

/// The entries of `directory`, sorted by name, each classified through its link.
pub fn list_children<'a>(
    folder: &'a dyn ScanFolder,
    directory: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Walked<Child>> {
    Box::pin(async move {
        let files = folder.files().await;
        let Some(entries) = list_sorted(files.as_ref(), directory, signal).await else {
            return Walked {
                found: Vec::new(),
                issues: Vec::new(),
            };
        };
        let files = &files;
        let checked = promise_all(
            entries
                .iter()
                .map(|entry| -> BoxFuture<'_, Checked> {
                    Box::pin(async move {
                        let path = join(directory, &entry.name);
                        let stat = files.stat(&path, signal).await;
                        let Some(stat) = stat.filter(|one| one.is_file || one.is_directory) else {
                            return Checked::Issue(warning(
                                &folder.host_path(&path).await,
                                "a symbolic link that does not lead to a file or folder; skipped",
                            ));
                        };
                        let canonical = folder.canonical_path(&path).await;
                        Checked::Child(Child {
                            name: entry.name.clone(),
                            path,
                            is_file: stat.is_file,
                            is_directory: stat.is_directory,
                            canonical,
                        })
                    })
                })
                .collect(),
        )
        .await;
        let mut walked = Walked {
            found: Vec::new(),
            issues: Vec::new(),
        };
        for each in checked {
            match each {
                Checked::Child(child) => walked.found.push(child),
                Checked::Issue(issue) => walked.issues.push(issue),
            }
        }
        walked
    })
}

/// A file found below the walked directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkedFile {
    /// Relative to the folder opened.
    pub path: String,
    /// The names below the walked directory: `frontend`, `component.md`.
    pub segments: Vec<String>,
    /// Where the file really is, when the host could say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// The state one walk shares: what it looks for and the real directories already entered.
struct Walk<'a> {
    folder: &'a dyn ScanFolder,
    extension: String,
    signal: Option<&'a Signal>,
    seen: Mutex<HashSet<String>>,
}

impl Walk<'_> {
    /// Whether `key` is a real directory not entered yet; it is entered by the call.
    fn enter(&self, key: &str) -> bool {
        let mut seen = self
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        seen.insert(key.to_owned())
    }
}

fn visit<'a>(
    walk: &'a Walk<'a>,
    directory: String,
    prefix: Vec<String>,
    depth: usize,
) -> BoxFuture<'a, Walked<WalkedFile>> {
    Box::pin(async move {
        let listed = list_children(walk.folder, &directory, walk.signal).await;
        let prefix = &prefix;
        let parts = promise_all(
            listed
                .found
                .iter()
                .map(|child| -> BoxFuture<'_, Walked<WalkedFile>> {
                    Box::pin(async move {
                        let mut segments = prefix.clone();
                        segments.push(child.name.clone());
                        if child.is_file {
                            let matches = child.name.to_lowercase().ends_with(&walk.extension);
                            return Walked {
                                found: if matches {
                                    vec![WalkedFile {
                                        path: child.path.clone(),
                                        segments,
                                        canonical: child.canonical.clone(),
                                    }]
                                } else {
                                    Vec::new()
                                },
                                issues: Vec::new(),
                            };
                        }
                        let key = child.canonical.as_deref().unwrap_or(&child.path);
                        if depth >= MAX_DEPTH || !walk.enter(key) {
                            return Walked {
                                found: Vec::new(),
                                issues: Vec::new(),
                            };
                        }
                        visit(walk, child.path.clone(), segments, depth + 1).await
                    })
                })
                .collect(),
        )
        .await;
        let mut walked = Walked {
            found: Vec::new(),
            issues: listed.issues,
        };
        for part in parts {
            walked.found.extend(part.found);
            walked.issues.extend(part.issues);
        }
        walked
    })
}

/// What `walk_files` takes besides the folder and the root.
#[derive(Clone, Debug, Deserialize)]
pub struct WalkOptions {
    /// The extension looked for (`.md`, case ignored).
    pub extension: String,
    /// Ends the walk early.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The files below `root` with the extension (`.md`, case ignored), at any depth, sorted by path.
pub fn walk_files<'a>(
    folder: &'a dyn ScanFolder,
    root: &'a str,
    options: &'a WalkOptions,
) -> BoxFuture<'a, Walked<WalkedFile>> {
    Box::pin(async move {
        let walk = Walk {
            folder,
            extension: options.extension.to_lowercase(),
            signal: options.signal.as_ref(),
            seen: Mutex::new(HashSet::new()),
        };
        if let Some(start) = folder.canonical_path(root).await {
            walk.enter(&start);
        }
        visit(&walk, root.to_owned(), Vec::new(), 0).await
    })
}

crate::registry::exports! {
    "foundation/folder-walk" {
        "MAX_DEPTH" => ok || MAX_DEPTH;
        "listChildren" => async ok |folder: Port, directory: String, signal: Option<Signal>|
            list_children(&folder, &directory, signal.as_ref()).await;
        "walkFiles" => async ok |folder: Port, root: String, options: WalkOptions|
            walk_files(&folder, &root, &options).await;
    }
}
