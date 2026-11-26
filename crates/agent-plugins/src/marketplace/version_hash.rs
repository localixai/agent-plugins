//! Port of `src/marketplace/version-hash.ts` at sha256:7499b30a1fa0.
//!
//! What a plugin version is hashed from (plugins/loading.md, "How Claude Code computes the version";
//! plugins/marketplace-reference.md, "Copy mode and link mode"): the files a `command` source copied,
//! the real path and top-level entries of a linked directory, and the subdirectory path of a
//! `git-subdir` source. The docs say what the hash covers, not how its input is laid out, so the
//! layouts below are judgments (spec/judgments.md). Every listing is sorted here by code unit, never
//! trusted from the port, so a digest never depends on the order a folder lists its entries in.
//!
//! Not switched: every export but `shortHash12` takes the host's `Sha256` port, which the boundary
//! has no port for yet, so the package runs the TypeScript and the core uses this port.

use crate::format::listing::list_sorted;
use crate::format::path_text::{child_path, without_trailing_slashes};
use crate::js::slice_code_units;
use crate::ports::files::PluginFiles;
use crate::ports::marketplace::Sha256;
use crate::ports::{BoxFuture, Signal};

/// "The commit SHA of the source, shortened to 12 characters": a hex digest cut to 12, lower case.
#[must_use]
pub fn short_hash_12(hex: &str) -> String {
    slice_code_units(&hex.to_lowercase(), 0, Some(12))
}

const SUBDIRECTORY_HASH_LENGTH: usize = 8;

async fn digest_of(text: &str, sha256: &dyn Sha256) -> String {
    sha256.hex(text.as_bytes()).await
}

/// The names directly under `directory`, sorted by code unit.
async fn names_in(
    files: &dyn PluginFiles,
    directory: &str,
    signal: Option<&Signal>,
) -> Vec<String> {
    list_sorted(files, directory, signal)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

/// The lines one entry contributes: a link by where it points (it is never followed, so a link out
/// of the folder cannot pull foreign bytes in), a directory by what is below it, a file by the digest
/// of its bytes. An empty directory contributes nothing.
fn entry_lines<'a>(
    files: &'a dyn PluginFiles,
    path: &'a str,
    sha256: &'a dyn Sha256,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Vec<String>> {
    Box::pin(async move {
        let Some(stat) = files.stat(path, signal).await else {
            return Vec::new();
        };
        if stat.is_symlink {
            return vec![format!("{path}\0->{}", stat.real_path.unwrap_or_default())];
        }
        if stat.is_directory {
            return tree_lines(files, path, sha256, signal).await;
        }
        match files.read_bytes(path, signal).await {
            None => Vec::new(),
            Some(bytes) => vec![format!("{path}\0{}", sha256.hex(&bytes).await)],
        }
    })
}

fn tree_lines<'a>(
    files: &'a dyn PluginFiles,
    directory: &'a str,
    sha256: &'a dyn Sha256,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Vec<String>> {
    Box::pin(async move {
        let mut lines = Vec::new();
        for name in names_in(files, directory, signal).await {
            let path = child_path(directory, &name);
            lines.extend(entry_lines(files, &path, sha256, signal).await);
        }
        lines
    })
}

/// The version of a `copy`-mode `command` source: "a hash of the copied files", 12 characters. The
/// input is one line per file or link in path order (`<path> NUL <digest of its bytes>`, or `<path>
/// NUL -> <link target>`), so the same files give the same hash ("a re-run that produces identical
/// files counts as up to date") and moving content between paths changes it.
pub async fn copy_mode_hash(
    files: &dyn PluginFiles,
    sha256: &dyn Sha256,
    signal: Option<&Signal>,
) -> String {
    let lines = tree_lines(files, ".", sha256, signal).await;
    short_hash_12(&digest_of(&lines.join("\n"), sha256).await)
}

/// The version of a `link`-mode `command` source: "the printed directory's real path and its
/// top-level entries, not ... the files inside them". The input is the real path and the top-level
/// names in order; nothing is opened, so a link-mode plugin of any size has a version.
pub async fn link_mode_hash(
    real_path: &str,
    files: &dyn PluginFiles,
    sha256: &dyn Sha256,
    signal: Option<&Signal>,
) -> String {
    let names = names_in(files, ".", signal).await;
    let mut parts = vec![real_path.to_owned()];
    parts.extend(names);
    short_hash_12(&digest_of(&parts.join("\0"), sha256).await)
}

/// The hash a `git-subdir` version carries ("a hash of the subdirectory path"): the first 8 hex
/// digits of the SHA-256 of the path, written without a leading `./` or a trailing `/` so the two
/// spellings of one directory give one version.
pub async fn subdirectory_path_hash(path: &str, sha256: &dyn Sha256) -> String {
    let mut rest = path;
    while let Some(after) = rest.strip_prefix("./") {
        rest = after;
    }
    let normalized = without_trailing_slashes(rest);
    slice_code_units(
        &digest_of(normalized, sha256).await,
        0,
        Some(SUBDIRECTORY_HASH_LENGTH),
    )
}

crate::registry::exports! {
    "marketplace/version-hash" {
        "shortHash12" => ok |hex: String| short_hash_12(&hex);
    }
}
