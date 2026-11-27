//! Port of `src/marketplace/install-records-at.ts`, switched.
//!
//! Which install records apply in one project folder (plugins/install.md, "Choose an install
//! scope"; plugins/overview.md, "Understand install scopes"): a user-scope install is "enabled for
//! you in every project on this machine", a project- or local-scope one in "this repository" only,
//! the folder its `projectPath` names ("Project the install belongs to", plugins/cli-reference.md).
//! Pure over the records `parseInstalledPlugins` read. Judgments (spec/judgments.md): a managed
//! record applies everywhere, like a user one; a project or local record without `projectPath`
//! belongs to no folder; the folders compare as `selectUninstall` compares them (`samePath`).

use indexmap::IndexMap;
use serde::Deserialize;

use super::command_output::same_path;
use crate::format::path_text::PathPlatform;
use crate::js::JsValue;

/// Where the session runs: the project folder whose `.claude/` settings it reads, if any.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallsAt {
    /// The project folder.
    #[serde(default)]
    pub project_path: Option<String>,
    /// Whose path rules apply.
    pub platform: PathPlatform,
}

/// Whether a scope's records apply everywhere or only in their own folder.
fn everywhere(scope: Option<&str>) -> bool {
    matches!(scope, Some("user" | "managed"))
}

fn applies_at(one: &JsValue, at: &InstallsAt) -> bool {
    if everywhere(one.get("scope").and_then(JsValue::as_str)) {
        return true;
    }
    match (
        at.project_path.as_deref(),
        one.get("projectPath").and_then(JsValue::as_str),
    ) {
        (Some(project_path), Some(record_path)) => {
            same_path(record_path, project_path, at.platform)
        }
        _ => false,
    }
}

/// The records that apply at `at`, by plugin id in file order; a plugin none of whose records apply is left out.
#[must_use]
pub fn installs_at(
    records: IndexMap<String, Vec<JsValue>>,
    at: &InstallsAt,
) -> IndexMap<String, Vec<JsValue>> {
    records
        .into_iter()
        .map(|(id, list)| {
            (
                id,
                list.into_iter()
                    .filter(|one| applies_at(one, at))
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

crate::registry::exports! {
    "marketplace/install-records-at" {
        "installsAt" => ok |records: IndexMap<String, Vec<JsValue>>, at: InstallsAt|
            installs_at(records, &at);
    }
}
