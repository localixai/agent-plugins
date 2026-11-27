//! Port of `src/marketplace/session-origins.ts`, switched.
//!
//! The plugins that are not installed: the origins `inline` and `skills-dir` (plugins/loading.md,
//! "Find where a plugin came from"), how a session-only plugin is listed and which copy of a name
//! wins (plugins/cli-reference.md, "Flags that load a plugin for one session"), and how to stop
//! loading a skills-directory plugin (plugins/create.md, "Stop loading the plugin"). Pure.

use serde::{Deserialize, Serialize};

/// `<name>@inline`: "a plugin that `--plugin-dir`, `--plugin-url`, `CLAUDE_CODE_PLUGIN_DIRS` or the Agent SDK `plugins` option loaded".
#[must_use]
pub fn inline_plugin_id(name: &str) -> String {
    format!("{name}@inline")
}

/// `<name>@skills-dir`: "`skills-dir` stands where a marketplace name would".
#[must_use]
pub fn skills_dir_plugin_id(name: &str) -> String {
    format!("{name}@skills-dir")
}

/// How `claude plugin list` shows a session-only plugin: "only when the same flag precedes the
/// subcommand, as in `claude --plugin-dir ./my-plugin plugin list`", as `<name>@inline` under a
/// heading that begins `Session-only plugins`, and `--json` reports its `scope` as `session`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionOnlyRow {
    /// `<name>@inline`.
    pub id: String,
    /// The heading it is listed under.
    pub heading: String,
    /// `session`.
    pub scope: &'static str,
}

/// How `claude plugin list` shows a session-only plugin.
#[must_use]
pub fn session_only_row(name: &str, flag_precedes_subcommand: bool) -> Option<SessionOnlyRow> {
    flag_precedes_subcommand.then(|| SessionOnlyRow {
        id: inline_plugin_id(name),
        heading: "Session-only plugins (--plugin-dir / --plugin-url):".to_owned(),
        scope: "session",
    })
}

/// Two copies of one name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameClash {
    /// The session-only copy was disabled with `claude plugin disable <name>@inline`.
    pub session_only_disabled: bool,
    /// Managed settings lock that plugin name.
    pub managed_locks_name: bool,
}

/// Which copy loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CopyThatLoads {
    /// `session-only`.
    #[serde(rename = "session-only")]
    SessionOnly,
    /// `installed`.
    #[serde(rename = "installed")]
    Installed,
}

/// "When a session-only plugin shares a name with an installed plugin, Claude Code loads the
/// session-only copy for that session and skips the installed one. The installed copy loads instead
/// if you disabled the session-only copy with `claude plugin disable <name>@inline`, or if managed
/// settings lock that plugin name."
#[must_use]
pub const fn copy_that_loads(clash: &NameClash) -> CopyThatLoads {
    if clash.session_only_disabled || clash.managed_locks_name {
        CopyThatLoads::Installed
    } else {
        CopyThatLoads::SessionOnly
    }
}

/// The two ways to stop loading a scaffolded plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopLoading {
    /// Delete its directory.
    pub delete_directory: bool,
    /// Or run this in a shell.
    pub disable_command: String,
}

/// The two ways to stop loading a scaffolded plugin: "delete its directory, or run `claude plugin
/// disable my-tool@skills-dir` in your shell".
#[must_use]
pub fn stop_loading_skills_dir_plugin(name: &str) -> StopLoading {
    StopLoading {
        delete_directory: true,
        disable_command: format!("claude plugin disable {}", skills_dir_plugin_id(name)),
    }
}

crate::registry::exports! {
    "marketplace/session-origins" {
        "inlinePluginId" => ok |name: String| inline_plugin_id(&name);
        "skillsDirPluginId" => ok |name: String| skills_dir_plugin_id(&name);
        "sessionOnlyRow" => ok |name: String, flag_precedes_subcommand: bool| session_only_row(&name, flag_precedes_subcommand);
        "copyThatLoads" => ok |clash: NameClash| copy_that_loads(&clash);
        "stopLoadingSkillsDirPlugin" => ok |name: String| stop_loading_skills_dir_plugin(&name);
    }
}
