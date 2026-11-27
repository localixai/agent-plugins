//! Port of `src/settings/permissions/managed-only.ts`, switched.
//!
//! Which settings files contribute permission rules and directories:
//! `allowManagedPermissionRulesOnly` (settings-reference.md), the policy an embedding host passes
//! (the same entry; permissions.md, "Settings precedence") and workspace trust (permissions.md,
//! "Project allow rules and workspace trust"). A rule is plain text with the file it came from;
//! the host parses it with `parseRule`.

use serde::Serialize;

use crate::js::array_join::string_of;
use crate::js::{JsValue, trim_start};
use crate::protocol::permission_modes::PERMISSION_BEHAVIORS;
use crate::registry::{Callback, Port};
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// A rule and the file it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SettingsRule {
    /// `allow`, `deny` or `ask`.
    pub behavior: &'static str,
    /// The rule as written.
    pub text: String,
    /// The file's scope.
    pub scope: PolicyScope,
}

/// A `permissions.additionalDirectories` entry and the file it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDirectory {
    /// The directory.
    pub path: String,
    /// The highest file that lists it.
    pub scope: PolicyScope,
    /// Only repository settings list it: `.claude/settings.json`, and
    /// `.claude/settings.local.json` when git tracks it. Such a directory is a working directory
    /// for permissions but "doesn't count" for `blockReadsOutsideWorkingDirectories` and
    /// `blockWritesOutsideWorkingDirectories` (settings-reference.md).
    pub repository_only: bool,
}

/// The host's `PolicyContext`, read by its members: its facts about the local settings file are
/// data, `isFolderTrusted` the host's answer.
struct Context {
    /// Git tracks `.claude/settings.local.json`, or `.claude` is a symlink.
    local_settings_tracked: Option<bool>,
    /// `.claude/settings.local.json` sits inside a git repository.
    local_settings_in_repository: Option<bool>,
    is_folder_trusted: Callback,
}

impl Context {
    async fn read(port: &Port) -> Self {
        Self {
            local_settings_tracked: port.member("localSettingsTracked").await,
            local_settings_in_repository: port.member("localSettingsInRepository").await,
            is_folder_trusted: port.member("isFolderTrusted").await,
        }
    }

    async fn is_folder_trusted(&self, scope: PolicyScope) -> bool {
        self.is_folder_trusted
            .call(vec![JsValue::from(scope.name())])
            .await
    }
}

/// What one file hands over once the locks and trust are applied.
struct Contribution {
    /// Its `ask` and `deny` rules count.
    restrictions: bool,
    /// Its `allow` rules count.
    allow: bool,
}

/// What the facts about the session decide for a file.
struct Standing {
    managed_only: bool,
    trusted: bool,
}

/// Per file: whether its rules count, and whether its grants (`allow` rules and
/// `additionalDirectories`) do. "Claude Code then ignores `allow`, `ask`, and `deny` rules in
/// user, project, local, and `--settings` files"; the parent settings "drop their `allow` rules
/// and `additionalDirectories`"; project and local grants wait for the trust dialog, "`deny` and
/// `ask` rules aren't affected".
const fn contributions(scope: PolicyScope, standing: &Standing) -> Contribution {
    let open = !standing.managed_only;
    match scope {
        PolicyScope::Managed => Contribution {
            restrictions: true,
            allow: true,
        },
        PolicyScope::Parent => Contribution {
            restrictions: true,
            allow: open,
        },
        PolicyScope::Flag | PolicyScope::User => Contribution {
            restrictions: open,
            allow: open,
        },
        PolicyScope::Local | PolicyScope::Project => Contribution {
            restrictions: open,
            allow: open && standing.trusted,
        },
        PolicyScope::AddDir => Contribution {
            restrictions: false,
            allow: false,
        },
    }
}

/// Whether a file's `additionalDirectories` count. The lock names "rules", so it leaves the
/// directories of user, project, local and `--settings` files alone; the parent settings drop them
/// (settings-reference.md, `allowManagedPermissionRulesOnly`).
const fn directories(scope: PolicyScope, standing: &Standing) -> bool {
    match scope {
        PolicyScope::Managed | PolicyScope::Flag | PolicyScope::User => true,
        PolicyScope::Parent => !standing.managed_only,
        PolicyScope::Local | PolicyScope::Project => standing.trusted,
        PolicyScope::AddDir => false,
    }
}

/// Whether the file is the host's policy, whose `!` rules for Read and Edit are never carved out.
const fn is_host_policy(scope: PolicyScope) -> bool {
    matches!(scope, PolicyScope::Parent)
}

