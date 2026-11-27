//! The core's counterpart of `src/settings/general/rows.ts` at sha256:fc290bc73fc3, which stays
//! TypeScript: its rows hold functions (`read`, `admin.pinnedWhen`, `merge.combine`) that the
//! package's `createPolicyView` (`src/settings/view.ts`) calls, so they cannot cross the boundary
//! as data. The same rows, each function a Rust one, so the core asks a general key's row without
//! the TypeScript table.
//!
//! One row per general key, from its entry in settings-reference.md (the Scope line and the notes
//! under it) and settings.md, "Lists merge instead of overriding" and "Exceptions to managed
//! settings precedence". A key with no row is combined by the general rule: blocks merge, lists
//! union, a single value comes from the highest file.

use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::js::{JsValue, strict_equals};
use crate::protocol::is_record::is_record;
use crate::protocol::permission_modes::PERMISSION_MODES;
use crate::settings::general::exceptions::{
    FALSE_FROM_ANY_SCOPE, LOWEST_CAP, ManagedException, TRUE_FROM_ANY_SCOPE,
    TRUE_FROM_USER_OR_FLAG, Unreadable,
};
use crate::settings::general::sandbox_credentials::is_deny_entry;
use crate::settings::key_spec::ScopeRule;
use crate::settings::layers::PolicyScope;
use crate::settings::merge::MERGES;
use crate::settings::paths::{SettingsPath, value_at};
use crate::settings::value_spec::{AdminTier, PinnedWhen, ValueSpec, spec};

fn at(settings: &JsValue, path: &[&str]) -> JsValue {
    let names = path.iter().map(|name| (*name).to_owned()).collect();
    value_at(settings, &SettingsPath::Names(names))
}

/// A row for a key inside a block: read at `path`, and written back there in the merged settings.
fn nested(
    path: &'static [&'static str],
    row: ValueSpec,
    is_value: fn(&JsValue) -> bool,
) -> ValueSpec {
    ValueSpec {
        path: Some(path),
        read: Some(Box::new(move |settings| {
            let found = at(settings, path);
            if is_value(&found) {
                found
            } else {
                JsValue::Undefined
            }
        })),
        ..row
    }
}

const fn is_boolean(value: &JsValue) -> bool {
    value.is_boolean()
}
const fn is_text(value: &JsValue) -> bool {
    value.is_string()
}
fn is_text_list(value: &JsValue) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(is_text))
}
fn is_default_mode(value: &JsValue) -> bool {
    PERMISSION_MODES
        .iter()
        .any(|mode| strict_equals(&JsValue::from(*mode), value))
}

/// `remoteControlAtStartup`: a project or local `false` beats a managed `true`; their `true` is ignored.
static REMOTE_CONTROL: LazyLock<ManagedException> = LazyLock::new(|| ManagedException {
    ladder: vec![JsValue::Bool(true), JsValue::Bool(false)],
    honored_from: vec![PolicyScope::Project, PolicyScope::Local],
    only_when_stricter: vec![PolicyScope::Project, PolicyScope::Local],
    unreadable: None,
});

/// `crossSessionInbound`: a stricter project or local value applies on the `accept` < `hold` < `refuse` ladder.
static CROSS_SESSION: LazyLock<ManagedException> = LazyLock::new(|| ManagedException {
    ladder: ["accept", "hold", "refuse"].map(JsValue::from).to_vec(),
    honored_from: vec![PolicyScope::Project, PolicyScope::Local],
    only_when_stricter: vec![PolicyScope::Project, PolicyScope::Local],
    // An unrecognized value in a user, project, local or `--settings` file holds inbound messages.
    unreadable: Some(Unreadable {
        floor: JsValue::from("hold"),
        from: vec![
            PolicyScope::Flag,
            PolicyScope::Local,
            PolicyScope::Project,
            PolicyScope::User,
        ],
    }),
});

