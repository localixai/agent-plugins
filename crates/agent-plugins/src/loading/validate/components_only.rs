//! Port of `src/loading/validate/components-only.ts`, switched.
//!
//! A folder with no manifest still validates when it holds components Claude Code loads without one
//! (oracle 2.1.319): a skills tree (`skills/<name>/SKILL.md`), a project's `.claude/skills`,
//! `.claude/commands` or `.claude/agents`, or a folder that is itself named `skills`, `commands` or
//! `agents`. Anything else has no manifest to find.

use serde::Deserialize;

use super::components::{
    Scan, component_issues, frontmatter_issues, scan_markdown_folder, scan_skill_folders,
};
use super::links::FolderStyle;
use super::paths::{child_of, entries_of};
use crate::format::validate::component_frontmatter::{ComponentKind, ComponentPlace};
use crate::format::validate::finding::ValidationIssue;
use crate::js::promise_all;
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

const KINDS: [ComponentKind; 3] = [
    ComponentKind::Skill,
    ComponentKind::Agent,
    ComponentKind::Command,
];

const fn folder_of(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Skill => "skills",
        ComponentKind::Agent => "agents",
        ComponentKind::Command => "commands",
    }
}

/// Agents of a bare folder named `agents` are only looked at for links, not for frontmatter (oracle
/// 2.1.319); those of a project's `.claude/agents/` are read, with their own unparsed-YAML wording
/// and the description warning (spec §2.11 table, J2.4, oracle 2.1.319 and 2.1.322).
const fn read_frontmatter(style: FolderStyle, kind: ComponentKind) -> bool {
    match (style, kind) {
        (FolderStyle::Plugin, ComponentKind::Skill | ComponentKind::Command)
        | (FolderStyle::Project, _) => true,
        (FolderStyle::Plugin, ComponentKind::Agent) => false,
    }
}

async fn has_skills_tree(files: &dyn PluginFiles, signal: Option<&Signal>) -> bool {
    for entry in entries_of(files, "skills", signal).await {
        let skill = child_of(&child_of("skills", &entry.name), "SKILL.md");
        if entry.is_directory && files.exists(&skill, signal).await {
            return true;
        }
    }
    false
}

/// The last segment of what the report calls the folder.
fn base_name(label: &str) -> String {
    label
        .trim_end_matches('/')
        .split(['\\', '/'])
        .next_back()
        .unwrap_or_default()
        .to_owned()
}

/// Where a bare folder is scanned, what the report calls it, and whose folder it is.
struct Where<'a> {
    dir: &'a str,
    report: &'a str,
    style: FolderStyle,
}

async fn scan_bare(
    kind: ComponentKind,
    files: &dyn PluginFiles,
    place: &Where<'_>,
    signal: Option<&Signal>,
) -> Scan {
    if kind == ComponentKind::Skill {
        scan_skill_folders(files, place.dir, place.style, signal).await
    } else {
        scan_markdown_folder(files, place.dir, place.style, signal).await
    }
}

const fn component_place(style: FolderStyle) -> ComponentPlace {
    match style {
        FolderStyle::Plugin => ComponentPlace::Plugin,
        FolderStyle::Project => ComponentPlace::Project,
    }
}

async fn bare_issues(
    kind: ComponentKind,
    files: &dyn PluginFiles,
    place: &Where<'_>,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let scan = scan_bare(kind, files, place, signal).await;
    if !read_frontmatter(place.style, kind) {
        return scan.issues;
    }
    let shown: Vec<String> = scan
        .files
        .iter()
        .map(|path| {
            if place.report.is_empty() {
                path.clone()
            } else {
                format!("{}/{path}", place.report)
            }
        })
        .collect();
    let found = frontmatter_issues(
        kind,
        files,
        &scan.files,
        signal,
        component_place(place.style),
    )
    .await;
    let mut issues = scan.issues;
    issues.extend(found.into_iter().enumerate().map(|(index, issue)| {
        ValidationIssue {
            file: shown
                .get(index)
                .cloned()
                .unwrap_or_else(|| issue.file.clone()),
            ..issue
        }
    }));
    issues
}

/// What `componentsOnly` is given.
#[derive(Debug, Deserialize)]
pub struct BareInput {
    /// The folder.
    pub files: Port,
    /// What the report calls the folder.
    pub label: String,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The components of a folder with no manifest, or `None` when it holds none the validator knows.
pub async fn components_only(
    files: &dyn PluginFiles,
    label: &str,
    signal: Option<&Signal>,
) -> Option<Vec<ValidationIssue>> {
    let tree = has_skills_tree(files, signal).await;
    let checks: Vec<BoxFuture<'_, Option<ComponentKind>>> = KINDS
        .into_iter()
        .map(|kind| -> BoxFuture<'_, Option<ComponentKind>> {
            Box::pin(async move {
                let path = child_of(".claude", folder_of(kind));
                files.exists(&path, signal).await.then_some(kind)
            })
        })
        .collect();
    let project: Vec<ComponentKind> = promise_all(checks).await.into_iter().flatten().collect();
    let name = base_name(label);
    let own: Vec<ComponentKind> = KINDS
        .into_iter()
        .filter(|kind| folder_of(*kind) == name)
        .collect();
    if !tree && project.is_empty() && own.is_empty() {
        return None;
    }
    let mut found = if tree {
        component_issues(files, signal).await
    } else {
        Vec::new()
    };
    for kind in project {
        let dir = child_of(".claude", folder_of(kind));
        let place = Where {
            dir: &dir,
            report: "",
            style: FolderStyle::Project,
        };
        found.extend(bare_issues(kind, files, &place, signal).await);
    }
    for kind in own {
        let place = Where {
            dir: ".",
            report: folder_of(kind),
            style: FolderStyle::Plugin,
        };
        found.extend(bare_issues(kind, files, &place, signal).await);
    }
    Some(found)
}

crate::registry::exports! {
    "loading/validate/components-only" {
        "componentsOnly" => async ok |input: BareInput|
            components_only(&input.files, &input.label, input.signal.as_ref()).await;
    }
}
