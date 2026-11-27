//! Port of `src/settings/env/ignored-variables.ts`, switched.
//!
//! The `env` variables Claude Code drops (settings-reference.md, "Variables Claude Code ignores in
//! `env`"), as data. Two lists: the variables "a checked-out repository shouldn't control",
//! dropped from project and local settings, and the variables Claude Code reads from the launch
//! environment only or exports itself, dropped from every file. The page's two open-ended lists
//! ("such as" the operating-system directories, the hosting-environment identity variables) hold
//! the names the page spells out and nothing the kit would have to guess (spec/judgments.md).

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{JsValue, Pattern, regex_test, string_of};
use crate::settings::key_spec::REPOSITORY_FILES;
use crate::settings::layers::PolicyScope;

/// Why a variable is dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    missing_docs,
    reason = "each variant is the group of its name in GROUPS"
)]
pub enum IgnoreReason {
    Storage,
    Windows,
    SessionContent,
    Telemetry,
    Startup,
    SignIn,
    Accessibility,
    DialogTimer,
    Attachments,
    FileMentions,
    Identity,
    LaunchOnly,
}

/// Which files lose the variable: a repository's (project and local) or every one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum From {
    Repository,
    EveryFile,
}

struct Group {
    reason: IgnoreReason,
    from: From,
    names: &'static [&'static str],
    /// A family named by its start (`XDG_*`, `CLAUDE_AX_*`, the `ProgramFiles` family).
    prefixes: &'static [&'static str],
    /// A family named by its end (`OTEL_EXPORTER_OTLP_*_ENDPOINT`).
    pattern: Option<&'static Pattern>,
    /// Windows names are case-insensitive, so these compare without regard to case (spec/judgments.md).
    case_insensitive: bool,
}

/// `OTEL_EXPORTER_OTLP_*` names ending in one of the six suffixes, generic or per signal.
static OTLP: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^OTEL_EXPORTER_OTLP_(?:[^\n\r\u{2028}\u{2029}]+_)?(?:ENDPOINT|HEADERS|PROTOCOL|CERTIFICATE|CLIENT_KEY|INSECURE)$",
    )
    .ok()
});

const fn group(reason: IgnoreReason, from: From, names: &'static [&'static str]) -> Group {
    Group {
        reason,
        from,
        names,
        prefixes: &[],
        pattern: None,
        case_insensitive: false,
    }
}

static GROUPS: [Group; 12] = [
    Group {
        prefixes: &["XDG_"],
        ..group(
            IgnoreReason::Storage,
            From::Repository,
            &[
                "CLAUDE_CONFIG_DIR",
                "CLAUDE_CODE_TMPDIR",
                "HOME",
                "TMPDIR",
                "TMP",
                "TEMP",
            ],
        )
    },
    // "the `ProgramFiles` family": `ProgramFiles`, `ProgramFiles(x86)` and `ProgramW6432` (spec/judgments.md).
    Group {
        prefixes: &["ProgramFiles"],
        case_insensitive: true,
        ..group(
            IgnoreReason::Windows,
            From::Repository,
            &[
                "SystemRoot",
                "ComSpec",
                "ProgramData",
                "LOCALAPPDATA",
                "PATHEXT",
                "PSModulePath",
                "ProgramW6432",
            ],
        )
    },
    group(
        IgnoreReason::SessionContent,
        From::Repository,
        &[
            "OTEL_LOG_RAW_API_BODIES",
            "ENABLE_BETA_TRACING_DETAILED",
            "BETA_TRACING_ENDPOINT",
        ],
    ),
    Group {
        pattern: Some(&OTLP),
        ..group(
            IgnoreReason::Telemetry,
            From::Repository,
            &[
                "CLAUDE_CODE_ENABLE_TELEMETRY",
                "CLAUDE_CODE_ENHANCED_TELEMETRY_BETA",
                "ENABLE_ENHANCED_TELEMETRY_BETA",
                "OTEL_LOGS_EXPORTER",
                "OTEL_METRICS_EXPORTER",
                "OTEL_TRACES_EXPORTER",
                "OTEL_LOG_USER_PROMPTS",
                "OTEL_LOG_ASSISTANT_RESPONSES",
                "OTEL_LOG_TOOL_CONTENT",
                "OTEL_LOG_TOOL_DETAILS",
                "OTEL_EXPORTER_PROMETHEUS_HOST",
                "OTEL_EXPORTER_PROMETHEUS_PORT",
            ],
        )
    },
    group(
        IgnoreReason::Startup,
        From::Repository,
        &[
            "CLAUDE_CODE_PROCESS_WRAPPER",
            "CLAUDE_CODE_SYNC_SKILLS",
            "CLAUDE_CODE_SYNC_PLUGINS",
            "CLAUDE_CODE_PLUGIN_CACHE_DIR",
            "CLAUDE_CODE_PLUGIN_SEED_DIR",
        ],
    ),
    group(
        IgnoreReason::SignIn,
        From::Repository,
        &[
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
            "CLAUDE_CODE_OAUTH_SCOPES",
        ],
    ),
    Group {
        prefixes: &["CLAUDE_AX_"],
        ..group(
            IgnoreReason::Accessibility,
            From::Repository,
            &["CLAUDE_AX_SCREEN_READER"],
        )
    },
    group(
        IgnoreReason::DialogTimer,
        From::Repository,
        &["CLAUDE_CODE_USER_DIALOG_TIMEOUT_MS"],
    ),
    group(
        IgnoreReason::Attachments,
        From::Repository,
        &["CLAUDE_CODE_DISABLE_ATTACHMENTS"],
    ),
    group(
        IgnoreReason::FileMentions,
        From::Repository,
        &["CLAUDE_CODE_DISABLE_FILE_MENTIONS"],
    ),
    group(
        IgnoreReason::Identity,
        From::EveryFile,
        &["CLAUDE_CODE_REMOTE", "CLAUDE_CODE_ACCOUNT_UUID"],
    ),
    group(
        IgnoreReason::LaunchOnly,
        From::EveryFile,
        &[
            "CLAUDE_CODE_MESSAGING_SOCKET",
            "CLAUDE_CODE_MESSAGING_TOKEN",
            "CLAUDE_CODE_PROJECT_DIR_NAME",
            "CLAUDE_CODE_RESTRICTED",
            "CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY",
            "CLAUDE_CODE_DISABLE_DANGEROUS_RM_TIMEOUT",
            "CLAUDE_CODE_DISABLE_SUBSTITUTION_RM_PROMPT",
            "CLAUDE_CODE_DISABLE_INLINE_SHELL_RM_PROMPT",
            "CLAUDE_CODE_DISABLE_MOUNTED_DRIVE_RM_PROMPT",
            "CLAUDE_CODE_EVEN_MORE_DANGEROUSLY_SKIP_PERMISSIONS",
            "CLAUDE_CODE_WARN_DISK_FREE_GIB",
            "CLAUDE_CODE_WARN_MEMORY_FREE_GIB",
        ],
    ),
];

