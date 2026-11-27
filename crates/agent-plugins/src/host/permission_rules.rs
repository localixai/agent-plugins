//! Port of `src/host/permission-rules.ts`, switched.
//!
//! The permission rules of a settings view as the rules module reads them, so a host that rebuilds
//! its permissions for each run (a new view, a new trust answer) needs no table of its own. Settings
//! and rules are one layer and never import each other (docs/ARCHITECTURE.md, "Layers"); this
//! joins them from above. The view is the host's object (`PolicyView`), and the facts cross as a
//! host value too, since `trusted` is the host's function: their data members are read by name.

use serde::Serialize;

use crate::format::issues::PluginIssue;
use crate::registry::{Callback, Port};
use crate::rules::parse::{RuleOrigin, parse_rule};
use crate::rules::types::{PermissionBehavior, PermissionRule, RuleSource};
use crate::settings::general::permission_rules::{
    AddedDirectory, LocalFileFacts, PermissionRuleLists, PolicyScope, RuleEntry,
    permission_rule_lists,
};

/// `PermissionRuleFacts`, as read from the host's object: the user's trust decision for the
/// folder of a repository file (`permissionRuleLists`), the local file's facts, and the
/// `--settings` file as the host names it (a path, or the inline JSON's label).
struct PermissionRuleFacts {
    trusted: Callback,
    local: LocalFileFacts,
    flag_settings: String,
}

/// What `permissionRulesOf` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionRulesOf {
    /// Every rule that applies, deny first, then ask, then allow; each with the source of its file.
    pub rules: Vec<PermissionRule>,
    /// The added directories.
    pub directories: Vec<AddedDirectory>,
    /// What the rules parser says about a rule it skips or ignores, each path led by the scope.
    pub issues: Vec<PluginIssue>,
}

/// Where each scope's rules were written, in the rules module's terms (permissions.md, "Settings
/// precedence" and "Managed settings"). The host's parent settings are the embedder's managed
/// policy. A `--add-dir` directory carries no permission rule, so it has no source.
fn source_of(scope: PolicyScope, facts: &PermissionRuleFacts) -> Option<RuleSource> {
    match scope {
        PolicyScope::Managed => Some(RuleSource::Managed),
        PolicyScope::Parent => Some(RuleSource::EmbedderManaged),
        PolicyScope::Flag => Some(RuleSource::FlagSettings {
            file: facts.flag_settings.clone(),
        }),
        PolicyScope::Local => Some(RuleSource::Local),
        PolicyScope::Project => Some(RuleSource::Project),
        PolicyScope::User => Some(RuleSource::User),
        PolicyScope::AddDir => None,
    }
}

/// A scope as the issue path names it (`PolicyScope`'s string).
const fn scope_name(scope: PolicyScope) -> &'static str {
    match scope {
        PolicyScope::Managed => "managed",
        PolicyScope::Parent => "parent",
        PolicyScope::Flag => "flag",
        PolicyScope::Local => "local",
        PolicyScope::Project => "project",
        PolicyScope::User => "user",
        PolicyScope::AddDir => "addDir",
    }
}

const BEHAVIORS: [PermissionBehavior; 3] = [
    PermissionBehavior::Deny,
    PermissionBehavior::Ask,
    PermissionBehavior::Allow,
];

/// The entries of one behaviour of the lists.
fn entries_of(lists: &PermissionRuleLists, behavior: PermissionBehavior) -> &[RuleEntry] {
    match behavior {
        PermissionBehavior::Allow => &lists.allow,
        PermissionBehavior::Deny => &lists.deny,
        PermissionBehavior::Ask => &lists.ask,
    }
}

/// The facts' members, read from the host's object.
async fn facts_of(facts: Port) -> PermissionRuleFacts {
    PermissionRuleFacts {
        trusted: facts.member("trusted").await,
        local: facts.member("local").await,
        flag_settings: facts.member("flagSettings").await,
    }
}

/// The permission rules of `view`, parsed with the source of each scope's file.
pub async fn permission_rules_of(view: Port, facts: Port) -> PermissionRulesOf {
    let facts = facts_of(facts).await;
    let lists = permission_rule_lists(view, facts.trusted, facts.local).await;
    let mut rules = Vec::new();
    let mut issues = Vec::new();
    for behavior in BEHAVIORS {
        for entry in entries_of(&lists, behavior) {
            let Some(source) = source_of(entry.scope, &facts) else {
                continue;
            };
            let read = parse_rule(&entry.rule, &RuleOrigin { behavior, source });
            rules.extend(read.rule);
            issues.extend(read.issues.into_iter().map(|issue| PluginIssue {
                path: format!("{}:{}", scope_name(entry.scope), issue.path),
                ..issue
            }));
        }
    }
    PermissionRulesOf {
        rules,
        directories: lists.additional_directories,
        issues,
    }
}

crate::registry::exports! {
    "host/permission-rules" {
        "permissionRulesOf" => async ok |view: Port, facts: Port|
            permission_rules_of(view, facts).await;
    }
}
