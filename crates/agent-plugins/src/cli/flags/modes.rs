//! Port of `src/cli/flags/modes.ts`, switched.
//!
//! The modes that leave customizations out (cli-reference.md, `--bare`, `--safe-mode`,
//! `--restricted`, `--disable-slash-commands`): which kinds each skips (`customizations.rs`), the
//! variable it sets, and for `--restricted` the rest of its list. What loads under the settings,
//! tool and permission rules is those modules' (`availableTools` reads `restricted`).

use indexmap::IndexMap;
use serde::Serialize;

use super::customizations::{ManagedLayer, SkipMode, managed_under_safe_mode, skipped_by};
use super::types::{CloudSessions, Customization};

// Each mode's `Contribution` (`types.rs`) as its own record, so its members keep the order the
// TypeScript writes them in.

/// What `--bare` contributes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bare {
    /// What it skips.
    pub skipped: Vec<Customization>,
    /// The variable it sets.
    pub env: IndexMap<String, String>,
    /// Never `true`: bare mode reads no stored login credentials.
    pub reads_stored_credentials: bool,
}

/// What `--safe-mode` contributes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SafeMode {
    /// What it skips.
    pub skipped: Vec<Customization>,
    /// The variable it sets.
    pub env: IndexMap<String, String>,
}

/// What `--disable-slash-commands` contributes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DisableSlashCommands {
    /// What it skips.
    pub skipped: Vec<Customization>,
}

/// What `--restricted` contributes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restricted {
    /// The session is restricted (`availableTools` reads it).
    pub restricted: bool,
    /// The built-in file tools are confined to the working directories.
    pub file_tools_confined: bool,
    /// Cloud sessions are refused.
    pub cloud_sessions: CloudSessions,
    /// The settings scopes that load (`PolicyScope`).
    pub setting_scopes: Vec<String>,
}

fn one_variable(name: &str) -> IndexMap<String, String> {
    IndexMap::from_iter([(name.to_owned(), "1".to_owned())])
}

/// "Sets `CLAUDE_CODE_SIMPLE`", which "Set to `1`" does (env-vars.md); bare mode "never reads OAuth
/// credentials or the system keychain": authentication comes from `ANTHROPIC_API_KEY` or an
/// `apiKeyHelper` in `--settings` (headless.md, "Start faster with bare mode").
#[must_use]
pub fn bare() -> Bare {
    Bare {
        skipped: skipped_by(SkipMode::Bare),
        env: one_variable("CLAUDE_CODE_SIMPLE"),
        reads_stored_credentials: false,
    }
}

/// "Sets `CLAUDE_CODE_SAFE_MODE`", which "Set to `1`" does (env-vars.md).
#[must_use]
pub fn safe_mode() -> SafeMode {
    SafeMode {
        skipped: skipped_by(SkipMode::SafeMode),
        env: one_variable("CLAUDE_CODE_SAFE_MODE"),
    }
}

/// `--disable-slash-commands`: what it skips.
#[must_use]
pub fn disable_slash_commands() -> DisableSlashCommands {
    DisableSlashCommands {
        skipped: skipped_by(SkipMode::DisableSlashCommands),
    }
}

/// `--restricted`: "loads only managed settings and `--settings`", "confines the built-in file
/// tools to the working directories", removes the tools that run commands or code and WebFetch (the
/// rules module's `restricted`), "refuses `bypassPermissions`" (the `--permission-mode` strategy)
/// and "refuses to create cloud sessions".
#[must_use]
pub fn restricted() -> Restricted {
    Restricted {
        restricted: true,
        file_tools_confined: true,
        cloud_sessions: CloudSessions::Refused,
        setting_scopes: ["managed", "parent", "flag"].map(str::to_owned).to_vec(),
    }
}

/// Whether a safe-mode session still applies the managed policy of a kind it skips; `None` where
/// the docs say nothing.
#[must_use]
pub fn managed_policy_under_safe_mode(kind: &str) -> Option<bool> {
    managed_under_safe_mode()
        .get(kind)
        .map(|treatment| *treatment == ManagedLayer::Applies)
}

crate::registry::exports! {
    "cli/flags/modes" {
        "bare" => ok bare;
        "safeMode" => ok safe_mode;
        "disableSlashCommands" => ok disable_slash_commands;
        "restricted" => ok restricted;
        "managedPolicyUnderSafeMode" => ok |kind: String| managed_policy_under_safe_mode(&kind);
    }
}
