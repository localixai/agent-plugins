//! Port of `src/marketplace/archive-scan.ts`, switched.
//!
//! What the kit reads out of a zip's entry list without unzipping anything (plugins/host-
//! marketplace.md, "Stay within the download limits for hosted files"; plugins/marketplace-
//! reference.md, "archive plugin source"): the totals the extraction limits judge, and the names
//! and links that must not leave the plugin directory. Pure over `ArchiveReader.scan`.
//!
//! Judgments (spec/judgments.md): the docs name no zip-slip or link rule for an archive, so an entry
//! whose path would leave the folder it is extracted into, and a symbolic link whose target leaves
//! the plugin directory (followed through the other links in the zip), and an entry written through
//! a link, are refused whole (an archive has no marketplace to dereference into, and
//! "Outside the marketplace: the symlink is skipped for security" cannot skip an entry the host is
//! about to extract). A link is found by its name as a file system that folds names would compare
//! it, and a link that shares its name with another entry is refused: on such a file system `a/x`
//! is written through a link stored as `A`.

use std::collections::{HashMap, VecDeque};

use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::{
    PathPlatform, escapes_as_written, is_absolute_path, without_trailing_slashes,
};
use crate::js::normalize::{NormalForm, normalize};
use crate::ports::marketplace::{ArchiveEntry, ArchiveScan, ArchiveStats};

/// `path.split(/[\\/]/).filter((part) => part !== '')`.
fn parts(path: &str) -> Vec<&str> {
    path.split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect()
}

/// The totals of a scan: derived, never reported by the host, so the limits cannot be argued with.
#[must_use]
pub fn archive_stats(scan: &ArchiveScan, zip_bytes: u64) -> ArchiveStats {
    let sizes = scan.entries.iter().map(|entry| entry.size);
    let mut top_level: Vec<String> = Vec::new();
    for name in scan
        .entries
        .iter()
        .filter_map(|entry| parts(&entry.path).first().copied())
    {
        if !top_level.iter().any(|seen| seen == name) {
            top_level.push(name.to_owned());
        }
    }
    ArchiveStats {
        entries: u64::try_from(scan.entries.len()).unwrap_or(u64::MAX),
        largest_file_bytes: sizes.clone().fold(0, u64::max),
        uncompressed_bytes: sizes.fold(0, u64::saturating_add),
        compressed_bytes: zip_bytes,
        top_level,
    }
}

fn leaves(path: &str) -> bool {
    path.contains('\0') || escapes_as_written(path, PathPlatform::Windows)
}

/// The most links one path may pass through before it is taken as a loop (Linux's `MAXSYMLINKS`).
const MAX_LINK_HOPS: u32 = 40;

/// A name as the least discriminating file system a host may extract onto compares it: without
/// regard to letter case or Unicode normalization (APFS, NTFS), nor to trailing dots and spaces
/// (Windows). Two names a file system does tell apart may fold alike; that only refuses more.
/// The case and normalization tables are Unicode 18's, where Bun's are 17's (docs/RUST.md, "Where
/// the crate answers differently"): a character Unicode 18 added may fold where Bun keeps it.
fn fold_name(name: &str) -> String {
    let folded = normalize(
        &normalize(name, NormalForm::Nfd)
            .to_uppercase()
            .to_lowercase(),
        NormalForm::Nfd,
    );
    folded.trim_end_matches(['.', ' ']).to_owned()
}

/// The key two paths share when some file system takes them for one file.
fn key_of(folders: &[&str]) -> String {
    folders
        .iter()
        .map(|folder| fold_name(folder))
        .collect::<Vec<_>>()
        .join("/")
}

struct Link<'a> {
    /// The link's path as the archive stores it, for a message.
    path: &'a str,
    target: &'a str,
}

/// Every link entry by its key, and every entry's path under its key.
struct Names<'a> {
    links: HashMap<String, Link<'a>>,
    paths: HashMap<String, Vec<&'a str>>,
}

fn names_of(scan: &ArchiveScan) -> Names<'_> {
    let mut links = HashMap::new();
    let mut paths: HashMap<String, Vec<&str>> = HashMap::new();
    for entry in &scan.entries {
        let key = key_of(&parts(&entry.path));
        paths.entry(key.clone()).or_default().push(&entry.path);
        if let Some(target) = entry
            .link_target
            .as_deref()
            .filter(|target| entry.is_symlink && !target.is_empty())
        {
            links.insert(
                key,
                Link {
                    path: &entry.path,
                    target,
                },
            );
        }
    }
    Names { links, paths }
}

struct Walk<'a> {
    folders: Vec<&'a str>,
    pending: VecDeque<&'a str>,
    hops: u32,
}

