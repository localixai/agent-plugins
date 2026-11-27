//! Port of `src/skills/scan/container.ts`, switched.
//!
//! One skills container that is not a scope's `.claude/skills`: the folder a plugin's manifest
//! lists under `skills` (plugins/components.md, "Skills"), a store's or a curator's skills
//! directory. Its immediate children are the skill folders, read as `scanFolders` reads them, and
//! the entries carry no body.

use std::sync::Arc;

use serde::Deserialize;

use super::locations::{ScanContext, ScanRequest, ScanScope, ScopeRule, scope_rule};
use super::read::{read_skill_folder, read_skills};
use super::scan_types::{ScanOutcome, Where, empty};
use crate::format::skill_name::SkillOrigin;
use crate::foundation::scope_folders::{ScopeContext, ScopePlace, ScopeRequest};
use crate::js::JsValue;
use crate::ports::folder_scan::FolderScan;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// What `scanSkillContainer` reads the container as.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRequest {
    /// What the entries report as their scope.
    pub scope: ScanScope,
    /// What a skill of the container is read as, as the host wrote it.
    pub origin: JsValue,
    /// The container holds a `SKILL.md` itself: the folder is one skill and its own subfolders
    /// are its files, not more skills (`"skills": ["./"]` lists a plugin whose root is one skill).
    /// It goes by this name unless its frontmatter names it.
    pub root_name: String,
    /// Ends the scan early.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The skills of `container` (a path relative to `dir`, `.` for `dir` itself) read through the
/// port. A folder that does not exist has no skills.
pub async fn scan_skill_container(
    folders: Arc<dyn FolderScan>,
    dir: &str,
    container: &str,
    request: &ContainerRequest,
    origin: SkillOrigin,
) -> ScanOutcome {
    let signal = request.signal.as_ref();
    let Some(folder) = folders.open(dir, signal).await else {
        return empty();
    };
    let rule = ScopeRule {
        origin: request.origin.clone(),
        layout_origin: origin,
        ..scope_rule(request.scope)
    };
    let context = ScanContext {
        scope: ScopeContext {
            folders,
            request: ScanRequest {
                scope: ScopeRequest {
                    home: dir.to_owned(),
                    cwd: dir.to_owned(),
                    ..ScopeRequest::default()
                },
                nested_directories: None,
            },
            signal: request.signal.clone(),
            extra_scope_folders: None,
        },
        allows: None,
    };
    let place = Where {
        context: &context,
        rule,
        scope: request.scope,
        folder,
        // Not a scope folder: never the documented one, so no skill folder here is read as a
        // plugin.
        place: ScopePlace {
            base: dir.to_owned(),
            root: container.to_owned(),
            index: 0,
            documented: false,
        },
    };
    let root = if container == "." {
        "SKILL.md".to_owned()
    } else {
        format!("{container}/SKILL.md")
    };
    if place.folder.files().await.exists(&root, signal).await {
        let canonical = place.folder.canonical_path(container).await;
        return read_skill_folder(&place, &request.root_name, container, canonical).await;
    }
    read_skills(&place, container, |_, _| -> BoxFuture<'_, _> {
        Box::pin(std::future::ready(Vec::new()))
    })
    .await
}

/// The host's `scanSkillContainer`: its origin read as the skill readers take it.
async fn host_scan(
    folders: Port,
    dir: String,
    container: String,
    request: ContainerRequest,
) -> Result<ScanOutcome, String> {
    let origin =
        SkillOrigin::deserialize(request.origin.clone()).map_err(|error| error.to_string())?;
    Ok(scan_skill_container(Arc::new(folders), &dir, &container, &request, origin).await)
}

crate::registry::exports! {
    "skills/scan/container" {
        "scanSkillContainer" => async fallible |folders: Port, dir: String, container: String, request: ContainerRequest|
            host_scan(folders, dir, container, request).await;
    }
}
