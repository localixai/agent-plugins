//! The core's counterpart of `src/ports/folder-scan.ts` at sha256:9e844f62c195, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! The folders a skills, commands or agents scan reads outside plugins: the scope folders
//! (`~/.claude`, the project's `.claude`, a managed folder) and the repository chain from the
//! working directory up.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::BoxFuture;
use super::files::PluginFiles;

/// One opened scope folder.
pub trait ScanFolder: Send + Sync {
    /// The folder as the scan named it.
    fn dir(&self) -> &str;
    /// Its files, read relative to it.
    fn files(&self) -> Arc<dyn PluginFiles>;
    /// A path inside it as a host path.
    fn host_path(&self, relative_path: &str) -> String;
    /// A path inside it with its links resolved, or `None` when it does not exist.
    fn canonical_path<'a>(&'a self, relative_path: &'a str) -> BoxFuture<'a, Option<String>>;
}

/// The directories from the working directory up to the repository root.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryChain {
    /// The working directory first, then each one above it up to the repository root.
    pub directories: Vec<String>,
    /// A worktree's main checkout, when the working directory is in a worktree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_checkout: Option<String>,
}

/// The host's view of the scope folders.
pub trait FolderScan: Send + Sync {
    /// The managed settings folder, when the host has one.
    fn managed_directory(&self) -> Option<String>;
    /// Opens a folder, or `None` when it does not exist.
    fn open<'a>(&'a self, dir: &'a str) -> BoxFuture<'a, Option<Arc<dyn ScanFolder>>>;
    /// The repository chain of `cwd`.
    fn repository<'a>(&'a self, cwd: &'a str) -> BoxFuture<'a, RepositoryChain>;
    /// The directories between `start` and `touched` (a file the session read), `start`
    /// excluded: where nested skills may be.
    fn directories_below<'a>(
        &'a self,
        start: &'a str,
        touched: &'a str,
    ) -> BoxFuture<'a, Vec<String>>;
}
