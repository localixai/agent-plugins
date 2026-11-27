//! Port of `src/settings/broken-file.ts`, switched.
//!
//! How a broken settings source is reported (settings.md, "Fix a broken settings file"; errors.md,
//! "Managed settings document could not be parsed"). The docs name what the person sees:
//!
//! - **Settings Error**: a user, project or local file has invalid JSON; "Claude Code skips" it and
//!   keeps going without its settings.
//! - **Settings Warning**: only individual entries fail; "Claude Code skips those values and keeps
//!   the rest of the file in effect" (the repairs of `repair.ts` and a key the file may not carry).
//! - **Managed settings**: the document that is not JSON puts none of its settings in effect, and
//!   Claude Code exits at startup rather than run without the policy (`stops_startup`).

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::issues::{PluginIssue, Severity, error};
use crate::settings::layers::PolicyScope;

const MANAGED_PARSE: &str = "Managed settings document could not be parsed as a JSON object; none of its settings are in effect. Fix or remove it.";

/// How a report names a source (settings.md, "Settings files and who they affect").
pub const SCOPE_LABEL: [(PolicyScope, &str); 7] = [
    (PolicyScope::Managed, "managed settings"),
    (PolicyScope::Parent, "parent settings"),
    (PolicyScope::Flag, "--settings"),
    (PolicyScope::Local, "project local settings"),
    (PolicyScope::Project, "shared project settings"),
    (PolicyScope::AddDir, "an --add-dir directory"),
    (PolicyScope::User, "user settings"),
];

/// `SCOPE_LABEL[scope]`.
#[must_use]
pub fn scope_label(scope: PolicyScope) -> &'static str {
    SCOPE_LABEL
        .iter()
        .find(|(each, _)| *each == scope)
        .map_or("", |(_, label)| label)
}

/// The sources whose broken document is a managed one: the policy tier.
const fn is_policy_tier(scope: PolicyScope) -> bool {
    match scope {
        PolicyScope::Managed | PolicyScope::Parent => true,
        PolicyScope::Flag
        | PolicyScope::Local
        | PolicyScope::Project
        | PolicyScope::AddDir
        | PolicyScope::User => false,
    }
}

/// Whether a broken source stops Claude Code (errors.md, "Managed settings document could not be
/// parsed": it "exits with code 1 at startup instead of running without the policy the document
/// carries"). The docs say it of the managed documents; the kit holds the parent settings of an
/// embedding host, also policy, to the same rule (spec/judgments.md). Any other source is skipped.
#[must_use]
pub const fn stops_startup(scope: PolicyScope) -> bool {
    is_policy_tier(scope)
}

/// A source that is present and is not a JSON object: `reason` says why when the text did not
/// parse. A user, project, local or `--settings` file is skipped. A managed document is fatal: the
/// line names the failed source (`source`, the path or profile the host read, when it says) before
/// the message, as errors.md shows it.
#[must_use]
pub fn broken_source(
    scope: PolicyScope,
    reason: Option<&str>,
    source: Option<&str>,
) -> PluginIssue {
    let why = reason.map_or_else(String::new, |reason| format!(" ({reason})"));
    let path = format!("{}:", scope.name());
    if is_policy_tier(scope) {
        let named = source.map_or_else(String::new, |source| format!("{source}: "));
        return error(&path, &format!("{named}{MANAGED_PARSE}{why}"));
    }
    error(
        &path,
        &format!(
            "Settings Error: {} is not a valid JSON object and is skipped{why}",
            scope_label(scope)
        ),
    )
}

/// What `/status` and the start-up dialog call a problem (settings.md, "Fix a broken settings
/// file").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ProblemKind {
    /// A file that does not parse.
    #[serde(rename = "Settings Error")]
    SettingsError,
    /// An entry that fails.
    #[serde(rename = "Settings Warning")]
    SettingsWarning,
    /// Anything in the policy tier.
    #[serde(rename = "Managed settings")]
    ManagedSettings,
}

const fn by_severity(severity: Severity) -> ProblemKind {
    match severity {
        Severity::Error => ProblemKind::SettingsError,
        Severity::Warning => ProblemKind::SettingsWarning,
    }
}

/// Which of the documented kinds a problem found in `scope` is: an entry that fails is a Settings
/// Warning, a file that does not parse a Settings Error, and anything in the policy tier is
/// reported as managed settings, which keep enforcing the rest of the file.
#[must_use]
pub const fn problem_kind(scope: PolicyScope, issue: &PluginIssue) -> ProblemKind {
    if is_policy_tier(scope) {
        ProblemKind::ManagedSettings
    } else {
        by_severity(issue.severity)
    }
}

crate::registry::exports! {
    "settings/broken-file" {
        "SCOPE_LABEL" => ok || SCOPE_LABEL.iter().map(|(scope, label)| (scope.name(), *label)).collect::<IndexMap<&str, &str>>();
        "stopsStartup" => ok |scope: PolicyScope| stops_startup(scope);
        "brokenSource" => ok |scope: PolicyScope, reason: Option<String>, source: Option<String>| broken_source(scope, reason.as_deref(), source.as_deref());
        "problemKind" => ok |scope: PolicyScope, issue: PluginIssue| problem_kind(scope, &issue);
    }
}
