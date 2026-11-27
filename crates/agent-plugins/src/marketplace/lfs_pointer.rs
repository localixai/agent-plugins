//! Port of `src/marketplace/lfs-pointer.ts`, switched.
//!
//! A Git LFS pointer file, which is what an LFS-tracked file is after a clone (plugins/host-
//! marketplace.md, "Keep plugin files out of Git LFS": "The clone never downloads LFS content, so
//! LFS-tracked files arrive as pointer files"). The docs do not describe the pointer's text; the
//! layout is Git LFS's own (a `version` line naming its spec, an `oid sha256:` line, a `size` line),
//! a judgment (spec/judgments.md). A pointer is a small text file, so a host reads at most
//! `MAX_POINTER_BYTES` of a file to ask. Git writes the lines with `\n`, and a checkout that converts
//! line ends leaves `\r\n`; both are read (judgment).

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::format::listing::{by_code_unit, list_sorted};
use crate::js::{Pattern, code_unit_length, is_whitespace, number, regex_test, split_line_breaks};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

const MAX_POINTER_BYTES: usize = 1024;

/// A Git LFS pointer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LfsPointer {
    /// The SHA-256 of the real content, lower case hex.
    pub oid: String,
    /// The size of the real content in bytes.
    pub size: f64,
}

/// `/^version https:\/\/git-lfs\.github\.com\/spec\/v\d+$/`.
static VERSION: Pattern =
    LazyLock::new(|| Regex::new(r"^version https://git-lfs\.github\.com/spec/v[0-9]+$").ok());
/// `/^oid sha256:([0-9a-f]{64})$/`.
static OID: Pattern = LazyLock::new(|| Regex::new(r"^oid sha256:([0-9a-f]{64})$").ok());
/// `/^size (\d+)$/`.
static SIZE: Pattern = LazyLock::new(|| Regex::new(r"^size ([0-9]+)$").ok());

/// The first capture group of `pattern` in `line`, if it matches.
fn captured<'a>(pattern: &Pattern, line: Option<&'a str>) -> Option<&'a str> {
    pattern
        .as_ref()?
        .captures(line.unwrap_or(""))?
        .get(1)
        .map(|found| found.as_str())
}

/// `text.trimEnd().split(/\r?\n/)`.
fn lines_of(text: &str) -> Vec<&str> {
    split_line_breaks(text.trim_end_matches(is_whitespace))
}

/// The pointer a text is, or `None` for any other text.
#[must_use]
pub fn read_lfs_pointer(text: &str) -> Option<LfsPointer> {
    if code_unit_length(text) > MAX_POINTER_BYTES {
        return None;
    }
    let lines = lines_of(text);
    let (version, oid, size) = (lines.first(), lines.get(1), lines.get(2));
    let digest = captured(&OID, oid.copied());
    let bytes = captured(&SIZE, size.copied());
    let well_formed = lines.len() <= 3 && regex_test(&VERSION, version.copied().unwrap_or(""));
    match (well_formed, digest, bytes) {
        (true, Some(digest), Some(bytes)) => Some(LfsPointer {
            oid: digest.to_owned(),
            size: number(bytes),
        }),
        _ => None,
    }
}

/// The warning for a plugin file that is a pointer: the plugin needs the real file and cannot get it.
#[must_use]
pub fn lfs_pointer_issue(path: &str) -> PluginIssue {
    warning(
        path,
        &format!(
            "{path} is a Git LFS pointer file, not the file itself: the clone never downloads LFS content. Keep plugin files out of Git LFS"
        ),
    )
}

fn child(directory: &str, name: &str) -> String {
    if directory == "." {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}

/// The warning for the file at `path` when it is a regular, small file that holds a pointer.
async fn pointer_warning(
    files: &dyn PluginFiles,
    path: &str,
    signal: Option<&Signal>,
) -> Option<PluginIssue> {
    let stat = files.stat(path, signal).await?;
    let small = usize::try_from(stat.size).is_ok_and(|size| size <= MAX_POINTER_BYTES);
    if !stat.is_file || stat.is_symlink || !small {
        return None;
    }
    let text = files.read_text(path, signal).await?;
    read_lfs_pointer(&text).map(|_| lfs_pointer_issue(path))
}

/// What a directory holds to look at.
struct Lookups {
    paths: Vec<String>,
    directories: Vec<String>,
}

/// The entries of `directory` to look at: files, and directories but `.git`, never through a link.
async fn lookups(files: &dyn PluginFiles, directory: &str, signal: Option<&Signal>) -> Lookups {
    let mut paths = Vec::new();
    let mut directories = Vec::new();
    for entry in list_sorted(files, directory, signal)
        .await
        .unwrap_or_default()
    {
        let path = child(directory, &entry.name);
        let stat = files.stat(&path, signal).await;
        let walked =
            stat.is_some_and(|stat| stat.is_directory && !stat.is_symlink) && entry.name != ".git";
        if walked {
            directories.push(path);
        } else {
            paths.push(path);
        }
    }
    Lookups { paths, directories }
}

/// The install check for a git-based plugin that was cloned: one warning per file of the plugin that
/// is a Git LFS pointer, since the clone never downloads LFS content and the plugin needs the real
/// file. Lists the plugin through its files (not into `.git`, not through links) and reads only
/// files small enough to be a pointer. Ordered by path.
pub async fn lfs_pointer_issues(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> Vec<PluginIssue> {
    let mut found: Vec<PluginIssue> = Vec::new();
    let mut pending = vec![".".to_owned()];
    while let Some(directory) = pending.pop() {
        let seen = lookups(files, &directory, signal).await;
        pending.extend(seen.directories);
        for path in &seen.paths {
            if let Some(issue) = pointer_warning(files, path, signal).await {
                found.push(issue);
            }
        }
    }
    found.sort_by(|a, b| by_code_unit(&a.path, &b.path));
    found
}

/// `{ signal }`, the options of `lfsPointerIssues`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LfsPointerOptions {
    /// Its cancellation.
    #[serde(default)]
    pub signal: Option<Signal>,
}

crate::registry::exports! {
    "marketplace/lfs-pointer" {
        "readLfsPointer" => ok |text: String| read_lfs_pointer(&text);
        "lfsPointerIssue" => ok |path: String| lfs_pointer_issue(&path);
        "lfsPointerIssues" => async ok |files: Port, options: Option<LfsPointerOptions>|
            lfs_pointer_issues(&files, options.unwrap_or_default().signal.as_ref()).await;
    }
}
