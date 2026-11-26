//! Port of `src/loading/validate/components.ts`, switched.
//!
//! The Markdown components `claude plugin validate` reads (oracle 2.1.319): each skill folder's
//! `SKILL.md`, and every `.md` below `agents/` and `commands/` at any depth. Paths the manifest
//! declares are not read for frontmatter. A link is never followed: the folder or entry is skipped,
//! and a skills folder says so.

use serde::{Deserialize, Serialize};

use super::links::{FolderStyle, linked_entries, linked_folder, unread_components};
use super::paths::{child_of, entries_of, plugin_path};
use crate::format::validate::component_frontmatter::{
    COMPONENT_KINDS, ComponentKind, ComponentPlace, frontmatter_findings,
};
use crate::format::validate::finding::{ValidationIssue, in_file};
use crate::js::JsValue;
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

const SKILL_FILE: &str = "SKILL.md";

fn is_markdown(name: &str) -> bool {
    name.to_lowercase().ends_with(".md")
}

/// The strings of a manifest key: one path, or the paths of a list.
#[must_use]
pub fn manifest_paths(value: &JsValue) -> Vec<String> {
    if let Some(path) = value.as_str() {
        return vec![plugin_path(path)];
    }
    value.as_array().map_or_else(Vec::new, |entries| {
        entries
            .iter()
            .filter_map(JsValue::as_str)
            .map(plugin_path)
            .collect()
    })
}

/// What a scan of one folder found: the files to read and what it has to say about links.
#[derive(Clone, Debug, Serialize)]
pub struct Scan {
    /// The files to read.
    pub files: Vec<String>,
    /// What the scan has to say about links.
    pub issues: Vec<ValidationIssue>,
}

/// Whether `path` is itself a link.
async fn is_symlink(files: &dyn PluginFiles, path: &str, signal: Option<&Signal>) -> bool {
    files
        .stat(path, signal)
        .await
        .is_some_and(|stat| stat.is_symlink)
}

/// The `SKILL.md` of each skill folder directly below `dir`, and the links among them.
pub async fn scan_skill_folders(
    files: &dyn PluginFiles,
    dir: &str,
    style: FolderStyle,
    signal: Option<&Signal>,
) -> Scan {
    if is_symlink(files, dir, signal).await {
        return Scan {
            files: Vec::new(),
            issues: vec![linked_folder(dir)],
        };
    }
    let mut found: Vec<String> = Vec::new();
    let mut links = 0.0_f64;
    let mut unread = 0.0_f64;
    for entry in entries_of(files, dir, signal).await {
        let path = child_of(dir, &entry.name);
        if is_symlink(files, &path, signal).await {
            links += 1.0;
            continue;
        }
        if !entry.is_directory {
            continue;
        }
        let Some(skill) = files.stat(&child_of(&path, SKILL_FILE), signal).await else {
            continue;
        };
        if skill.is_file && !skill.is_symlink {
            found.push(child_of(&path, SKILL_FILE));
        } else {
            unread += 1.0;
        }
    }
    let mut issues = linked_entries(dir, links, style);
    issues.extend(unread_components(dir, unread, style));
    Scan {
        files: found,
        issues,
    }
}

/// The Markdown files below a folder at any depth, links left alone; `style` decides whether they are counted.
pub fn scan_markdown_folder<'a>(
    files: &'a dyn PluginFiles,
    dir: &'a str,
    style: FolderStyle,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Scan> {
    Box::pin(async move {
        if is_symlink(files, dir, signal).await {
            return Scan {
                files: Vec::new(),
                issues: vec![linked_folder(dir)],
            };
        }
        let mut found: Vec<String> = Vec::new();
        let mut links = 0.0_f64;
        for entry in entries_of(files, dir, signal).await {
            let path = child_of(dir, &entry.name);
            if is_symlink(files, &path, signal).await {
                links += 1.0;
                continue;
            }
            if entry.is_file && is_markdown(&entry.name) {
                found.push(path.clone());
            }
            if entry.is_directory {
                found.extend(
                    scan_markdown_folder(files, &path, style, signal)
                        .await
                        .files,
                );
            }
        }
        let issues = if style == FolderStyle::Project {
            linked_entries(dir, links, style)
        } else {
            Vec::new()
        };
        Scan {
            files: found,
            issues,
        }
    })
}

const fn default_dir(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Skill => "skills",
        ComponentKind::Agent => "agents",
        ComponentKind::Command => "commands",
    }
}

/// The scan of the default folder of one kind: skill folders, or Markdown files at any depth.
async fn scan_default(
    kind: ComponentKind,
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> Scan {
    if kind == ComponentKind::Skill {
        scan_skill_folders(files, default_dir(kind), FolderStyle::Plugin, signal).await
    } else {
        scan_markdown_folder(files, default_dir(kind), FolderStyle::Plugin, signal).await
    }
}

/// What `frontmatterIssues` is given besides the files.
#[derive(Debug, Default, Deserialize)]
pub struct FrontmatterOptions {
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// Where the files sit; a plugin's folder by default.
    #[serde(default)]
    pub place: Option<ComponentPlace>,
}

/// The frontmatter findings of the files of a scan.
pub async fn frontmatter_issues(
    kind: ComponentKind,
    files: &dyn PluginFiles,
    paths: &[String],
    signal: Option<&Signal>,
    place: ComponentPlace,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for path in paths {
        if let Some(text) = files.read_text(path, signal).await {
            issues.extend(in_file(path, &frontmatter_findings(kind, &text, place), ""));
        }
    }
    issues
}

/// Link warnings and frontmatter findings of every skill, agent and command file, kind by kind.
pub async fn component_issues(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for kind in COMPONENT_KINDS {
        let scan = scan_default(kind, files, signal).await;
        issues.extend(scan.issues);
        issues.extend(
            frontmatter_issues(kind, files, &scan.files, signal, ComponentPlace::Plugin).await,
        );
    }
    issues
}

crate::registry::exports! {
    "loading/validate/components" {
        "manifestPaths" => ok |value: JsValue| manifest_paths(&value);
        "scanSkillFolders" => async ok |files: Port, dir: String, style: FolderStyle, signal: Option<Signal>|
            scan_skill_folders(&files, &dir, style, signal.as_ref()).await;
        "scanMarkdownFolder" => async ok |files: Port, dir: String, style: FolderStyle, signal: Option<Signal>|
            scan_markdown_folder(&files, &dir, style, signal.as_ref()).await;
        "frontmatterIssues" => async ok |kind: ComponentKind, files: Port, paths: Vec<String>, options: Option<FrontmatterOptions>| {
            let options = options.unwrap_or_default();
            frontmatter_issues(kind, &files, &paths, options.signal.as_ref(), options.place.unwrap_or_default()).await
        };
        "componentIssues" => async ok |files: Port, signal: Option<Signal>|
            component_issues(&files, signal.as_ref()).await;
    }
}
