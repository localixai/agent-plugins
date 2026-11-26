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

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// The variable, as the host's `Environment` port reads it.
pub const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// `base` and a `/`-separated relative path joined the way the host writes paths (`Locations.joinPath`).
/// A function of the host's, so its answer may wait: a Rust host passes a closure, answered at once.
pub trait Join: Send + Sync {
    /// The joined path.
    fn join<'a>(&'a self, base: &'a str, relative_path: &'a str) -> BoxFuture<'a, String>;
}

impl<F: Fn(&str, &str) -> String + Send + Sync> Join for F {
    fn join<'a>(&'a self, base: &'a str, relative_path: &'a str) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(self(base, relative_path)))
    }
}

impl Join for Callback {
    fn join<'a>(&'a self, base: &'a str, relative_path: &'a str) -> BoxFuture<'a, String> {
        Box::pin(self.call(vec![JsValue::from(base), JsValue::from(relative_path)]))
    }
}

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
pub async fn config_directory_of(home: &str, config_dir: Option<&str>, join: &dyn Join) -> String {
    let ConfigFolder { base, folder } = config_folder_of(home, config_dir);
    if folder.is_empty() {
        base
    } else {
        join.join(&base, &folder).await
    }
}

/// The global config file: `.claude.json` in `CLAUDE_CONFIG_DIR` when set, else in the home.
pub async fn global_config_file_of(
    home: &str,
    config_dir: Option<&str>,
    join: &dyn Join,
) -> String {
    join.join(&config_folder_of(home, config_dir).base, ".claude.json")
        .await
}

crate::registry::exports! {
    "format/config-directory" {
        "CONFIG_DIR_VARIABLE" => ok || CONFIG_DIR_VARIABLE;
        "configFolderOf" => ok |home: String, config_dir: Option<String>| config_folder_of(&home, config_dir.as_deref());
        "configDirectoryOf" => async ok |home: String, config_dir: Option<String>, join: Callback|
            config_directory_of(&home, config_dir.as_deref(), &join).await;
        "globalConfigFileOf" => async ok |home: String, config_dir: Option<String>, join: Callback|
            global_config_file_of(&home, config_dir.as_deref(), &join).await;
    }
}
