//! Port of `src/marketplace/staged-commit.ts`, switched.
//!
//! Installing a fetched plugin by moving the directory the host staged it in into the version folder
//! of the cache (plugins/troubleshooting.md, "Could not move the new copy of this plugin version
//! into `<path>`": "Claude Code downloads a fresh copy of its files and moves it into that version's
//! folder in the plugin cache"). The move is the host's, atomic and with the earlier copy set aside
//! and put back; a move that fails is the documented message, never a throw. A staged tree that
//! holds symbolic links is installed with the `TreePlan` of `planTreeCopy`, which the port applies;
//! a plain move would ship the links that point outside the marketplace and could not dereference
//! the ones inside it.

use serde::Deserialize;

use super::install_failures::move_failure_message;
use super::ports::{StagedCommit, StagedDirectory};
use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;
use crate::ports::Signal;
use crate::registry::Port;

/// What an install of a staged directory is given: the host's directory and the request it takes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInput {
    /// Where the host staged the plugin.
    pub staged_path: String,
    /// A version folder of the cache.
    pub destination: String,
    /// What the copy does with each symbolic link; absent when the staged tree holds no link.
    #[serde(default)]
    pub plan: Option<JsValue>,
    /// The host's staged directory.
    pub staged: Port,
    /// Cancels the move.
    pub signal: Signal,
}

/// Installs `stagedPath` at `destination` (moved whole, or materialized by `plan`). `undefined` on success; otherwise the issue that says why it
/// is not installed, with what became of the copy installed before. An abort is an issue of its own:
/// nothing moved, so there is no earlier copy to report.
pub async fn commit_staged_plugin(input: &CommitInput) -> Option<PluginIssue> {
    let request = StagedCommit {
        staged_path: &input.staged_path,
        destination: &input.destination,
        plan: input.plan.as_ref(),
    };
    let failure = input.staged.commit(&request, &input.signal).await?;
    let message = move_failure_message(&input.destination, &failure)
        .unwrap_or_else(|| "The install was aborted before the move".to_owned());
    Some(error("source", &message))
}

crate::registry::exports! {
    "marketplace/staged-commit" {
        "commitStagedPlugin" => async ok |input: CommitInput| commit_staged_plugin(&input).await;
    }
}
