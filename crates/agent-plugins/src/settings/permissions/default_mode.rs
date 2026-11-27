//! Port of `src/settings/permissions/default-mode.ts`, switched.
//!
//! Which permission mode a session starts in (permission-modes.md, "Which mode a session starts
//! in", "Start in a different permission mode", "Skip all checks with bypassPermissions mode",
//! "Bypass permissions ignored in a background session"; settings-reference.md,
//! `permissions.defaultMode`, `disableAutoMode`, `permissions.disableBypassPermissionsMode`). Pure
//! over the settings and what the host knows of the launch.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::is_truthy;
use crate::protocol::permission_modes::normalize_permission_mode;
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::permissions::start_notices::{NoticeFacts, start_notices};
use crate::settings::view::PolicyView;

/// Where the session runs: the terminal, the VS Code extension, or `claude -p` and the Agent SDK.
pub const LAUNCH_SURFACES: [&str; 3] = ["terminal", "vscode", "headless"];

/// One of `LAUNCH_SURFACES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchSurface {
    /// The terminal.
    Terminal,
    /// The VS Code extension.
    Vscode,
    /// `claude -p` and the Agent SDK.
    Headless,
}

/// What the host knows of the launch and the session (`StartInput`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartInput {
    surface: LaunchSurface,
    permission_mode: Option<String>,
    dangerously_skip_permissions: Option<bool>,
    allow_dangerously_skip_permissions: Option<bool>,
    auto_available: bool,
    hipaa: Option<bool>,
    background: Option<bool>,
    restricted: Option<bool>,
    privileged: Option<bool>,
}

/// The settings keys the start reads, from the view (`StartSettings`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSettings {
    disable_auto_mode: bool,
    disable_bypass_permissions_mode: bool,
    skip_dangerous_mode_permission_prompt: bool,
}

/// The mode a session starts in, or why it is refused.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StartingMode {
    /// The session starts.
    Started {
        /// The mode.
        mode: &'static str,
        /// `flag`, `settings` or `built-in`: why the session starts in it.
        source: &'static str,
        /// Bypass permissions was enabled at launch, so plan mode does not enforce its blocks at a
        /// terminal.
        bypass_available: bool,
        /// Auto mode is in the session's mode cycle.
        auto_available: bool,
        /// An interactive session entering `bypassPermissions` shows the warning dialog first.
        bypass_warning: bool,
        /// Notices the session shows at its start, verbatim (`startNotices`).
        notices: Vec<&'static str>,
        /// What the session shows and the settings said, as warnings.
        issues: Vec<PluginIssue>,
    },
    /// The launch is refused.
    Refused {
        /// Why.
        reason: String,
    },
}

const BYPASS: &str = "bypassPermissions";
const AUTO: &str = "auto";
const MANUAL: &str = "default";

/// The notice a background session shows for an ignored bypass request (permission-modes.md,
/// verbatim).
pub const BYPASS_IGNORED_NOTICE: &str = "Bypass permissions was requested at launch and ignored · if that was you, ~/.claude/settings.json needs \"skipDangerousModePermissionPrompt\": true";
const BYPASS_LOCKED: &str = "bypassPermissions is rejected while \"permissions.disableBypassPermissionsMode\" is \"disable\"";
const BYPASS_RESTRICTED: &str =
    "bypassPermissions is refused in a session started with --restricted";
/// permission-modes.md, verbatim.
pub const BYPASS_AS_ROOT: &str =
    "--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons";
const BYPASS_NOT_ACCEPTED: &str =
    "Bypass permissions needs the warning accepted first: accept it in an interactive session";

/// The settings files each surface reads `permissions.defaultMode` from (settings-reference.md):
/// "For conversations the VS Code extension starts, Claude Code reads only user, managed, and
/// `--settings` values."
const fn reads(surface: LaunchSurface, scope: PolicyScope) -> bool {
    match surface {
        LaunchSurface::Terminal | LaunchSurface::Headless => true,
        LaunchSurface::Vscode => {
            matches!(
                scope,
                PolicyScope::User | PolicyScope::Managed | PolicyScope::Flag
            )
        }
    }
}

/// The built-in default: `default` under `disableAutoMode`, in `-p` and the SDK, and under the
/// HIPAA configuration.
fn built_in(input: &StartInput, settings: StartSettings) -> &'static str {
    if settings.disable_auto_mode || input.hipaa == Some(true) {
        return MANUAL;
    }
    match input.surface {
        LaunchSurface::Terminal | LaunchSurface::Vscode => AUTO,
        LaunchSurface::Headless => MANUAL,
    }
}