/// One step of a walk: `false` when it climbs above the top, loops or meets an absolute link.
fn step<'a>(walk: &mut Walk<'a>, part: &'a str, names: &Names<'a>) -> bool {
    if part == "." {
        return true;
    }
    if part == ".." {
        return walk.folders.pop().is_some();
    }
    walk.folders.push(part);
    let Some(target) = names
        .links
        .get(&key_of(&walk.folders))
        .map(|link| link.target)
    else {
        return true;
    };
    walk.hops += 1;
    if walk.hops > MAX_LINK_HOPS || is_absolute_path(target) {
        return false;
    }
    // A link is replaced by its target, which is read from the folder the link is in.
    walk.folders.pop();
    for part in parts(target).into_iter().rev() {
        walk.pending.push_front(part);
    }
    true
}

/// Where a link at `from` ends up, as the folders below the zip's top: `None` when it climbs
/// above the top. Only the entry list is read: a link the path passes through is replaced by its
/// own target, as the file system would, so a chain of links each inside cannot climb out.
fn resolve_link<'a>(from: &'a str, target: &'a str, names: &Names<'a>) -> Option<Vec<&'a str>> {
    let mut folders = parts(from);
    folders.pop();
    let mut walk = Walk {
        folders,
        pending: parts(target).into_iter().collect(),
        hops: 0,
    };
    while let Some(part) = walk.pending.pop_front() {
        if !step(&mut walk, part, names) {
            return None;
        }
    }
    Some(walk.folders)
}

fn starts_with(path: &[&str], prefix: &[&str]) -> bool {
    prefix
        .iter()
        .enumerate()
        .all(|(index, part)| path.get(index) == Some(part))
}

fn link_issue(entry: &ArchiveEntry, root: &[&str], names: &Names<'_>) -> Option<PluginIssue> {
    let named = format!("The archive entry \"{}\" is a symbolic link", entry.path);
    let target = match entry.link_target.as_deref() {
        None | Some("") => return Some(error("source", &format!("{named} with no target"))),
        Some(target) => target,
    };
    let landed = if is_absolute_path(target) {
        None
    } else {
        resolve_link(&entry.path, target, names)
    };
    if landed.is_some_and(|landed| starts_with(&landed, root)) {
        return None;
    }
    Some(error(
        "source",
        &format!("{named} to \"{target}\", which leaves the plugin directory"),
    ))
}

/// The link an entry would be written through: a folder above it that is a link entry.
fn link_above<'a>(entry: &ArchiveEntry, names: &Names<'a>) -> Option<&'a str> {
    let folders = parts(&entry.path);
    (1..folders.len()).find_map(|depth| {
        let above = folders.get(..depth)?;
        names.links.get(&key_of(above)).map(|link| link.path)
    })
}

/// The other entry an entry shares its file with, when one of the two is a link.
fn same_file<'a>(entry: &ArchiveEntry, names: &Names<'a>) -> Option<&'a str> {
    let key = key_of(&parts(&entry.path));
    let sharing = names.paths.get(&key).map_or(&[][..], Vec::as_slice);
    if sharing.len() < 2 || !names.links.contains_key(&key) {
        return None;
    }
    // The first of the others: the entry's own path is listed once for each entry that stores it.
    let first_is_own = sharing.first() == Some(&entry.path.as_str());
    sharing.get(usize::from(first_is_own)).copied()
}

/// What is wrong with one entry, in the order judged: its path, a link above it, a link it shares
/// its name with, its own link.
fn entry_issue(entry: &ArchiveEntry, root: &[&str], names: &Names<'_>) -> Option<PluginIssue> {
    if leaves(&entry.path) {
        return Some(error(
            "source",
            &format!(
                "The archive entry \"{}\" would be extracted outside the plugin directory",
                entry.path
            ),
        ));
    }
    if let Some(through) = link_above(entry, names) {
        return Some(error(
            "source",
            &format!(
                "The archive entry \"{}\" is written through the symbolic link \"{through}\"",
                entry.path
            ),
        ));
    }
    if let Some(other) = same_file(entry, names) {
        return Some(error(
            "source",
            &format!(
                "The archive entry \"{}\" names the same file as \"{other}\", and one of them is a symbolic link",
                entry.path
            ),
        ));
    }
    if entry.is_symlink {
        link_issue(entry, root, names)
    } else {
        None
    }
}

/// The first entry that would be written outside the plugin directory, is written through a link,
/// shares its name with a link, or is a link that leaves it.
#[must_use]
pub fn unsafe_entry_issue(scan: &ArchiveScan, root: &str) -> Option<PluginIssue> {
    let root_parts = parts(without_trailing_slashes(root));
    let names = names_of(scan);
    scan.entries
        .iter()
        .find_map(|entry| entry_issue(entry, &root_parts, &names))
}

crate::registry::exports! {
    "marketplace/archive-scan" {
        "archiveStats" => ok |scan: ArchiveScan, zip_bytes: u64| archive_stats(&scan, zip_bytes);
        "unsafeEntryIssue" => ok |scan: ArchiveScan, root: String| unsafe_entry_issue(&scan, &root);
    }
}
