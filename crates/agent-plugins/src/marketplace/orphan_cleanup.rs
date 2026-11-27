//! Port of `src/marketplace/orphan-cleanup.ts`, switched.
//!
//! Cleanup of previous versions (plugins/loading.md): "When you update or uninstall a plugin,
//! Claude Code writes an `.orphaned_at` marker into the previous version directory. It removes
//! that directory in a background cleanup 14 days later, so a session that already loaded the old
//! version keeps running. The sweep runs only while `installed_plugins.json` records at least one
//! install." Pure: which directories to mark and which to remove; the host writes the marker,
//! reads its time and deletes. Judgments (spec/judgments.md): a directory is swept at exactly 14
//! days; a directory a remaining record points at is neither marked nor swept, since it is in use
//! again; only a version directory of the cache, exactly `cache/<marketplace>/<plugin>/<version>`,
//! is marked or swept: a plugin loaded in place points at its own source directory, and a sweep of
//! it, of the cache or of a marketplace folder would delete what is not an old version.

use indexmap::IndexSet;
use serde::Deserialize;

use super::installed_plugins::InstallRecord;
use crate::format::config_directory::Join;
use crate::format::path_text::without_trailing_slashes;
use crate::registry::{Callback, Port};

/// The marker's file name.
pub const ORPHANED_MARKER: &str = ".orphaned_at";

/// 14 days.
pub const ORPHAN_GRACE_MS: f64 = 14.0 * 24.0 * 60.0 * 60.0 * 1000.0;

/// The marker's path in a version directory.
pub async fn orphan_marker_path(version_dir: &str, join: &dyn Join) -> String {
    join.join(version_dir, ORPHANED_MARKER).await
}

fn slash(path: &str) -> String {
    without_trailing_slashes(&path.replace('\\', "/")).to_owned()
}

/// `<marketplace>/<plugin>/<version>`: three names, none empty, `.` or `..`.
const VERSION_DIR_SEGMENTS: usize = 3;
fn is_name(segment: &str) -> bool {
    !segment.is_empty() && segment != "." && segment != ".."
}

/// Whether a directory is a version directory of the plugin cache, `cache/<marketplace>/<plugin>/
/// <version>/`, written with no `.` or `..` segment: only those are marked, and later swept, as
/// orphans. The cache itself, a marketplace or plugin folder, a folder below a version and a path
/// that would need resolving are not (judgment, spec/judgments.md). `layout` is the host's
/// `StoreLayout`: its `cache()` names the cache.
async fn is_cached_version_dir(layout: &Port, dir: &str) -> bool {
    let cache = layout.method::<String>("cache", vec![]).await;
    let prefix = format!("{}/", slash(&cache));
    let path = slash(dir);
    if !path.starts_with(&prefix) {
        return false;
    }
    let rest = path.get(prefix.len()..).unwrap_or_default();
    let segments: Vec<&str> = rest.split('/').collect();
    segments.len() == VERSION_DIR_SEGMENTS && segments.iter().all(|segment| is_name(segment))
}

/// The version directories an update or uninstall leaves behind: those of `removed` inside the
/// cache that no `remaining` record uses.
pub async fn dirs_to_orphan(
    removed: &[InstallRecord],
    remaining: &[InstallRecord],
    layout: &Port,
) -> Vec<String> {
    let in_use: IndexSet<&str> = remaining
        .iter()
        .map(|one| one.install_path.as_str())
        .collect();
    let dirs: IndexSet<&str> = removed
        .iter()
        .map(|one| one.install_path.as_str())
        .collect();
    let mut orphaned = Vec::new();
    for dir in dirs {
        if !in_use.contains(dir) && is_cached_version_dir(layout, dir).await {
            orphaned.push(dir.to_owned());
        }
    }
    orphaned
}

/// A version directory with its marker.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Orphan {
    /// The directory.
    pub dir: String,
    /// When the marker was written, in Unix milliseconds.
    pub orphaned_at_ms: f64,
}

/// What a sweep reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanSweepInput {
    /// Every install `installed_plugins.json` still records.
    pub remaining: Vec<InstallRecord>,
    /// The marked directories.
    pub orphans: Vec<Orphan>,
    /// Now, in Unix milliseconds.
    pub now_ms: f64,
    /// The host's `StoreLayout`.
    pub layout: Port,
}

/// The directories the background cleanup removes now.
pub async fn orphan_sweep(input: &OrphanSweepInput) -> Vec<String> {
    if input.remaining.is_empty() {
        return Vec::new();
    }
    let in_use: IndexSet<&str> = input
        .remaining
        .iter()
        .map(|one| one.install_path.as_str())
        .collect();
    let mut swept = Vec::new();
    for one in &input.orphans {
        if input.now_ms - one.orphaned_at_ms >= ORPHAN_GRACE_MS
            && !in_use.contains(one.dir.as_str())
            && is_cached_version_dir(&input.layout, &one.dir).await
        {
            swept.push(one.dir.clone());
        }
    }
    swept
}

crate::registry::exports! {
    "marketplace/orphan-cleanup" {
        "ORPHANED_MARKER" => ok || ORPHANED_MARKER;
        "ORPHAN_GRACE_MS" => ok || ORPHAN_GRACE_MS;
        "orphanMarkerPath" => async ok |version_dir: String, join: Callback|
            orphan_marker_path(&version_dir, &join).await;
        "dirsToOrphan" => async ok |removed: Vec<InstallRecord>, remaining: Vec<InstallRecord>, layout: Port|
            dirs_to_orphan(&removed, &remaining, &layout).await;
        "orphanSweep" => async ok |input: OrphanSweepInput| orphan_sweep(&input).await;
    }
}
