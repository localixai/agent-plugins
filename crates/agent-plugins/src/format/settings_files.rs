//! Port of `src/format/settings-files.ts`, switched.
//!
//! Where the three settings files a person edits live, as settings.md writes them ("Settings files"):
//! the user file under the configuration folder, the project and local files under `.claude/` of the
//! project. One spelling for every message, label and path the kit builds from them.

use serde::Serialize;

/// The three settings files (`SETTINGS_FILE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsFile {
    /// The user file's name inside the configuration folder (`~/.claude` or `CLAUDE_CONFIG_DIR`).
    pub user_name: &'static str,
    /// The user file as the docs write it.
    pub user: &'static str,
    /// The project file, relative to the project folder.
    pub project: &'static str,
    /// The local file, relative to the folder it stays in.
    pub local: &'static str,
}

/// Where the settings files live.
pub const SETTINGS_FILE: SettingsFile = SettingsFile {
    user_name: "settings.json",
    user: "~/.claude/settings.json",
    project: ".claude/settings.json",
    local: ".claude/settings.local.json",
};

crate::registry::exports! {
    "format/settings-files" {
        "SETTINGS_FILE" => ok || SETTINGS_FILE;
    }
}
