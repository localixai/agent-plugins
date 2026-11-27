//! Port of `src/settings/general/worktree-block.ts`, switched.
//!
//! The `worktree` block (settings-reference.md, `worktree` and its entries): the fields as the
//! files combine them, with the documented default of each. An invalid field never reaches the
//! layer: the read drops it with a warning (`closed-fields.ts`), so a lower file's value or the
//! default applies.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::undefined_if_none;
use crate::settings::view::PolicyView;

/// Where a new worktree starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseRef {
    /// Branches from `origin/<default-branch>`.
    Fresh,
    /// Branches from the current local `HEAD`.
    Head,
}

/// How background sessions isolate their file edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BgIsolation {
    /// Edits are refused in the main checkout.
    Worktree,
    /// Jobs edit the working copy.
    None,
}

/// The `worktree` block with each field's documented default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSettings {
    /// `"fresh"` branches from `origin/<default-branch>`, `"head"` from the current local `HEAD`.
    pub base_ref: BaseRef,
    /// Directories symlinked from the main repository into each worktree; none by default.
    pub symlink_directories: Vec<String>,
    /// Directories checked out by git sparse-checkout; `undefined` checks out the whole tree.
    #[serde(serialize_with = "undefined_if_none")]
    pub sparse_paths: Option<Vec<String>>,
    /// How background sessions isolate their file edits.
    pub bg_isolation: BgIsolation,
    /// The folder Desktop SSH sessions create their worktrees in, from user settings only.
    #[serde(serialize_with = "undefined_if_none")]
    pub location: Option<String>,
}

fn strings(value: Option<&JsValue>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// The `worktree` block as the files combine it, with the defaults.
pub async fn read_worktree(view: &dyn PolicyView) -> WorktreeSettings {
    let block = view.get("worktree").await;
    let field = |name: &str| block.as_ref().and_then(|block| block.get(name));
    WorktreeSettings {
        base_ref: match field("baseRef").and_then(JsValue::as_str) {
            Some("head") => BaseRef::Head,
            Some(_) | None => BaseRef::Fresh,
        },
        symlink_directories: strings(field("symlinkDirectories")).unwrap_or_default(),
        sparse_paths: strings(field("sparsePaths")),
        bg_isolation: match field("bgIsolation").and_then(JsValue::as_str) {
            Some("none") => BgIsolation::None,
            Some(_) | None => BgIsolation::Worktree,
        },
        location: view
            .get("worktree.location")
            .await
            .and_then(|value| value.as_str().map(str::to_owned)),
    }
}

/// Where a new worktree starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BaseRefTarget {
    /// `origin/<default-branch>`.
    OriginDefaultBranch,
    /// The main checkout's `HEAD`.
    CheckoutHead,
    /// The linked worktree's `HEAD`.
    LinkedWorktreeHead,
}

/// What the session knows of where it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseRefFacts {
    /// The session runs inside a linked worktree rather than the main checkout.
    pub inside_linked_worktree: bool,
}

/// What a `baseRef` resolves to: `"head"` is the current local `HEAD`, and "inside a linked
/// worktree, `"head"` resolves to that worktree's `HEAD`, not the main checkout's".
#[must_use]
pub const fn base_ref_target(base_ref: BaseRef, facts: BaseRefFacts) -> BaseRefTarget {
    match base_ref {
        BaseRef::Fresh => BaseRefTarget::OriginDefaultBranch,
        BaseRef::Head => {
            if facts.inside_linked_worktree {
                BaseRefTarget::LinkedWorktreeHead
            } else {
                BaseRefTarget::CheckoutHead
            }
        }
    }
}

/// What the session knows of a background session's edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's BackgroundEditFacts holds these four facts, each its own"
)]
pub struct BackgroundEditFacts {
    /// The session was moved to the background with `←` or `/background`.
    pub moved_to_background: bool,
    /// The session has called `EnterWorktree`.
    pub entered_worktree: bool,
    /// The session runs inside a git repository.
    pub inside_git_repository: bool,
    /// A `WorktreeCreate` hook failed (the non-git version control case).
    pub worktree_create_hook_failed: bool,
}

/// Whether `Edit` and `Write` are refused in the main checkout of a background session.
/// `"worktree"` blocks them "until the session calls `EnterWorktree`"; `"none"` lets jobs edit the
/// working copy; a session moved to the background "edits files in place whatever this key says";
/// and "outside a git repository, a `WorktreeCreate` hook that fails releases the block".
#[must_use]
pub const fn background_edits_blocked(
    bg_isolation: BgIsolation,
    facts: BackgroundEditFacts,
) -> bool {
    if matches!(bg_isolation, BgIsolation::None)
        || facts.moved_to_background
        || facts.entered_worktree
    {
        return false;
    }
    facts.inside_git_repository || !facts.worktree_create_hook_failed
}

/// Who creates a worktree.
pub const WORKTREE_CONSUMERS: [WorktreeConsumer; 5] = [
    WorktreeConsumer::DesktopSsh,
    WorktreeConsumer::WorktreeFlag,
    WorktreeConsumer::EnterWorktreeTool,
    WorktreeConsumer::IsolatedSubagent,
    WorktreeConsumer::BackgroundSession,
];

/// One of `WORKTREE_CONSUMERS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeConsumer {
    /// Desktop SSH sessions.
    DesktopSsh,
    /// The `--worktree` flag.
    WorktreeFlag,
    /// The `EnterWorktree` tool.
    EnterWorktreeTool,
    /// A subagent with isolation.
    IsolatedSubagent,
    /// A background session.
    BackgroundSession,
}

/// What the host knows of where a worktree is made.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeFolderFacts {
    /// Who creates the worktree.
    pub consumer: WorktreeConsumer,
    /// The **Worktree folder** set on the SSH connection in Desktop.
    #[serde(default)]
    pub connection_folder: Option<String>,
    /// The organization restricts which folders sessions may use.
    #[serde(default)]
    pub organization_restricts_folders: Option<bool>,
}

/// The consumers that read `worktree.location`: "Only the desktop app reads this key".
const fn reads_location(consumer: WorktreeConsumer) -> bool {
    match consumer {
        WorktreeConsumer::DesktopSsh => true,
        WorktreeConsumer::WorktreeFlag
        | WorktreeConsumer::EnterWorktreeTool
        | WorktreeConsumer::IsolatedSubagent
        | WorktreeConsumer::BackgroundSession => false,
    }
}

/// The folder worktrees go in instead of `<project-root>/.claude/worktrees/`, or `undefined` for
/// the project's own. The connection's folder "takes precedence"; an organization's restriction
/// keeps the worktrees inside the project.
#[must_use]
pub fn worktree_folder(settings: &WorktreeSettings, facts: &WorktreeFolderFacts) -> Option<String> {
    if !reads_location(facts.consumer) || facts.organization_restricts_folders == Some(true) {
        return None;
    }
    facts
        .connection_folder
        .clone()
        .or_else(|| settings.location.clone())
}

crate::registry::exports! {
    "settings/general/worktree-block" {
        "readWorktree" => async ok |view: Port| read_worktree(&view).await;
        "baseRefTarget" => ok |base_ref: BaseRef, facts: BaseRefFacts| base_ref_target(base_ref, facts);
        "backgroundEditsBlocked" => ok |bg_isolation: BgIsolation, facts: BackgroundEditFacts| background_edits_blocked(bg_isolation, facts);
        "WORKTREE_CONSUMERS" => ok || WORKTREE_CONSUMERS;
        "worktreeFolder" => ok |settings: WorktreeSettings, facts: WorktreeFolderFacts| worktree_folder(&settings, &facts);
    }
}
