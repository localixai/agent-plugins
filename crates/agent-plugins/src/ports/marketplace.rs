//! The core's counterpart of `src/ports/marketplace.ts` at sha256:c46e731cf570, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! What installing from a marketplace needs of the host: hashing, reading an archive, listing a
//! repository's tags and keeping the install records.

use serde::{Deserialize, Serialize};

use super::{BoxFuture, PortFailure, Signal};

/// SHA-256 of bytes.
pub trait Sha256: Send + Sync {
    /// The lower-case hex digest of `bytes`.
    fn hex<'a>(&'a self, bytes: &'a [u8]) -> BoxFuture<'a, String>;
}

/// The totals of an archive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveStats {
    /// How many entries it holds.
    pub entries: u64,
    /// The size of its largest file.
    pub largest_file_bytes: u64,
    /// Its size unpacked.
    pub uncompressed_bytes: u64,
    /// Its size packed.
    pub compressed_bytes: u64,
    /// The names at its top level.
    pub top_level: Vec<String>,
}

/// One entry of an archive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    /// Its path inside the archive.
    pub path: String,
    /// Its size unpacked.
    pub size: u64,
    /// Whether it is a symbolic link.
    pub is_symlink: bool,
    /// Where the link points, for a link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
}

/// Every entry of an archive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveScan {
    /// The entries, in the archive's order.
    pub entries: Vec<ArchiveEntry>,
}

/// Reads an archive's entries without unpacking it. A failure's kind is `corrupt` or `aborted`.
pub trait ArchiveReader: Send + Sync {
    /// Scans the archive.
    fn scan<'a>(
        &'a self,
        bytes: &'a [u8],
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<ArchiveScan, PortFailure>>;
}

/// A git repository, as a marketplace source names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GitRepository {
    /// `owner/repo` on GitHub.
    Shorthand {
        /// The `owner/repo` text.
        shorthand: String,
    },
    /// Any git URL.
    Url {
        /// The URL.
        url: String,
    },
}

/// Lists a repository's tags. A failure's kind is `connection`, `timeout` or `aborted`.
pub trait GitTags: Send + Sync {
    /// The repository's tags.
    fn list<'a>(
        &'a self,
        repository: &'a GitRepository,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<Vec<String>, PortFailure>>;
}

/// The install records (`installed_plugins.json`, `known_marketplaces.json`), kept by the host.
/// A failure's kind is `write`, `remove` or `aborted`.
pub trait PluginStateStore: Send + Sync {
    /// A record's text, or `None` when there is none.
    fn read<'a>(&'a self, name: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<String>>;
    /// Writes a record.
    fn write<'a>(
        &'a self,
        name: &'a str,
        text: &'a str,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<(), PortFailure>>;
    /// The names of the records there are.
    fn names<'a>(&'a self, signal: &'a Signal) -> BoxFuture<'a, Vec<String>>;
    /// When a record was last written, in milliseconds since the Unix epoch.
    fn modified_at<'a>(&'a self, name: &'a str, signal: &'a Signal) -> BoxFuture<'a, Option<f64>>;
    /// Removes a record.
    fn remove<'a>(
        &'a self,
        name: &'a str,
        signal: &'a Signal,
    ) -> BoxFuture<'a, Result<(), PortFailure>>;
}
