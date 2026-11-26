//! The core's counterpart of `src/ports/files.ts` at sha256:43aaee1e6f01, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Read access to files (docs/API.md, "Ports"). `PluginFiles` reads one plugin folder: its paths
//! are relative to the folder, use `/` and never leave it; `.` is the folder itself.
//! `TextFileReader` reads wherever the host says: its paths are host paths (absolute, built from
//! `Locations`, such as `~/.claude/agent-memory/<agent>/`), and a `PluginFiles` satisfies it only
//! with its own relative paths. A path that does not exist is `None`, never an error: absence is
//! an ordinary answer, a plugin is allowed to omit a file. A path the system could not check is a
//! [`FileStat`] with `check_failed`, the one error channel of the port.

use serde::{Deserialize, Serialize};

use super::BoxFuture;

/// What a host knows about one entry; flags, so no code compares a kind to a string.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    /// A regular file.
    pub is_file: bool,
    /// A directory.
    pub is_directory: bool,
    /// A symbolic link (the entry itself, not its target).
    pub is_symlink: bool,
    /// Its size in bytes.
    pub size: u64,
    /// Where a symlink really points: relative to the folder when the target is inside it, an
    /// absolute host path (or a `..`-relative one) when it is not; absent for a regular entry. It
    /// is reported for the entry asked about only, whether or not the target exists: the crate
    /// stats each folder above a path to see a link the path crosses, so a host must not hide an
    /// outside link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub real_path: Option<String>,
    /// The operating system's error code when the path could not be checked for a reason other
    /// than "not found" (`ELOOP`, `EIO`, `ESTALE`, `EACCES`, on the path or a folder above it);
    /// `is_file` and `is_directory` are then false. A host that cannot tell answers `None` as for
    /// a missing path. The crate refuses what such a path names and reports
    /// `<component> path could not be checked: <path> (<code>)` (errors.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_failed: Option<String>,
}

/// One entry of a directory listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    /// The entry's name, without its folder.
    pub name: String,
    /// A regular file.
    pub is_file: bool,
    /// A directory.
    pub is_directory: bool,
}

/// The narrowest read: a text file. Which paths it takes depends on who holds it: root-relative
/// for a [`PluginFiles`], host paths from `Locations` for a reader of an agent memory folder.
pub trait TextFileReader: Send + Sync {
    /// The file's text decoded as UTF-8, or `None` when there is no such file.
    fn read_text<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Option<String>>;
}

/// One plugin folder, read through paths relative to it.
pub trait PluginFiles: TextFileReader {
    /// The file's bytes, or `None` when there is no such file.
    fn read_bytes<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Option<Vec<u8>>>;
    /// Entries of a directory in any order (the crate sorts them), or `None` when it does not
    /// exist.
    fn list<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>>;
    /// What the host knows about the entry, or `None` when there is none.
    fn stat<'a>(&'a self, path: &'a str) -> BoxFuture<'a, Option<FileStat>>;
    /// Whether there is an entry at `path`.
    fn exists<'a>(&'a self, path: &'a str) -> BoxFuture<'a, bool>;
}
