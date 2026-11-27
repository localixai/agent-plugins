//! Ported from `src/skills/scan/scan-types.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a scan of the scope folders returns. An entry holds the skill or command without its body
//! (`InvocableHead`) and the `BodyReader` that gets the body when the skill runs: a catalog of
//! hundreds of skills keeps their names, descriptions and settings, not megabytes of Markdown
//! (spec/judgments.md, "metadata-first catalog").

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::locations::{ScanContext, ScanScope, ScopeRule};
use crate::format::issues::PluginIssue;
use crate::format::skill_types::BodyReader;
use crate::foundation::scope_folders::ScopePlace;
use crate::js::JsObject;
use crate::ports::folder_scan::ScanFolder;

/// One skill or command of a scope folder, without its body.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedEntry {
    /// The skill or command without its body.
    pub invocable: JsObject,
    /// Reads the body now, through the same `FolderScan` port; see `BodyReader` for the contract.
    pub read_body: BodyReader,
    /// The scope it was read from.
    pub scope: ScanScope,
    /// The skill folder's name, or the command's name: the other name a skill is typed by.
    pub dir_name: String,
    /// The base directory the entry was read below.
    pub base: String,
    /// Where the folder (or file) really lives; two entries with one target are one skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// A skill folder that is also a plugin, `<name>@skills-dir` (skills.md, "Skill folder as a
/// plugin").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsDirectoryPlugin {
    /// `<name>@skills-dir`.
    pub id: String,
    /// The plugin's name: its manifest's, else its folder's.
    pub name: String,
    /// The folder's host path; `FolderScan.open` gives the files to load it with.
    pub dir: String,
    /// The scope it was read from.
    pub scope: ScanScope,
    /// A project's plugin loads only after the workspace trust dialog is accepted.
    pub requires_trust: bool,
}

/// What a scan found.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOutcome {
    /// The skills and commands, in scan order.
    pub entries: Vec<ScannedEntry>,
    /// The skill folders that are also plugins.
    pub plugins: Vec<SkillsDirectoryPlugin>,
    /// What was found reading them.
    pub issues: Vec<PluginIssue>,
    /// The skills folders the session watches for changes (those that exist).
    pub watch: Vec<String>,
}

/// `EMPTY`: a scan that found nothing.
#[must_use]
pub fn empty() -> ScanOutcome {
    ScanOutcome::default()
}

/// The parts one after the other.
#[must_use]
pub fn merge(parts: Vec<ScanOutcome>) -> ScanOutcome {
    parts.into_iter().fold(empty(), |mut all, part| {
        all.entries.extend(part.entries);
        all.plugins.extend(part.plugins);
        all.issues.extend(part.issues);
        all.watch.extend(part.watch);
        all
    })
}

/// One base directory of one scope, opened.
#[derive(Clone)]
pub struct Where<'a> {
    /// What the scan reads with.
    pub context: &'a ScanContext,
    /// The scope's rule.
    pub rule: ScopeRule,
    /// The scope.
    pub scope: ScanScope,
    /// The base directory, opened.
    pub folder: Arc<dyn ScanFolder>,
    /// Where the base sits in the scope and which scope folder below it is read.
    pub place: ScopePlace,
}
