//! Port of `src/settings/general/permission-rules.ts`, switched.
//!
//! The permission rules every settings file adds up to, and the directories they add. The view of
//! the layers (`PolicyView`, `src/settings/view.ts`) is the host's object, asked by its methods
//! `get` and `eachFromHighest`; whether the user trusts a scope's folder is the host's callback.

use serde::{Deserialize, Serialize};

use crate::js::{JsValue, is_truthy};
use crate::protocol::permission_modes::PERMISSION_BEHAVIORS;
use crate::registry::{Callback, Port};

/// Where a settings layer comes from (`PolicyScope` of `src/settings/layers.ts`, `POLICY_SCOPES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyScope {
    /// Managed settings.
    Managed,
    /// The embedding host's parent settings.
    Parent,
    /// `--settings`.
    Flag,
    /// `.claude/settings.local.json`.
    Local,
    /// `.claude/settings.json`.
    Project,
    /// `~/.claude/settings.json`.
    User,
    /// An added directory's settings.
    AddDir,
}

/// A rule as written in a settings file, with the file's scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleEntry {
    /// The rule.
    pub rule: String,
    /// The file's scope.
    pub scope: PolicyScope,
}

/// A directory `permissions.additionalDirectories` adds, with the file's scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AddedDirectory {
    /// The directory.
    pub directory: String,
    /// The file's scope.
    pub scope: PolicyScope,
}

/// The rules of each behaviour, highest scope first, a rule written in two files once, and the
/// added directories.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleLists {
    /// `allow` rules.
    pub allow: Vec<RuleEntry>,
    /// `deny` rules.
    pub deny: Vec<RuleEntry>,
    /// `ask` rules.
    pub ask: Vec<RuleEntry>,
    /// The added directories.
    pub additional_directories: Vec<AddedDirectory>,
}

/// What the host learned about the local settings file. permissions.md ("When your local settings
/// file needs trust"): the file is normally your own, so "Claude Code applies its allow rules and
/// additional directories without the trust step. When the file is tracked in git, or `.claude` is
/// a symlink, Claude Code treats it as repository-supplied instead and holds its rules until you
/// trust the folder." Claude Code runs git only once the folder is trusted: before that the host has
/// no answer and says `tracked: true`, which holds the file like project settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileFacts {
    /// `.claude/settings.local.json` is tracked in git.
    pub tracked: bool,
    /// `.claude` is a symlink.
    pub claude_dir_is_symlink: bool,
}

/// Whether the files of a scope wait for the user's trust of the folder: the shared project file
/// always (permissions.md, "Project allow rules and workspace trust"), the local file only when it
/// is repository-supplied, every other source never (`WAITS_FOR_TRUST`).
const fn waits_for_trust(scope: PolicyScope, local: LocalFileFacts) -> bool {
    match scope {
        PolicyScope::Managed
        | PolicyScope::Parent
        | PolicyScope::Flag
        | PolicyScope::User
        | PolicyScope::AddDir => false,
        PolicyScope::Local => local.tracked || local.claude_dir_is_symlink,
        PolicyScope::Project => true,
    }
}

/// What `allowManagedPermissionRulesOnly` keeps of one layer. Managed settings keep every rule. The
/// embedding host's parent settings "count as part of the managed tier" but their `allow` rules are
/// dropped, "and keeps their `deny` and `ask` rules". Every other source loses all of them
/// (`KEPT_UNDER_LOCK`).
const fn kept_under_lock(scope: PolicyScope) -> &'static [&'static str] {
    match scope {
        PolicyScope::Managed => PERMISSION_BEHAVIORS,
        PolicyScope::Parent => &["deny", "ask"],
        PolicyScope::Flag
        | PolicyScope::Local
        | PolicyScope::Project
        | PolicyScope::User
        | PolicyScope::AddDir => &[],
    }
}

/// A `Read` or `Edit` rule whose pattern starts with `!` carves paths out of the rules above it.
/// settings-reference.md, `allowManagedPermissionRulesOnly`: a host's parent settings keep their
/// `deny` and `ask` rules "except `Read` and `Edit` rules whose pattern starts with `!`. A host
/// can't carve paths out of the managed rules with a `!` rule, whether or not you set this key."
/// (`CARVE_OUT`, `/^(?:Read|Edit)\(!/`).
fn carve_out(rule: &str) -> bool {
    rule.starts_with("Read(!") || rule.starts_with("Edit(!")
}

