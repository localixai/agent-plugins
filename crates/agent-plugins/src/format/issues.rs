//! Port of `src/format/issues.ts`, switched.
//! What a check found, in the shape `claude plugin validate` reports
//! (plugins/manifest-reference.md, "Validate the manifest"): an error keeps the plugin from
//! loading, a warning does not. `path` names the field (`userConfig.api_token`) or the file the
//! issue is about.

use serde::{Deserialize, Serialize};

/// How much an issue weighs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Keeps the plugin from loading.
    Error,
    /// Reported; the plugin loads.
    Warning,
}

/// Both severities, in the package's order.
pub const SEVERITIES: [Severity; 2] = [Severity::Error, Severity::Warning];

/// One finding about a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginIssue {
    /// How much it weighs.
    pub severity: Severity,
    /// The field or the file it is about.
    pub path: String,
    /// What is wrong.
    pub message: String,
    /// The advice the `/plugin` Errors tab shows under the message, when Claude Code gives one
    /// (plugins/troubleshooting.md, "commands path not found"). A host shows it with the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
}

/// An error about `path`.
#[must_use]
pub fn error(path: &str, message: &str) -> PluginIssue {
    PluginIssue {
        severity: Severity::Error,
        path: path.to_owned(),
        message: message.to_owned(),
        guidance: None,
    }
}

/// A warning about `path`.
#[must_use]
pub fn warning(path: &str, message: &str) -> PluginIssue {
    PluginIssue {
        severity: Severity::Warning,
        path: path.to_owned(),
        message: message.to_owned(),
        guidance: None,
    }
}

/// Whether any issue keeps the plugin from loading.
#[must_use]
pub fn blocks_loading(issues: &[PluginIssue]) -> bool {
    issues.iter().any(|found| found.severity == Severity::Error)
}

crate::registry::exports! {
    "format/issues" {
        "SEVERITIES" => ok || SEVERITIES;
        "error" => ok |path: String, message: String| error(&path, &message);
        "warning" => ok |path: String, message: String| warning(&path, &message);
        "blocksLoading" => ok |issues: Vec<PluginIssue>| blocks_loading(&issues);
    }
}