/// `sandbox.filesystem.disabled`: "When managed settings configure `sandbox.filesystem` at all, or
/// list any `sandbox.credentials.files` entry with `"mode": "deny"`, only managed settings can set
/// the key." A degraded entry is a `deny` entry once repaired, so it pins the key the same way.
fn pins_filesystem_disabled(managed: &[JsValue]) -> bool {
    managed.iter().any(|settings| {
        let files = at(settings, &["sandbox", "credentials", "files"]);
        let denies = files
            .as_array()
            .is_some_and(|entries| entries.iter().any(is_deny_entry));
        is_record(&at(settings, &["sandbox", "filesystem"])) || denies
    })
}

/// A lock of the managed settings: `true` at `path` in any of them.
fn locked_by(path: &'static [&'static str]) -> PinnedWhen {
    Box::new(move |managed| {
        managed
            .iter()
            .any(|settings| at(settings, path) == JsValue::Bool(true))
    })
}

fn pinned(pinned_when: PinnedWhen) -> AdminTier {
    AdminTier {
        pinned_when: Some(pinned_when),
        ..AdminTier::default()
    }
}

/// `enableArtifact`: "`disableArtifact: true` from any scope" counts as `enableArtifact: false`.
fn read_enable_artifact(settings: &JsValue) -> JsValue {
    if settings.get("disableArtifact") == Some(&JsValue::Bool(true)) {
        JsValue::Bool(false)
    } else {
        settings.get("enableArtifact").cloned().unwrap_or_default()
    }
}

/// The table as the TypeScript writes it: `key: scopes, merge`, a `nested` row's path and value
/// test after `@`, then the row's other members (`exception`, `admin`, `read`) as written there.
macro_rules! general_specs {
    ($($key:literal: $scopes:ident, $merge:ident $(@ [$($name:literal),*] $is_value:ident)?
        $(, $field:ident: $value:expr)*;)*) => {
        IndexMap::from([$((
            $key,
            {
                let row = ValueSpec { $($field: $value,)* ..spec(ScopeRule::$scopes, MERGES.$merge) };
                $(let row = nested(&[$($name),*], row, $is_value);)?
                row
            },
        ),)*])
    };
}