fn is_host_carve_out(scope: PolicyScope, behavior: &str, rule: &str) -> bool {
    scope == PolicyScope::Parent && behavior != "allow" && carve_out(rule)
}

fn text_list(permissions: &JsValue, name: &str) -> Vec<String> {
    match permissions.get(name) {
        Some(JsValue::Array(list)) => list
            .iter()
            .filter_map(JsValue::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// A layer that sets `permissions` (`KeyHolder` of `src/settings/layers.ts`).
#[derive(Clone, Debug, Deserialize)]
pub struct KeyHolder {
    /// Its scope.
    pub scope: PolicyScope,
    /// The value it sets.
    pub value: JsValue,
}

/// `held(scope)`: the scope's files wait for trust and the user has not given it; the host is asked
/// only for a scope that waits.
async fn held(scope: PolicyScope, trusted: Callback, local: LocalFileFacts) -> bool {
    if !waits_for_trust(scope, local) {
        return false;
    }
    let scope = crate::js::value::to_js_value(&scope).unwrap_or_default();
    !is_truthy(&trusted.call::<JsValue>(vec![scope]).await)
}

/// What one behaviour's rules add up to.
struct Lists<'a> {
    holders: &'a [KeyHolder],
    lock: bool,
    trusted: Callback,
    local: LocalFileFacts,
}

impl Lists<'_> {
    async fn entries_of(&self, behavior: &str) -> Vec<RuleEntry> {
        let mut entries: Vec<RuleEntry> = Vec::new();
        for holder in self.holders {
            if self.lock && !kept_under_lock(holder.scope).contains(&behavior) {
                continue;
            }
            if behavior == "allow" && held(holder.scope, self.trusted, self.local).await {
                continue;
            }
            entries.extend(
                text_list(&holder.value, behavior)
                    .into_iter()
                    .filter(|rule| !is_host_carve_out(holder.scope, behavior, rule))
                    .map(|rule| RuleEntry {
                        rule,
                        scope: holder.scope,
                    }),
            );
        }
        // The highest file's entry stands for a rule several files write.
        let mut kept: Vec<RuleEntry> = Vec::new();
        for entry in entries {
            if !kept.iter().any(|one| one.rule == entry.rule) {
                kept.push(entry);
            }
        }
        kept
    }
}

/// The permission rules every settings file adds up to (settings.md, "Lists merge instead of
/// overriding": `permissions.allow` combines across scopes; permissions.md, "Settings precedence": a
/// level cannot remove another's rule, so a managed deny stays whatever the others say). When the
/// managed tier sets `allowManagedPermissionRulesOnly` the other sources add none
/// (settings-reference.md). A repository's `allow` rules and `additionalDirectories` apply only once
/// the folder is trusted (the local file only when `local` says it is repository-supplied); `deny`
/// and `ask` rules "only restrict" and apply at once.
pub async fn permission_rule_lists(
    view: Port,
    trusted: Callback,
    local: LocalFileFacts,
) -> PermissionRuleLists {
    let locked: JsValue = view
        .method("get", vec!["allowManagedPermissionRulesOnly".into()])
        .await;
    let lock = locked == JsValue::Bool(true);
    let holders: Vec<KeyHolder> = view
        .method("eachFromHighest", vec!["permissions".into()])
        .await;
    let lists = Lists {
        holders: &holders,
        lock,
        trusted,
        local,
    };
    let mut directories: Vec<AddedDirectory> = Vec::new();
    for holder in &holders {
        if lock && holder.scope != PolicyScope::Managed {
            continue;
        }
        if held(holder.scope, trusted, local).await {
            continue;
        }
        directories.extend(
            text_list(&holder.value, "additionalDirectories")
                .into_iter()
                .map(|directory| AddedDirectory {
                    directory,
                    scope: holder.scope,
                }),
        );
    }
    let mut additional_directories: Vec<AddedDirectory> = Vec::new();
    for entry in directories {
        if !additional_directories
            .iter()
            .any(|one| one.directory == entry.directory)
        {
            additional_directories.push(entry);
        }
    }
    PermissionRuleLists {
        allow: lists.entries_of("allow").await,
        deny: lists.entries_of("deny").await,
        ask: lists.entries_of("ask").await,
        additional_directories,
    }
}

crate::registry::exports! {
    "settings/general/permission-rules" {
        "permissionRuleLists" => async ok |view: Port, trusted: Callback, local: LocalFileFacts|
            permission_rule_lists(view, trusted, local).await;
    }
}
