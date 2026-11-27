//! Port of `src/agents/workflow/saved-scan.ts`, switched.
//!
//! Reads the saved workflows (workflows.md, "Save the workflow for reuse") through the `FolderScan`
//! port: `<dir>/.claude/workflows/*.js` for the working directory and every parent up to the
//! repository root ("Project workflows also load from every `.claude/workflows/` along that path"),
//! and the personal `~/.claude/workflows/` (the `workflows/` folder under `CLAUDE_CONFIG_DIR` when
//! set). A workflow runs as `/<meta.name>`; a script without a readable `meta` is left out ("Claude
//! Code drops `/<name>` from `/` autocomplete"), with the issues of its `meta` at the file's path. A
//! workflow named `anthropic-skills` or `anthropic-skills:<name>` does not load (skills.md, "Names
//! reserved for synced skills"), with an error at its path.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::saved::{SaveScope, SavedResolution, SavedWorkflow, resolve_saved_workflows};
use crate::format::config_directory::config_folder_of;
use crate::format::issues::{PluginIssue, error};
use crate::format::skill_name::reserved_namespace;
use crate::format::workflow_meta::{WorkflowMeta, read_workflow_meta};
use crate::foundation::scope_folders::{
    Depth, ScopeBase, ScopeContext, ScopeFilesOptions, ScopeRequest, place_path, read_scope_files,
    scope_bases,
};
use crate::js::promise_all;
use crate::ports::folder_scan::FolderScan;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// What a scan of the saved workflows is asked for: a `ScopeRequest` and its cancellation.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedScanRequest {
    /// The scope request.
    #[serde(flatten)]
    pub scope: ScopeRequest,
    /// Ends the scan early.
    #[serde(default)]
    pub signal: Option<Signal>,
}

impl AsRef<ScopeRequest> for SavedScanRequest {
    fn as_ref(&self) -> &ScopeRequest {
        &self.scope
    }
}

/// The saved workflows each name runs, and what is wrong with the files read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavedScan {
    /// The workflow each name runs.
    pub winners: Vec<SavedWorkflow>,
    /// The ones a winner of the same name hides.
    pub shadowed: Vec<SavedWorkflow>,
    /// What is wrong with the files read.
    pub issues: Vec<PluginIssue>,
}

struct Place {
    base: String,
    /// The workflows folder below the base.
    folder: String,
    scope: SaveScope,
    distance: f64,
}

/// What one place gives.
struct PlaceRead {
    found: Vec<SavedWorkflow>,
    issues: Vec<PluginIssue>,
}

/// skills.md, "Names reserved for synced skills": a saved workflow of that name "doesn't load".
const RESERVED_WORKFLOW: &str = "name uses \"anthropic-skills\", a name reserved for skills synced from claude.ai; the workflow does not load, rename it";

async fn read_place(folders: &dyn FolderScan, place: &Place, signal: Option<&Signal>) -> PlaceRead {
    let Some(opened) = folders.open(&place.base, signal).await else {
        return PlaceRead {
            found: Vec::new(),
            issues: Vec::new(),
        };
    };
    let options = ScopeFilesOptions {
        extension: ".js".to_owned(),
        depth: Depth::Flat,
        signal: signal.cloned(),
    };
    let read = read_scope_files(opened.as_ref(), &place.folder, &options).await;
    let metas: Vec<(String, _)> = read
        .files
        .iter()
        .map(|file| (file.host_path.clone(), read_workflow_meta(&file.text)))
        .collect();
    let loaded: Vec<(&String, &WorkflowMeta)> = metas
        .iter()
        .filter_map(|(path, read)| read.meta.as_ref().map(|meta| (path, meta)))
        .collect();
    let (reserved, kept): (Vec<_>, Vec<_>) = loaded
        .into_iter()
        .partition(|(_, meta)| reserved_namespace(&meta.name).is_some());
    let found = kept
        .into_iter()
        .map(|(path, meta)| SavedWorkflow {
            name: meta.name.clone(),
            description: Some(meta.description.clone()),
            scope: place.scope,
            distance: place.distance,
            path: path.clone(),
        })
        .collect();
    let mut issues = read.issues;
    for (path, read) in &metas {
        issues.extend(read.issues.iter().map(|issue| PluginIssue {
            path: path.clone(),
            ..issue.clone()
        }));
    }
    issues.extend(
        reserved
            .into_iter()
            .map(|(path, _)| error(path, RESERVED_WORKFLOW)),
    );
    PlaceRead { found, issues }
}

const WORKFLOWS: &str = ".claude/workflows";

/// The saved workflows the scope folders hold, resolved by name.
pub async fn scan_saved_workflows(folders: Port, request: SavedScanRequest) -> SavedScan {
    let signal = request.signal.clone();
    let user = config_folder_of(&request.scope.home, request.scope.config_dir.as_deref());
    let folders: Arc<dyn FolderScan> = Arc::new(folders);
    let context = ScopeContext {
        folders: Arc::clone(&folders),
        request,
        signal: signal.clone(),
        extra_scope_folders: None,
    };
    let chain = scope_bases(ScopeBase::Repository, &context).await;
    let mut places: Vec<Place> = chain
        .into_iter()
        .enumerate()
        .map(|(distance, base)| Place {
            base,
            folder: WORKFLOWS.to_owned(),
            scope: SaveScope::Project,
            distance: f64::from(u32::try_from(distance).unwrap_or(u32::MAX)),
        })
        .collect();
    places.push(Place {
        base: user.base,
        folder: place_path(&user.folder, "workflows"),
        scope: SaveScope::Personal,
        distance: 0.0,
    });
    let reads = promise_all(
        places
            .iter()
            .map(|place| -> BoxFuture<'_, PlaceRead> {
                Box::pin(read_place(folders.as_ref(), place, signal.as_ref()))
            })
            .collect(),
    )
    .await;
    let found: Vec<SavedWorkflow> = reads
        .iter()
        .flat_map(|read| read.found.iter().cloned())
        .collect();
    let SavedResolution { winners, shadowed } = resolve_saved_workflows(&found);
    SavedScan {
        winners,
        shadowed,
        issues: reads.into_iter().flat_map(|read| read.issues).collect(),
    }
}

crate::registry::exports! {
    "agents/workflow/saved-scan" {
        "scanSavedWorkflows" => async ok |folders: Port, request: SavedScanRequest|
            scan_saved_workflows(folders, request).await;
    }
}