/// What picked the mode: `None` is the built-in default.
struct Chosen {
    mode: Option<&'static str>,
    source: &'static str,
    issues: Vec<PluginIssue>,
}

const NOTHING: Chosen = Chosen {
    mode: None,
    source: "built-in",
    issues: Vec::new(),
};

fn settings_issue(scope: PolicyScope, mode: &str, outcome: &str) -> PluginIssue {
    let scope = scope.name();
    warning(
        &format!("{scope}:permissions.defaultMode"),
        &format!("\"{mode}\" does not take effect from {scope} settings; {outcome}"),
    )
}

/// The mode `permissions.defaultMode` selects: the highest file the surface reads that sets it. An
/// `auto` there from a project or local file is no value and "Claude Code then uses the built-in
/// default rather than a `defaultMode` from `~/.claude/settings.json`"; `bypassPermissions` from
/// those files "doesn't take effect either, and the session starts in Manual mode".
async fn from_settings(view: &dyn PolicyView, surface: LaunchSurface) -> Chosen {
    let found = view
        .each_from_highest("permissions")
        .await
        .into_iter()
        .find_map(|each| {
            let mode = each
                .value
                .get("defaultMode")?
                .as_str()
                .and_then(normalize_permission_mode)?;
            reads(surface, each.scope).then_some((each.scope, mode))
        });
    let Some((scope, mode)) = found else {
        return NOTHING;
    };
    let repository = matches!(scope, PolicyScope::Project | PolicyScope::Local);
    match mode {
        AUTO if repository => Chosen {
            issues: vec![settings_issue(scope, mode, "the built-in default applies")],
            ..NOTHING
        },
        BYPASS if repository => Chosen {
            mode: Some(MANUAL),
            source: "settings",
            issues: vec![settings_issue(
                scope,
                mode,
                "the session starts in Manual mode",
            )],
        },
        _ => Chosen {
            mode: Some(mode),
            source: "settings",
            issues: Vec::new(),
        },
    }
}

/// `--dangerously-skip-permissions` is `--permission-mode bypassPermissions`; an unknown value is
/// refused.
fn from_flag(input: &StartInput) -> Result<Option<Chosen>, String> {
    let flag = |mode| Chosen {
        mode: Some(mode),
        source: "flag",
        issues: Vec::new(),
    };
    if input.dangerously_skip_permissions == Some(true) {
        return Ok(Some(flag(BYPASS)));
    }
    let Some(typed) = &input.permission_mode else {
        return Ok(None);
    };
    normalize_permission_mode(typed)
        .map(|mode| Some(flag(mode)))
        .ok_or_else(|| format!("\"{typed}\" is not a permission mode"))
}

/// What a background session does with a bypass request.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Gate {
    Honor,
    Ignore,
    Refuse,
}

/// How the acceptance of the bypass warning is recorded (permission-modes.md, "Skip all checks
/// with bypassPermissions mode"): in user or managed settings a background session honors it
/// (`recorded`), elsewhere it ignores the request (`elsewhere`), with none it refuses (`none`).
async fn bypass_acceptance(view: &dyn PolicyView) -> Gate {
    let holders = view
        .each_from_highest("skipDangerousModePermissionPrompt")
        .await;
    let accepted: Vec<_> = holders
        .iter()
        .filter(|each| is_truthy(&each.value))
        .collect();
    if accepted
        .iter()
        .any(|each| matches!(each.scope, PolicyScope::User | PolicyScope::Managed))
    {
        Gate::Honor
    } else if accepted.is_empty() {
        Gate::Refuse
    } else {
        Gate::Ignore
    }
}

/// What a background session does with the bypass request of a launch, by what was asked for: a
/// mode that is refused with no acceptance recorded, `--allow-dangerously-skip-permissions`, which
/// is only ignored.
async fn background_gate(view: &dyn PolicyView, input: &StartInput, wanted: &str) -> Gate {
    let requested = wanted == BYPASS || input.allow_dangerously_skip_permissions == Some(true);
    if input.background != Some(true) || !requested {
        return Gate::Honor;
    }
    let gate = bypass_acceptance(view).await;
    if gate == Gate::Refuse && wanted != BYPASS {
        Gate::Ignore
    } else {
        gate
    }
}

