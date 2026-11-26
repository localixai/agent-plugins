//! Port of `src/agents/workflow/saved.ts` at sha256:f38e5ef4023c.
//!
//! Where a saved workflow is written and which one `/<name>` runs (workflows.md, "Save the workflow
//! for reuse"). Pure over the facts the host reads from the disk; the host does the write and the
//! `lstat`s.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::config_directory::{Join, config_directory_of};
use crate::format::issues::{PluginIssue, error};
use crate::format::listing::by_code_unit;
use crate::js::{JsValue, json_stringify};
use crate::registry::{Callback, Set};

/// The places a workflow is saved.
pub const SAVE_SCOPES: [SaveScope; 2] = [SaveScope::Project, SaveScope::Personal];

/// One of `SAVE_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveScope {
    /// The project's `.claude/workflows/`.
    Project,
    /// `~/.claude/workflows/`.
    Personal,
}

/// What to save and what the host knows of the disk. `J` is `Locations.joinPath` (`Join`): a base
/// and a `/`-separated relative path, joined the way the host writes paths.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest<J> {
    /// Where.
    pub scope: SaveScope,
    /// The workflow's file name without `.js`.
    pub name: String,
    /// The working directory, then each parent up to the repository root, nearest first (`FolderScan.repository`).
    pub directories: Vec<String>,
    /// The directories of that chain that already have a `.claude/workflows` folder.
    pub with_workflows: Set<String>,
    /// The home folder.
    pub home: String,
    /// `CLAUDE_CONFIG_DIR`, when set.
    #[serde(default)]
    pub config_dir: Option<String>,
    /// Joins a base and a relative path.
    pub join: J,
}

/// Where the file goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePlan {
    /// Where.
    pub scope: SaveScope,
    /// The folder the file goes in.
    pub directory: String,
    /// The file: `<directory>/<name>.js`.
    pub path: String,
    /// The paths that must not be symbolic links, for the host to check before it writes.
    pub must_not_be_links: Vec<String>,
}

const PROJECT_FOLDER: &str = ".claude/workflows";

/// "the closest `.claude/workflows/` directory that already exists between your working directory and the repository root, or the repository root if none exists yet".
fn project_base<J>(request: &SaveRequest<J>) -> Option<&String> {
    let closest = request
        .directories
        .iter()
        .find(|directory| request.with_workflows.members.contains(directory));
    closest.or_else(|| request.directories.last())
}

/// What a location answers: where the file goes, and the paths that must not be links.
struct Found {
    directory: String,
    path: String,
    must_not_be_links: Vec<String>,
}

/// The two save locations. Project: refused if `.claude`, `.claude/workflows` or the target file is a
/// symbolic link. Personal: `~/.claude/workflows/`, or the `workflows/` directory under
/// `CLAUDE_CONFIG_DIR`; refused only if the target file itself is a link, "so a `~/.claude` directory
/// managed by a dotfiles tool still works".
async fn location<J: Join>(request: &SaveRequest<J>, file: &str) -> Option<Found> {
    match request.scope {
        SaveScope::Project => {
            let base = project_base(request)?;
            let directory = request.join.join(base, PROJECT_FOLDER).await;
            let path = request
                .join
                .join(base, &format!("{PROJECT_FOLDER}/{file}"))
                .await;
            let claude = request.join.join(base, ".claude").await;
            Some(Found {
                must_not_be_links: vec![claude, directory.clone(), path.clone()],
                directory,
                path,
            })
        }
        SaveScope::Personal => {
            let configuration =
                config_directory_of(&request.home, request.config_dir.as_deref(), &request.join)
                    .await;
            let directory = request.join.join(&configuration, "workflows").await;
            let path = request.join.join(&directory, file).await;
            Some(Found {
                must_not_be_links: vec![path.clone()],
                directory,
                path,
            })
        }
    }
}

/// A name that is one file name: no folder, no `.` or `..`, not empty (judgment: the docs name no rule).
/// `/^[^/\\\0]+$/`.
fn is_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', '\0'])
}

/// Where a workflow is saved, or why it cannot be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SaveOutcome {
    /// The plan.
    Planned {
        /// Where the file goes.
        plan: SavePlan,
    },
    /// Why there is none.
    Refused {
        /// The error.
        issue: PluginIssue,
    },
}