/// A `Read` or `Edit` rule whose pattern starts with `!`: a carve-out of the managed rules
/// (`/^\s*(?:Read|Edit)\(\s*!/`, `\s` JavaScript's blanks).
fn carve_out(text: &str) -> bool {
    let rest = trim_start(text);
    rest.strip_prefix("Read(")
        .or_else(|| rest.strip_prefix("Edit("))
        .is_some_and(|inside| trim_start(inside).starts_with('!'))
}

/// Whether a file's grants may apply: the trust dialog gates the shared project file, and the
/// local file only "when the file is tracked in git, or `.claude` is a symlink" (permissions.md,
/// "When your local settings file needs trust").
async fn trust(scope: PolicyScope, context: &Context) -> bool {
    match scope {
        PolicyScope::Project => context.is_folder_trusted(scope).await,
        PolicyScope::Local => {
            context.local_settings_tracked != Some(true) || context.is_folder_trusted(scope).await
        }
        PolicyScope::Managed
        | PolicyScope::Parent
        | PolicyScope::Flag
        | PolicyScope::User
        | PolicyScope::AddDir => true,
    }
}

/// Whether a file is the repository's, not yours: the shared file, or a local file git tracks or
/// that lies outside a git repository; only a local file git reports as untracked is yours.
fn repository_file(scope: PolicyScope, context: &Context) -> bool {
    match scope {
        PolicyScope::Project => true,
        PolicyScope::Local => {
            context.local_settings_tracked == Some(true)
                || context.local_settings_in_repository != Some(true)
        }
        PolicyScope::Managed
        | PolicyScope::Parent
        | PolicyScope::Flag
        | PolicyScope::User
        | PolicyScope::AddDir => false,
    }
}

/// The rules and directories in force.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionLists {
    /// `allowManagedPermissionRulesOnly` is on.
    pub managed_only: bool,
    /// The rules, highest file first.
    pub rules: Vec<SettingsRule>,
    /// The directories, highest file first.
    pub directories: Vec<SettingsDirectory>,
}

fn texts(value: Option<&JsValue>) -> Vec<String> {
    match value {
        Some(JsValue::Array(items)) => items.iter().map(string_of).collect(),
        _ => Vec::new(),
    }
}

/// The rules of one file that count: the lock, trust and the host's `!` carve-outs applied.
fn rules_of(scope: PolicyScope, block: &JsValue, contribution: &Contribution) -> Vec<SettingsRule> {
    PERMISSION_BEHAVIORS
        .iter()
        .flat_map(|behavior| {
            let counts = if *behavior == "allow" {
                contribution.allow
            } else {
                contribution.restrictions
            };
            let carves_out = is_host_policy(scope) && *behavior != "allow";
            let listed = if counts {
                texts(block.get(behavior))
            } else {
                Vec::new()
            };
            listed
                .into_iter()
                .filter(move |text| !(carves_out && carve_out(text)))
                .map(move |text| SettingsRule {
                    behavior,
                    text,
                    scope,
                })
        })
        .collect()
}

/// The rules and directories in force, in precedence order (highest file first).
async fn permission_lists(view: &dyn PolicyView, context: &Context) -> PermissionLists {
    let managed_only =
        view.get("allowManagedPermissionRulesOnly").await == Some(JsValue::Bool(true));
    let mut rules = Vec::new();
    let mut listed = Vec::new();
    for holder in view.each_from_highest("permissions").await {
        let scope = holder.scope;
        let standing = Standing {
            managed_only,
            trusted: trust(scope, context).await,
        };
        if directories(scope, &standing) {
            listed.extend(
                texts(holder.value.get("additionalDirectories"))
                    .into_iter()
                    .map(|path| SettingsDirectory {
                        path,
                        scope,
                        repository_only: repository_file(scope, context),
                    }),
            );
        }
        rules.extend(rules_of(
            scope,
            &holder.value,
            &contributions(scope, &standing),
        ));
    }
    PermissionLists {
        managed_only,
        rules,
        directories: first_of_each(&listed),
    }
}

/// Each directory once, as the highest file that lists it; repository-only when every file that
/// lists it is.
fn first_of_each(list: &[SettingsDirectory]) -> Vec<SettingsDirectory> {
    list.iter()
        .enumerate()
        .filter(|(index, entry)| {
            list.iter().position(|other| other.path == entry.path) == Some(*index)
        })
        .map(|(_, entry)| SettingsDirectory {
            repository_only: list
                .iter()
                .filter(|other| other.path == entry.path)
                .all(|other| other.repository_only),
            ..entry.clone()
        })
        .collect()
}

crate::registry::exports! {
    "settings/permissions/managed-only" {
        "permissionLists" => async ok |view: Port, context: Port|
            permission_lists(&view, &Context::read(&context).await).await;
    }
}
