//! Port of `src/settings/permissions/policy.ts`, switched.
//!
//! The permission keys of the settings as one object over a view (docs/API.md, "Settings"). The
//! host holds `startingMode` and `runningMode` as functions of the core's.

use std::sync::Arc;

use serde::Serialize;

use super::auto_mode::{AutoModeSettings, auto_mode_settings};
use super::default_mode::{StartInput, StartSettings, starting_mode};
use super::managed_only::{SettingsDirectory, SettingsRule, permission_lists_of};
use super::running_mode::{PermissionMode, running_mode_of};
use crate::js::JsValue;
use crate::registry::{Function, Port};
use crate::settings::view::PolicyView;

/// The permission policy as the host holds it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's PermissionsPolicy holds one flag per key"
)]
pub struct PermissionsPolicy {
    /// `allowManagedPermissionRulesOnly`: managed settings are the only source of permission rules.
    pub managed_rules_only: bool,
    /// The rules in force, as written, highest file first; the host parses each with `parseRule`.
    pub rules: Vec<SettingsRule>,
    /// The directories `permissions.additionalDirectories` adds, each once.
    pub additional_directories: Vec<SettingsDirectory>,
    /// A `true` in any file: the file tools refuse reads outside the working directories.
    pub block_reads_outside_working_directories: bool,
    /// A `true` in any file: `Write`, `Edit` and `NotebookEdit` refuse paths outside the working
    /// directories.
    pub block_writes_outside_working_directories: bool,
    /// `permissions.denyInsteadOfPrompting`: a `true` from user, `--settings` or managed settings.
    /// A call that would wait on a prompt is denied; project and local files are not read for it.
    pub deny_instead_of_prompting: bool,
    /// `permissions.disableBypassPermissionsMode` is `"disable"` in any file.
    pub disable_bypass_permissions_mode: bool,
    /// `disableAutoMode` (or `permissions.disableAutoMode`) is `"disable"` in any file.
    pub disable_auto_mode: bool,
    /// `skipDangerousModePermissionPrompt` is `true` in any file that carries it.
    pub skip_dangerous_mode_permission_prompt: bool,
    /// The auto mode's settings.
    pub auto_mode: AutoModeSettings,
    /// The mode a new session starts in.
    pub starting_mode: Function,
    /// What a session running in `current` changes to once these settings reach it: an auto
    /// session leaves auto under a managed `disableAutoMode`, with `auto mode disabled by
    /// settings`.
    pub running_mode: Function,
}

struct Starting(Arc<dyn PolicyView>, StartSettings);

crate::registry::methods! {
    Starting {
        "call" => sync async ok |this, input: StartInput| starting_mode(&*this.0, &this.1, &input).await;
    }
}

struct Running(Arc<dyn PolicyView>);

crate::registry::methods! {
    Running {
        "call" => sync async ok |this, current: PermissionMode| running_mode_of(&*this.0, current).await;
    }
}

fn is_disable(value: Option<&JsValue>) -> bool {
    value.and_then(JsValue::as_str) == Some("disable")
}

async fn is_true(view: &dyn PolicyView, key: &str) -> bool {
    view.get(key).await == Some(JsValue::Bool(true))
}

/// The policy over `view`; `context` is the host's `PolicyContext`.
pub async fn create_permissions_policy(
    view: Arc<dyn PolicyView>,
    context: &Port,
) -> PermissionsPolicy {
    let block = view.get("permissions").await;
    let lists = permission_lists_of(&*view, context).await;
    let top = view.get("disableAutoMode").await;
    let block_member = |name: &str| block.as_ref().and_then(|block| block.get(name));
    let disable_auto_mode = is_disable(top.as_ref()) || is_disable(block_member("disableAutoMode"));
    let disable_bypass_permissions_mode = is_disable(block_member("disableBypassPermissionsMode"));
    let skip = is_true(&*view, "skipDangerousModePermissionPrompt").await;
    PermissionsPolicy {
        managed_rules_only: lists.managed_only,
        rules: lists.rules,
        additional_directories: lists.directories,
        block_reads_outside_working_directories: is_true(
            &*view,
            "permissions.blockReadsOutsideWorkingDirectories",
        )
        .await,
        block_writes_outside_working_directories: is_true(
            &*view,
            "permissions.blockWritesOutsideWorkingDirectories",
        )
        .await,
        deny_instead_of_prompting: is_true(&*view, "permissions.denyInsteadOfPrompting").await,
        disable_bypass_permissions_mode,
        disable_auto_mode,
        skip_dangerous_mode_permission_prompt: skip,
        auto_mode: auto_mode_settings(&*view).await,
        starting_mode: Function::new(Starting(
            Arc::clone(&view),
            StartSettings {
                disable_auto_mode,
                disable_bypass_permissions_mode,
                skip_dangerous_mode_permission_prompt: skip,
            },
        )),
        running_mode: Function::new(Running(view)),
    }
}

crate::registry::exports! {
    "settings/permissions/policy" {
        "createPermissionsPolicy" => async ok |view: Port, context: Port|
            create_permissions_policy(Arc::new(view), &context).await;
    }
}
