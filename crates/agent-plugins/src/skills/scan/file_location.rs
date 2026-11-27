//! Port of `src/skills/scan/file-location.ts`, switched.
//!
//! Where a skill or command file of a scope lives.

use serde::{Deserialize, Serialize};

use super::locations::{ScanScope, scope_rule};
use super::read::SKILL_FILE;

/// A skill folder (`SKILL.md`) or an older command file (skills.md, "Command files").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScopeFileKind {
    /// A skill folder.
    Skill,
    /// A command file.
    Command,
}

/// The path below a scope's container of a file of each kind named `name`.
fn below(kind: ScopeFileKind, name: &str) -> String {
    match kind {
        ScopeFileKind::Skill => format!("{name}/{SKILL_FILE}"),
        // `.claude/commands/<path>.md`: each folder before the file is a part of the name before
        // a `:`.
        ScopeFileKind::Command => format!("{}.md", name.split(':').collect::<Vec<_>>().join("/")),
    }
}

/// Where a skill or command file named `name` of `scope` lives, below `scope_folder`: the scope's
/// `.claude` folder (for `user`, the configuration directory). The rule the scan reads by
/// (`SCOPE_RULES`; skills.md, "Choose where skills load"): `skills/<name>/SKILL.md` and
/// `commands/<name>.md`. `None` when the scope reads no file of that kind: the managed, nested and
/// synced scopes read no commands (a synced skill's folder is claude.ai's: `skills/synced/`).
#[must_use]
pub fn scope_file_path(
    scope: ScanScope,
    kind: ScopeFileKind,
    scope_folder: &str,
    name: &str,
) -> Option<String> {
    let rule = scope_rule(scope);
    let container = match kind {
        ScopeFileKind::Skill => rule.skills,
        ScopeFileKind::Command => rule.commands,
    }?;
    Some(format!(
        "{scope_folder}/{}/{}",
        container.folder,
        below(kind, name)
    ))
}

crate::registry::exports! {
    "skills/scan/file-location" {
        "scopeFilePath" => ok |scope: ScanScope, kind: ScopeFileKind, scope_folder: String, name: String|
            scope_file_path(scope, kind, &scope_folder, &name);
    }
}
