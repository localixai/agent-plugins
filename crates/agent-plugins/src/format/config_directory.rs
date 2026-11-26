//! Port of `src/format/config-directory.ts`, switched.
//!
//! The user's configuration directory (env-vars.md, `CLAUDE_CONFIG_DIR`: "Override the configuration
//! directory (default: `~/.claude`). All settings, session history, and plugins are stored under this
//! path"; claude-directory.md: "every `~/.claude` path on this page lives under that directory instead.
//! The same applies to `.claude.json`"). The one owner of that rule: every user-scope path the kit
//! builds (skills, commands, agents, output styles, settings, the plugin store, saved workflows, agent
//! memory, the `headersHelper` directory, `.claude.json`) starts here. An empty value counts as unset
//! (judgment, spec/judgments.md). Probed against Claude Code 2.1.322 as a black box: with the variable
//! set, `~/.claude/{skills,agents,commands,settings.json}` are not read and the directory's own are.

use serde::{Deserialize, Serialize};

/// The variable, as the host's `Environment` port reads it.
pub const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// `base` and a `/`-separated relative path joined the way the host writes paths (`Locations.joinPath`).
pub type Join = dyn Fn(&str, &str) -> String;

/// The configuration directory as a base directory and the folder below it: `.claude` below the home,
/// or the `CLAUDE_CONFIG_DIR` directory itself with an empty folder. A scan opens the base through the
/// host's port and reads below the folder, so no path is joined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFolder {
    /// The directory a scan opens.
    pub base: String,
    /// `.claude`, or `''` when the base is the configuration directory itself.
    pub folder: String,
}

/// The `CLAUDE_CONFIG_DIR` value when it is set and not empty.
fn set_value(config_dir: Option<&str>) -> Option<&str> {
    config_dir.filter(|value| !value.is_empty())
}

/// The configuration directory as a base and a folder (see `ConfigFolder`).
#[must_use]
pub fn config_folder_of(home: &str, config_dir: Option<&str>) -> ConfigFolder {
    match set_value(config_dir) {
        Some(directory) => ConfigFolder {
            base: directory.to_owned(),
            folder: String::new(),
        },
        None => ConfigFolder {
            base: home.to_owned(),
            folder: ".claude".to_owned(),
        },
    }
}

/// The configuration directory as one path: `CLAUDE_CONFIG_DIR`, else `<home>/.claude`.
#[must_use]
pub fn config_directory_of(home: &str, config_dir: Option<&str>, join: &Join) -> String {
    match set_value(config_dir) {
        Some(directory) => directory.to_owned(),
        None => join(home, ".claude"),
    }
}

/// The global config file: `.claude.json` in `CLAUDE_CONFIG_DIR` when set, else in the home.
#[must_use]
pub fn global_config_file_of(home: &str, config_dir: Option<&str>, join: &Join) -> String {
    join(set_value(config_dir).unwrap_or(home), ".claude.json")
}

crate::registry::exports! {
    "format/config-directory" {
        "CONFIG_DIR_VARIABLE" => ok || CONFIG_DIR_VARIABLE;
        "configFolderOf" => ok |home: String, config_dir: Option<String>| config_folder_of(&home, config_dir.as_deref());
    }
}