/// The mode the launch ends in: a bypass request a background session ignores is Manual; then the
/// settings' locks and the availability of auto mode.
fn mode_of(
    wanted: &'static str,
    ignored: bool,
    input: &StartInput,
    settings: StartSettings,
) -> &'static str {
    let locked_auto = wanted == AUTO && (settings.disable_auto_mode || !input.auto_available);
    let locked_bypass = wanted == BYPASS && (ignored || settings.disable_bypass_permissions_mode);
    if locked_auto || locked_bypass {
        MANUAL
    } else {
        wanted
    }
}

/// Applies the background gate and the locks to the mode the flag, the settings or the built-in
/// default picked; the mode cycle and the dialog of the session; `--restricted` refuses bypass.
async fn launch(
    view: &dyn PolicyView,
    chosen: Chosen,
    wanted: &'static str,
    input: &StartInput,
    settings: &StartSettings,
) -> StartingMode {
    let gate = background_gate(view, input, wanted).await;
    if gate == Gate::Refuse {
        return refuse(BYPASS_NOT_ACCEPTED);
    }
    let ignored = gate == Gate::Ignore;
    let mode = mode_of(wanted, ignored, input, *settings);
    let asked =
        input.allow_dangerously_skip_permissions == Some(true) && input.restricted != Some(true);
    let no_bypass = settings.disable_bypass_permissions_mode;
    let mut issues = chosen.issues;
    if ignored {
        issues.push(warning("permissions", BYPASS_IGNORED_NOTICE));
    }
    StartingMode::Started {
        mode,
        source: if chosen.mode.is_none() {
            "built-in"
        } else {
            chosen.source
        },
        bypass_available: !no_bypass && !ignored && (mode == BYPASS || asked),
        auto_available: input.auto_available && !settings.disable_auto_mode,
        bypass_warning: mode == BYPASS
            && input.surface != LaunchSurface::Headless
            && !settings.skip_dangerous_mode_permission_prompt,
        notices: start_notices(&NoticeFacts {
            hipaa: input.hipaa == Some(true),
            terminal: input.surface == LaunchSurface::Terminal,
            built_in: chosen.mode.is_none(),
        }),
        issues,
    }
}

fn refuse(reason: &str) -> StartingMode {
    StartingMode::Refused {
        reason: reason.to_owned(),
    }
}

/// Why a start in `bypassPermissions` is refused whatever picked it, or `None`.
fn bypass_refusal(input: &StartInput, wanted: &str) -> Option<&'static str> {
    if wanted != BYPASS {
        return None;
    }
    if input.restricted == Some(true) {
        return Some(BYPASS_RESTRICTED);
    }
    (input.privileged == Some(true)).then_some(BYPASS_AS_ROOT)
}

/// The mode a session starts in. The first of these applies: the flag, `permissions.defaultMode`,
/// the built-in default; `disableAutoMode` turns an `auto` start (from any of the three) into
/// `default`, and `permissions.disableBypassPermissionsMode` rejects the flag.
pub async fn starting_mode(
    view: &dyn PolicyView,
    settings: &StartSettings,
    input: &StartInput,
) -> StartingMode {
    let flag = match from_flag(input) {
        Ok(flag) => flag,
        Err(reason) => return StartingMode::Refused { reason },
    };
    let no_bypass = settings.disable_bypass_permissions_mode;
    if no_bypass && flag.as_ref().is_some_and(|flag| flag.mode == Some(BYPASS)) {
        return refuse(BYPASS_LOCKED);
    }
    let chosen = match flag {
        Some(flag) => flag,
        None => from_settings(view, input.surface).await,
    };
    let wanted = chosen.mode.unwrap_or_else(|| built_in(input, *settings));
    match bypass_refusal(input, wanted).filter(|_| !no_bypass) {
        Some(reason) => refuse(reason),
        None => launch(view, chosen, wanted, input, settings).await,
    }
}

crate::registry::exports! {
    "settings/permissions/default-mode" {
        "LAUNCH_SURFACES" => ok || LAUNCH_SURFACES;
        "BYPASS_IGNORED_NOTICE" => ok || BYPASS_IGNORED_NOTICE;
        "BYPASS_AS_ROOT" => ok || BYPASS_AS_ROOT;
        "startingMode" => async ok |view: Port, settings: StartSettings, input: StartInput|
            starting_mode(&view, &settings, &input).await;
    }
}
