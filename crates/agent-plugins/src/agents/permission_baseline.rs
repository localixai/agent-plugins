//! Port of `src/agents/permission-baseline.ts`, switched.
//!
//! `PermissionMode` and `PermissionBehavior` are the official types of `protocol/official.ts`,
//! which is not in the core yet: they are declared here, as the SDK's unions, until it is.

use serde::{Deserialize, Serialize};

/// The permission modes of permissions.md ("Permission modes"), as the SDK's `PermissionMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Asks before anything beyond a read.
    Default,
    /// Edits in scope run without asking.
    AcceptEdits,
    /// Everything runs.
    BypassPermissions,
    /// Plans only.
    Plan,
    /// Whatever would prompt is denied.
    DontAsk,
    /// A background classifier decides.
    Auto,
}

/// The three rule behaviours of permissions.md ("Manage permissions"), as the SDK's
/// `PermissionBehavior`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Runs.
    Allow,
    /// Refused.
    Deny,
    /// Prompts.
    Ask,
}

/// The kinds of action the "Available modes" table tells apart. A read-only Bash command (the
/// built-in read-only set of permissions.md) is a `read`: permission-modes.md has Manual and
/// `dontAsk` still run it and `acceptEdits` prompt for the other commands "except the built-in
/// read-only set". Classifying a command as read-only is the host's.
pub const ACTION_CATEGORIES: [ActionCategory; 4] = [
    ActionCategory::Read,
    ActionCategory::Edit,
    ActionCategory::FilesystemCommand,
    ActionCategory::Other,
];

/// One of `ACTION_CATEGORIES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionCategory {
    /// A read.
    Read,
    /// A file edit.
    Edit,
    /// A common filesystem command.
    FilesystemCommand,
    /// Anything else.
    Other,
}

/// `classify`: the mode's background classifier decides (auto mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BaselineDecision {
    /// Runs.
    Allow,
    /// Refused.
    Deny,
    /// Prompts.
    Ask,
    /// The mode's background classifier decides (auto mode).
    Classify,
}

/// One tool call as far as a mode's baseline is concerned. `in_scope` is "paths in the working
/// directory or `additionalDirectories`": where it matters (`acceptEdits`, `dontAsk`, `auto`) the
/// table says so, elsewhere it is ignored. Deciding the scope of a path is the host's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineAction {
    /// The kind of action.
    pub category: ActionCategory,
    /// The path is in the working directory or `additionalDirectories`.
    pub in_scope: bool,
}

/// A cell that depends on the path being in scope.
#[derive(Clone, Copy)]
struct ScopedCell {
    in_scope: BaselineDecision,
    out_of_scope: BaselineDecision,
}

#[derive(Clone, Copy)]
enum Cell {
    Plain(BaselineDecision),
    Scoped(ScopedCell),
}

#[derive(Clone, Copy)]
struct Row {
    read: Cell,
    edit: Cell,
    filesystem_command: Cell,
    other: Cell,
}

const fn rest(decision: BaselineDecision) -> Row {
    Row {
        read: Cell::Plain(decision),
        edit: Cell::Plain(decision),
        filesystem_command: Cell::Plain(decision),
        other: Cell::Plain(decision),
    }
}

/// In scope allowed, out of scope prompts: the `acceptEdits` cell for edits and filesystem commands,
/// and every asking mode's cell for reads (§13.1 item 2; in `auto` the first read outside prompts,
/// §13.9.4 item 2, and which read is the first is session state the baseline does not keep).
const ALLOW_IN_SCOPE: Cell = Cell::Scoped(ScopedCell {
    in_scope: BaselineDecision::Allow,
    out_of_scope: BaselineDecision::Ask,
});

/// permission-modes.md, "Available modes", "What runs without asking", cell by cell. A "prompt"
/// is `ask`. Modelled is the baseline only, as the doc says: "Modes set the baseline" and rules,
/// hooks, protected paths and critical-path removals layer on top (spec/judgments.md).
const fn baseline(mode: PermissionMode) -> Row {
    match mode {
        // "Reads only"
        PermissionMode::Default => Row {
            read: ALLOW_IN_SCOPE,
            ..rest(BaselineDecision::Ask)
        },
        // "Reads, file edits, and common filesystem commands" (in scope)
        PermissionMode::AcceptEdits => Row {
            read: ALLOW_IN_SCOPE,
            edit: ALLOW_IN_SCOPE,
            filesystem_command: ALLOW_IN_SCOPE,
            ..rest(BaselineDecision::Ask)
        },
        // "Reads, plus classifier-approved commands when auto mode is available": only the baseline.
        PermissionMode::Plan => Row {
            read: ALLOW_IN_SCOPE,
            ..rest(BaselineDecision::Ask)
        },
        // "Everything, with background safety checks": reads and in-scope edits skip the classifier
        // (decision order, step 2), "everything else goes to the classifier" (step 3).
        PermissionMode::Auto => Row {
            read: ALLOW_IN_SCOPE,
            edit: Cell::Scoped(ScopedCell {
                in_scope: BaselineDecision::Allow,
                out_of_scope: BaselineDecision::Classify,
            }),
            filesystem_command: Cell::Plain(BaselineDecision::Classify),
            other: Cell::Plain(BaselineDecision::Classify),
        },
        // "File reads inside your working directories ...; anything that would prompt is denied"
        PermissionMode::DontAsk => Row {
            read: Cell::Scoped(ScopedCell {
                in_scope: BaselineDecision::Allow,
                out_of_scope: BaselineDecision::Deny,
            }),
            ..rest(BaselineDecision::Deny)
        },
        // "Everything"
        PermissionMode::BypassPermissions => rest(BaselineDecision::Allow),
    }
}

const fn cell_of(row: Row, category: ActionCategory) -> Cell {
    match category {
        ActionCategory::Read => row.read,
        ActionCategory::Edit => row.edit,
        ActionCategory::FilesystemCommand => row.filesystem_command,
        ActionCategory::Other => row.other,
    }
}

/// What `mode` does with `action` before any rule, hook or classifier verdict.
#[must_use]
pub const fn baseline_decision(mode: PermissionMode, action: BaselineAction) -> BaselineDecision {
    match cell_of(baseline(mode), action.category) {
        Cell::Plain(decision) => decision,
        Cell::Scoped(cell) => {
            if action.in_scope {
                cell.in_scope
            } else {
                cell.out_of_scope
            }
        }
    }
}

crate::registry::exports! {
    "agents/permission-baseline" {
        "ACTION_CATEGORIES" => ok || ACTION_CATEGORIES;
        "baselineDecision" => ok |mode: PermissionMode, action: BaselineAction| baseline_decision(mode, action);
    }
}
