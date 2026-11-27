//! Port of `src/skills/scan/nested.ts`, switched.
//!
//! Skills of a `.claude/skills` below the working directory (skills.md, "Load skills in monorepos
//! and subdirectories"): "Skills in a `.claude/skills/` directory below where you started don't
//! load at startup. They load the first time Claude reads or edits a file in that subdirectory and
//! stay available for the rest of the session."

use serde::Serialize;

use super::locations::{ScanContext, ScanRequest, ScanScope, scan_context_of};
use super::scan::scan_scope;
use super::scan_types::ScanOutcome;
use crate::foundation::scope_folders::HostContext;
use crate::registry::Set;

/// What `scanNested` found.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedScan {
    /// The skills and commands found.
    #[serde(flatten)]
    pub outcome: ScanOutcome,
    /// The directories scanned now: the session keeps them as loaded and never asks again.
    pub directories: Vec<String>,
}

/// The skills of the directories between the working directory and the file Claude touched,
/// outermost first, leaving out the ones the session already scanned (`loaded`). A file outside
/// the working directory loads nothing: that is `/add-dir`'s.
pub async fn scan_nested(context: &ScanContext, touched: &str, loaded: &[String]) -> NestedScan {
    let scope = &context.scope;
    let below = scope
        .folders
        .directories_below(&scope.request.scope.cwd, touched, scope.signal.as_ref())
        .await;
    let directories: Vec<String> = below
        .into_iter()
        .filter(|directory| !loaded.contains(directory))
        .collect();
    let mut nested = context.clone();
    nested.scope.request.nested_directories = Some(directories.clone());
    let outcome = scan_scope(&nested, ScanScope::Nested).await;
    NestedScan {
        outcome,
        directories,
    }
}

crate::registry::exports! {
    "skills/scan/nested" {
        "scanNested" => async ok |context: HostContext<ScanRequest>, touched: String, loaded: Option<Set<String>>|
            scan_nested(
                &scan_context_of(context).await,
                &touched,
                &loaded.map(|loaded| loaded.members).unwrap_or_default(),
            )
            .await;
    }
}