/// One row per general key (the module comment), in the TypeScript's order.
pub static GENERAL_SPECS: LazyLock<IndexMap<&'static str, ValueSpec>> = LazyLock::new(|| {
    general_specs! {
        // "Any file" for the keys below; a value is whole (an object such as `statusLine` never mixes the
        // fields of two files), so the highest file that sets it is the value.
        "agent": AnyAndParent, highest_wins;
        "disableAgentView": AnyAndParent, highest_wins;
        "fileSuggestion": AnyAndParent, highest_wins;
        "outputStyle": AnyAndParent, highest_wins;
        // "User or managed" (settings-reference.md, `processWrapper`): a repository's file "never" supplies it.
        "processWrapper": UserOrManaged, highest_wins;
        "statusLine": AnyAndParent, highest_wins;
        "subagentStatusLine": AnyAndParent, highest_wins;
        "teammateMode": AnyAndParent, highest_wins;
        "theme": AnyAndParent, highest_wins;
        // "Any file"; blocks merge by the general rule (the object form of `false` takes the highest file).
        "attribution": AnyAndParent, deep_merge;
        // "User or managed" (settings-reference.md, `autoMode`); `classifyAllShell` is "read wherever `autoMode` is read".
        "autoMode.classifyAllShell": UserManagedFlag, highest_wins
            @ ["autoMode", "classifyAllShell"] is_boolean, exception: Some(&TRUE_FROM_USER_OR_FLAG);
        // Managed settings define it: that list applies as-is and "blocks a parent-supplied list"
        // (managed-settings.md). Otherwise "the arrays merge as usual".
        "availableModels": AnyAndParent, union_arrays,
            admin: Some(AdminTier { owns: Some(true), ..AdminTier::default() });
        "crossSessionInbound": AnyAndParent, highest_wins, exception: Some(&CROSS_SESSION);
        // "`disableArtifact: true` from any scope" counts as `enableArtifact: false`; its `false` is ignored.
        "enableArtifact": AnyAndParent, highest_wins, exception: Some(&FALSE_FROM_ANY_SCOPE),
            read: Some(Box::new(read_enable_artifact));
        "env": AnyAndParent, deep_merge;
        // "An ordered chain where position carries meaning": the whole value of the highest file.
        "fallbackModel": AnyAndParent, highest_wins;
        "isolatePeerMachines": AnyAndParent, highest_wins, exception: Some(&TRUE_FROM_ANY_SCOPE);
        "maxEffortLevel": AnyAndParent, highest_wins, exception: Some(&LOWEST_CAP);
        "model": AnyAndParent, highest_wins;
        // One lineup, "ignores the key in project and local settings"; the highest of the three wins whole.
        "modelPicker": UserManagedFlag, highest_wins;
        // The embedding host's parent settings may carry rules too (permissions.md, "Settings precedence").
        "permissions.blockReadsOutsideWorkingDirectories": AnyAndParent, highest_wins
            @ ["permissions", "blockReadsOutsideWorkingDirectories"] is_boolean,
            exception: Some(&TRUE_FROM_ANY_SCOPE);
        "permissions.blockWritesOutsideWorkingDirectories": AnyAndParent, highest_wins
            @ ["permissions", "blockWritesOutsideWorkingDirectories"] is_boolean,
            exception: Some(&TRUE_FROM_ANY_SCOPE);
        // The value decides where it is read from (settings.md, "A value you set is ignored").
        "permissions.defaultMode": RepositoryCannotElevate, highest_wins
            @ ["permissions", "defaultMode"] is_default_mode;
        // "User or managed, and `--settings`. A repository can't set it for you" (settings-reference.md);
        // a `true` from `~/.claude/settings.json` or `--settings` beats a managed `false` (settings.md).
        "permissions.denyInsteadOfPrompting": UserManagedFlag, highest_wins
            @ ["permissions", "denyInsteadOfPrompting"] is_boolean,
            exception: Some(&TRUE_FROM_USER_OR_FLAG);
        "remoteControlAtStartup": AnyAndParent, highest_wins, exception: Some(&REMOTE_CONTROL);
        "sandbox": AnyAndParent, deep_merge;
        // "Only managed" locks (managed-settings.md, "Keys only a managed source can set").
        "sandbox.filesystem.allowManagedReadPathsOnly": Managed, highest_wins
            @ ["sandbox", "filesystem", "allowManagedReadPathsOnly"] is_boolean;
        // "User or managed" plus `--settings` (sandboxing.md): a project file cannot switch the isolation off.
        "sandbox.filesystem.disabled": UserManagedFlag, highest_wins
            @ ["sandbox", "filesystem", "disabled"] is_boolean,
            admin: Some(pinned(Box::new(pins_filesystem_disabled)));
        "sandbox.network.allowManagedDomainsOnly": Managed, highest_wins
            @ ["sandbox", "network", "allowManagedDomainsOnly"] is_boolean;
        // "Honor only managed `allowedDomains`" under the lock (managed-settings.md, "Keys only a managed
        // source can set"); without it "the host's ... sandbox allowlists still apply" and the lists merge.
        "sandbox.network.allowedDomains": AnyAndParent, union_arrays
            @ ["sandbox", "network", "allowedDomains"] is_text_list,
            admin: Some(pinned(locked_by(&["sandbox", "network", "allowManagedDomainsOnly"])));
        // "Only `filesystem.allowRead` paths from managed settings are respected" under the lock.
        "sandbox.filesystem.allowRead": AnyAndParent, union_arrays
            @ ["sandbox", "filesystem", "allowRead"] is_text_list,
            admin: Some(pinned(locked_by(&["sandbox", "filesystem", "allowManagedReadPathsOnly"])));
        "spinnerTipsEnabled": AnyAndParent, highest_wins;
        "worktree": AnyAndParent, deep_merge;
        // "`User`, in `~/.claude/settings.json` on the remote machine" (settings-reference.md, `worktree.location`).
        "worktree.location": User, highest_wins @ ["worktree", "location"] is_text;
    }
});
