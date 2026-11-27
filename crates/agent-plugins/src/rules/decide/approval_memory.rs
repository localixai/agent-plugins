//! Port of `src/rules/decide/approval-memory.ts`, switched.
//!
//! What "Yes, and don't ask again" does after a prompt (permissions.md, "Permission system", the last
//! column of the table), per type of tool: how long the approval lasts and what it is keyed to. A
//! saved approval is a rule the host writes to `.claude/settings.local.json` at the root of the git
//! repository (resolved through worktrees to the main checkout); the host decides where it can.

use serde::{Deserialize, Serialize};

/// The types of tool of the table.
pub const TOOL_TYPES: [ToolType; 5] = [
    ToolType::ReadOnly,
    ToolType::Bash,
    ToolType::FileModification,
    ToolType::WebFetch,
    ToolType::WebSearch,
];

/// One of `TOOL_TYPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolType {
    /// `read-only`.
    ReadOnly,
    /// `bash`.
    Bash,
    /// `file-modification`.
    FileModification,
    /// `web-fetch`.
    WebFetch,
    /// `web-search`.
    WebSearch,
}

/// How long an approval lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalLifetime {
    /// `none`.
    None,
    /// `session`.
    Session,
    /// `repository`.
    Repository,
}

/// What a saved approval names: a command prefix, a domain, or the whole tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyedBy {
    /// `command`.
    Command,
    /// `domain`.
    Domain,
    /// `tool`.
    Tool,
    /// `none`.
    None,
}

/// What choosing "Yes, and don't ask again" does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalMemory {
    /// How long it lasts.
    pub lifetime: ApprovalLifetime,
    /// What a saved approval names: a command prefix, a domain, or the whole tool.
    pub keyed_by: KeyedBy,
}

/// The table's last column, one row per tool type.
const fn approval_memory_of(tool_type: ToolType) -> ApprovalMemory {
    match tool_type {
        // "N/A": a read inside the working directories needs no approval to remember.
        ToolType::ReadOnly => ApprovalMemory {
            lifetime: ApprovalLifetime::None,
            keyed_by: KeyedBy::None,
        },
        // "Permanently per repository and command"
        ToolType::Bash => ApprovalMemory {
            lifetime: ApprovalLifetime::Repository,
            keyed_by: KeyedBy::Command,
        },
        // "Until session end": "A file-modification approval isn't saved to the file".
        ToolType::FileModification => ApprovalMemory {
            lifetime: ApprovalLifetime::Session,
            keyed_by: KeyedBy::None,
        },
        // "Permanently per repository and domain"
        ToolType::WebFetch => ApprovalMemory {
            lifetime: ApprovalLifetime::Repository,
            keyed_by: KeyedBy::Domain,
        },
        // "Permanently per repository"
        ToolType::WebSearch => ApprovalMemory {
            lifetime: ApprovalLifetime::Repository,
            keyed_by: KeyedBy::Tool,
        },
    }
}

/// The type of tool a built-in tool belongs to in that table: file reads and searches, shell
/// execution (`Bash` and `PowerShell`, the shells this table's "Shell execution" row names by its
/// example, spec/judgments.md), file modification, WebFetch and WebSearch. Any other tool is not in the table.
const TYPE_OF: [(&str, ToolType); 10] = [
    ("Read", ToolType::ReadOnly),
    ("Grep", ToolType::ReadOnly),
    ("Glob", ToolType::ReadOnly),
    ("Bash", ToolType::Bash),
    ("PowerShell", ToolType::Bash),
    ("Edit", ToolType::FileModification),
    ("Write", ToolType::FileModification),
    ("NotebookEdit", ToolType::FileModification),
    ("WebFetch", ToolType::WebFetch),
    ("WebSearch", ToolType::WebSearch),
];

/// The type of tool of a built-in tool; `undefined` for a tool the table does not list.
#[must_use]
pub fn tool_type_of(tool_name: &str) -> Option<ToolType> {
    TYPE_OF
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, tool_type)| *tool_type)
}

const NOTHING_SAVED: ApprovalMemory = ApprovalMemory {
    lifetime: ApprovalLifetime::None,
    keyed_by: KeyedBy::None,
};

/// What a session can change about "Yes, and don't ask again".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalOptions {
    /// `allowManagedPermissionRulesOnly` is on: it "hides the always-allow choices in permission
    /// prompts, and stops saving new rules" (settings-reference.md), so an approval that would be saved
    /// as a rule is not offered (spec/judgments.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_locked: Option<bool>,
}

/// What choosing "Yes, and don't ask again" does for a tool; `undefined` for a tool the table does not list.
#[must_use]
pub fn approval_memory(tool_name: &str, options: ApprovalOptions) -> Option<ApprovalMemory> {
    let memory = approval_memory_of(tool_type_of(tool_name)?);
    Some(
        if options.rules_locked == Some(true) && memory.lifetime == ApprovalLifetime::Repository {
            NOTHING_SAVED
        } else {
            memory
        },
    )
}

crate::registry::exports! {
    "rules/decide/approval-memory" {
        "TOOL_TYPES" => ok || TOOL_TYPES;
        "toolTypeOf" => ok |tool_name: String| tool_type_of(&tool_name);
        "approvalMemory" => ok |tool_name: String, options: Option<ApprovalOptions>| approval_memory(&tool_name, options.unwrap_or_default());
    }
}
