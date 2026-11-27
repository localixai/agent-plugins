//! Port of `src/marketplace/remote-plugins.ts`, switched.
//!
//! Which of the user's plugins a session on another machine gets (desktop.md, "Install plugins",
//! "Windows remote machines", "Files and processes on the remote machine"): a session over SSH gets
//! the plugins copied from the user's computer to `~/.claude/remote/plugins/` and loads them from
//! there; on a Windows remote machine a plugin that "holds a file or folder name Windows can't store,
//! such as a name with `:`" is not copied and "the session lists the plugins that Desktop left out";
//! "Plugins aren't available in WSL sessions". Pure over the paths the host listed: copying, the SSH
//! connection and the list the session shows are the host's.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{Pattern, regex_test};

/// Where the copies go on the remote machine, under its user's home directory.
pub const REMOTE_PLUGINS_DIRECTORY: &str = "~/.claude/remote/plugins/";

/// What the session runs on: an SSH remote machine of either system, or a WSL distribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteTarget {
    /// An SSH remote machine running a POSIX system.
    Posix,
    /// An SSH remote machine running Windows.
    Windows,
    /// A WSL distribution.
    Wsl,
}

/// A plugin and the paths inside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePluginFacts {
    /// `<name>@<marketplace>`.
    pub id: String,
    /// Every file and folder path inside the plugin, relative and `/`-separated.
    pub paths: Vec<String>,
}

/// What is copied and what is left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePluginPlan {
    /// The plugins to copy to `REMOTE_PLUGINS_DIRECTORY`, in the order given.
    pub copy: Vec<String>,
    /// The plugins left out, for the session to list; a WSL session leaves none out: it gets none.
    pub left_out: Vec<String>,
}

/// Judgment (spec/judgments.md): a name Windows can't store is one with a character it refuses in a
/// file name (`< > : " \ | ? *` or a control character), one ending in a space or a dot, or a
/// reserved device name with or without an extension. The docs name only `:`.
const fn is_refused_character(character: char) -> bool {
    matches!(character, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*')
}

/// Code points 0 to 31: all in the Basic Multilingual Plane, so one UTF-16 unit each.
fn has_control_character(segment: &str) -> bool {
    segment.chars().any(|character| u32::from(character) < 32)
}

/// `/^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.[^.]*)?$/i`: the letters fold by ASCII only.
static RESERVED_NAME: Pattern =
    LazyLock::new(|| Regex::new(r"^(?i-u:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.[^.]*)?$").ok());

/// `/[ .]$/`.
fn has_bad_ending(segment: &str) -> bool {
    segment.ends_with([' ', '.'])
}

fn storable_on_windows(segment: &str) -> bool {
    !segment.chars().any(is_refused_character)
        && !has_control_character(segment)
        && !regex_test(&RESERVED_NAME, segment)
        && !has_bad_ending(segment)
}

fn plugin_storable(plugin: &RemotePluginFacts) -> bool {
    plugin
        .paths
        .iter()
        .all(|path| path.split('/').all(storable_on_windows))
}

/// What each target does with a plugin: copy it, leave it out, or give the session none at all.
#[must_use]
pub fn plan_remote_plugins(
    target: RemoteTarget,
    plugins: &[RemotePluginFacts],
) -> RemotePluginPlan {
    match target {
        RemoteTarget::Posix => RemotePluginPlan {
            copy: plugins.iter().map(|plugin| plugin.id.clone()).collect(),
            left_out: Vec::new(),
        },
        RemoteTarget::Windows => RemotePluginPlan {
            copy: plugins
                .iter()
                .filter(|plugin| plugin_storable(plugin))
                .map(|plugin| plugin.id.clone())
                .collect(),
            left_out: plugins
                .iter()
                .filter(|plugin| !plugin_storable(plugin))
                .map(|plugin| plugin.id.clone())
                .collect(),
        },
        RemoteTarget::Wsl => RemotePluginPlan {
            copy: Vec::new(),
            left_out: Vec::new(),
        },
    }
}

crate::registry::exports! {
    "marketplace/remote-plugins" {
        "REMOTE_PLUGINS_DIRECTORY" => ok || REMOTE_PLUGINS_DIRECTORY;
        "planRemotePlugins" => ok |target: RemoteTarget, plugins: Vec<RemotePluginFacts>| plan_remote_plugins(target, &plugins);
    }
}