/// The plan of a save, or the issue that refuses it.
pub async fn plan_workflow_save<J: Join>(request: &SaveRequest<J>) -> SaveOutcome {
    let bad = !is_file_name(&request.name) || request.name == "." || request.name == "..";
    if bad {
        let written = json_stringify(&JsValue::from(request.name.as_str())).unwrap_or_default();
        return SaveOutcome::Refused {
            issue: error(
                "name",
                &format!("{written} is not a file name for a workflow"),
            ),
        };
    }
    let Some(found) = location(request, &format!("{}.js", request.name)).await else {
        return SaveOutcome::Refused {
            issue: error("directories", "no working directory to save under"),
        };
    };
    SaveOutcome::Planned {
        plan: SavePlan {
            scope: request.scope,
            directory: found.directory,
            path: found.path,
            must_not_be_links: found.must_not_be_links,
        },
    }
}

/// "shows an error instead of writing through one": the first of `must_not_be_links` that is a symbolic link.
#[must_use]
pub fn refuse_links(plan: &SavePlan, linked: &[String]) -> Option<PluginIssue> {
    let link = plan
        .must_not_be_links
        .iter()
        .find(|path| linked.contains(path))?;
    Some(error(
        link,
        &format!("refusing to save the workflow through a symbolic link: {link}"),
    ))
}

/// A workflow file found in a save location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedWorkflow {
    /// What `/<name>` runs it as: its `meta.name`.
    pub name: String,
    /// Its `meta.description`, which the `meta` block must hold (workflows.md, "Edit a saved script"),
    /// for a listing beside the name. Always set by `scanSavedWorkflows`; optional so a host that builds
    /// the list itself need not give one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Where it is saved.
    pub scope: SaveScope,
    /// For a project workflow, how many levels above the working directory its `.claude/workflows` is: 0 is the working directory's own.
    pub distance: f64,
    /// The file.
    pub path: String,
}

/// Which workflow each name runs.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedResolution {
    /// The workflow each name runs.
    pub winners: Vec<SavedWorkflow>,
    /// The ones a winner of the same name hides.
    pub shadowed: Vec<SavedWorkflow>,
}

/// Lower is stronger: the project's closest folder first, the personal one last.
fn rank(workflow: &SavedWorkflow) -> f64 {
    match workflow.scope {
        SaveScope::Personal => f64::INFINITY,
        SaveScope::Project => workflow.distance,
    }
}

/// `rank(a) - rank(b) || byCodeUnit(a.path, b.path)`: a difference of zero or NaN (two personal
/// workflows) falls through to the path.
fn compare(first: &SavedWorkflow, second: &SavedWorkflow) -> Ordering {
    match (rank(first) - rank(second)).partial_cmp(&0.0) {
        Some(Ordering::Less) => Ordering::Less,
        Some(Ordering::Greater) => Ordering::Greater,
        Some(Ordering::Equal) | None => by_code_unit(&first.path, &second.path),
    }
}

/// "when more than one defines the same name Claude Code runs the one closest to the working
/// directory. If a project workflow and a personal workflow share a name, the project one runs." Two in
/// one folder with one name: the first path in code-unit order (judgment).
#[must_use]
pub fn resolve_saved_workflows(found: &[SavedWorkflow]) -> SavedResolution {
    let mut ordered: Vec<&SavedWorkflow> = found.iter().collect();
    ordered.sort_by(|first, second| compare(first, second));
    let mut winners: IndexMap<&str, &SavedWorkflow> = IndexMap::new();
    let mut shadowed = Vec::new();
    for workflow in ordered {
        if winners.contains_key(workflow.name.as_str()) {
            shadowed.push(workflow.clone());
        } else {
            winners.insert(workflow.name.as_str(), workflow);
        }
    }
    SavedResolution {
        winners: winners.into_values().cloned().collect(),
        shadowed,
    }
}

crate::registry::exports! {
    "agents/workflow/saved" {
        "SAVE_SCOPES" => ok || SAVE_SCOPES;
        "planWorkflowSave" => async ok |request: SaveRequest<Callback>| plan_workflow_save(&request).await;
        "refuseLinks" => ok |plan: SavePlan, linked: Set<String>| refuse_links(&plan, &linked.members);
        "resolveSavedWorkflows" => ok |found: Vec<SavedWorkflow>| resolve_saved_workflows(&found);
    }
}
