//! Port of `src/settings/permissions/running-mode.ts`, switched.
//!
//! permission-modes.md, "Eliminate prompts with auto mode": with `disableAutoMode: "disable"`, "A
//! session already running in auto mode leaves it when the setting reaches that session from an
//! admin-deployed source, and shows `auto mode disabled by settings`". The admin-deployed sources are
//! the managed ones (managed-settings.md, "Which managed source Claude Code uses"); the mode it
//! leaves for is Manual, as a start in auto under the setting is (judgment, spec/judgments.md).

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// The notice the session shows, verbatim.
pub const AUTO_MODE_DISABLED_NOTICE: &str = "auto mode disabled by settings";

/// A permission mode of the official types (`PermissionMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Asks before each action.
    Default,
    /// Accepts file edits.
    AcceptEdits,
    /// Skips permission checks.
    BypassPermissions,
    /// Plans, never edits.
    Plan,
    /// Denies what is not allowed.
    DontAsk,
    /// A classifier decides.
    Auto,
}

/// What a running session's mode becomes once new settings reach it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RunningMode {
    /// The mode.
    pub mode: PermissionMode,
    /// Set when the settings moved the session out of its mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<&'static str>,
}

const fn leaves(mode: PermissionMode) -> bool {
    match mode {
        PermissionMode::Default
        | PermissionMode::AcceptEdits
        | PermissionMode::BypassPermissions
        | PermissionMode::Plan
        | PermissionMode::DontAsk => false,
        PermissionMode::Auto => true,
    }
}

/// Whether a managed source sets `disableAutoMode` (or `permissions.disableAutoMode`) to `"disable"`.
async fn admin_disables_auto_mode(view: &dyn PolicyView) -> bool {
    let disable = JsValue::String("disable".to_owned());
    let top = view
        .each_from_highest("disableAutoMode")
        .await
        .iter()
        .any(|holder| holder.scope == PolicyScope::Managed && holder.value == disable);
    let nested = view
        .each_from_highest("permissions")
        .await
        .iter()
        .any(|holder| {
            holder.scope == PolicyScope::Managed
                && holder.value.get("disableAutoMode") == Some(&disable)
        });
    top || nested
}

/// The mode of a session running in `current` once the settings of `view` reach it.
pub async fn running_mode_of(view: &dyn PolicyView, current: PermissionMode) -> RunningMode {
    if leaves(current) && admin_disables_auto_mode(view).await {
        RunningMode {
            mode: PermissionMode::Default,
            notice: Some(AUTO_MODE_DISABLED_NOTICE),
        }
    } else {
        RunningMode {
            mode: current,
            notice: None,
        }
    }
}

crate::registry::exports! {
    "settings/permissions/running-mode" {
        "AUTO_MODE_DISABLED_NOTICE" => ok || AUTO_MODE_DISABLED_NOTICE;
        "runningModeOf" => async ok |view: Port, current: PermissionMode|
            running_mode_of(&view, current).await;
    }
}