/// Whether the group names `name`; a Windows host's names, and the Windows group's anywhere, compare without case.
fn named(group: &Group, name: &str, windows: bool) -> bool {
    let fold = |text: &str| -> String {
        if windows || group.case_insensitive {
            text.to_uppercase()
        } else {
            text.to_owned()
        }
    };
    let candidate = fold(name);
    group.names.iter().any(|one| fold(one) == candidate)
        || group
            .prefixes
            .iter()
            .any(|prefix| candidate.starts_with(&fold(prefix)))
        || group
            .pattern
            .is_some_and(|pattern| regex_test(pattern, if windows { &candidate } else { name }))
}

/// The variables whose one off value still applies from a repository, "because they turn
/// something off": `none` for the three exporter selectors, "an off value such as `0`" for the
/// three content variables. Which spellings are off values beyond `0` is the kit's judgment
/// (spec/judgments.md).
const OFF_VALUES: &[&str] = &["0", "false"];
const OFF_VALUE_OF: [(&str, &[&str]); 6] = [
    ("OTEL_LOGS_EXPORTER", &["none"]),
    ("OTEL_METRICS_EXPORTER", &["none"]),
    ("OTEL_TRACES_EXPORTER", &["none"]),
    ("OTEL_LOG_USER_PROMPTS", OFF_VALUES),
    ("OTEL_LOG_TOOL_CONTENT", OFF_VALUES),
    ("OTEL_LOG_TOOL_DETAILS", OFF_VALUES),
];

/// The text of an `env` value as the process gets it: claude sets JavaScript's `String(value)`
/// for a value that is not a string (oracle 2.1.321, spec/judgments.md
/// `settings::env::check_env`).
#[must_use]
pub fn env_text(value: &JsValue) -> String {
    string_of(value)
}

/// Whether `value` is one of the off values that still apply from a project or local file for
/// `name`: the value in any case, as written otherwise; on a Windows host the name in any case too.
#[must_use]
pub fn turns_telemetry_off(name: &str, value: &JsValue, windows: bool) -> bool {
    let key = if windows {
        name.to_uppercase()
    } else {
        name.to_owned()
    };
    let off = OFF_VALUE_OF
        .iter()
        .find(|(one, _)| *one == key)
        .map(|(_, off)| *off);
    match (off, value.as_str()) {
        (Some(off), Some(text)) => off.contains(&text.to_lowercase().as_str()),
        _ => false,
    }
}

/// Why Claude Code drops the variable `name` set to `value` in a file of `scope`, or `None` when
/// the variable stays. `windows`: the host's variable names are case-insensitive.
#[must_use]
pub fn ignored_because(
    scope: PolicyScope,
    name: &str,
    value: &JsValue,
    windows: bool,
) -> Option<IgnoreReason> {
    let from_repository = REPOSITORY_FILES.contains(&scope);
    let group = GROUPS.iter().find(|one| {
        (from_repository || one.from == From::EveryFile) && named(one, name, windows)
    })?;
    if from_repository && turns_telemetry_off(name, value, windows) {
        None
    } else {
        Some(group.reason)
    }
}

crate::registry::exports! {
    "settings/env/ignored-variables" {
        "ignoredBecause" => ok |scope: PolicyScope, name: String, value: JsValue, windows: Option<bool>|
            ignored_because(scope, &name, &value, windows.unwrap_or(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_otlp_pattern_compiles_and_names_the_per_signal_endpoints() {
        assert!(OTLP.is_some());
        let value = JsValue::from("x");
        let why = ignored_because(
            PolicyScope::Project,
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            &value,
            false,
        );
        assert_eq!(why, Some(IgnoreReason::Telemetry));
        assert_eq!(
            ignored_because(PolicyScope::User, "HOME", &value, false),
            None
        );
    }
}
