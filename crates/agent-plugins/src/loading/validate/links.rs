//! Ported from `src/loading/validate/links.ts`; only the core uses it now, so it has no wrapper.
//!
//! What the validator says about links (oracle 2.1.319). It never follows a symlink: a folder that
//! is one is not read, an entry that is one is not read, and each warns, because Claude Code does
//! follow them in some places. A plugin's own folders and a project's `.claude` folders word it
//! differently, for a session always follows the links of a project it loads.

use serde::{Deserialize, Serialize};

use crate::js::number_to_string;

use crate::format::validate::finding::{ValidationIssue, warning_in};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;

/// The two kinds of folder the validator reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderStyle {
    /// A plugin's own folder.
    Plugin,
    /// A project's `.claude` folder.
    Project,
}

const INSTALLED_FOLLOWS: &str = "inside the plugin's installed folder, or, for a plugin read in place from a local marketplace folder, inside that folder; a project's .claude/skills plugin, only if it points inside the folder holding that .claude.";

fn folder_link() -> String {
    format!(
        "This directory is a symlink, so the validator read nothing in it. An installed plugin loads it only if it points {INSTALLED_FOLLOWS} Otherwise replace the link with a copy of the folder. To check its files, validate the folder it points to."
    )
}

impl FolderStyle {
    /// What a session does with a link of this kind of folder.
    fn followed_at(self) -> String {
        match self {
            Self::Plugin => format!(
                "A session started with --plugin-dir on this plugin follows a symlink here. An installed plugin follows one only if it points {INSTALLED_FOLLOWS} To check what a symlink points to, validate that path."
            ),
            Self::Project => "A session loading this directory does follow them, so validate the real paths separately.".to_owned(),
        }
    }
}

const NOT_REGULAR: &str = "the path is not a regular file (a symlink, a FIFO, a directory). A FIFO or a directory is never read.";

/// Whether `path` is itself a symbolic link.
pub async fn is_link(files: &dyn PluginFiles, path: &str, signal: Option<&Signal>) -> bool {
    files
        .stat(path, signal)
        .await
        .is_some_and(|stat| stat.is_symlink)
}

/// A folder that is itself a link: nothing in it is read.
#[must_use]
pub fn linked_folder(folder: &str) -> ValidationIssue {
    warning_in(folder, "directory", &folder_link())
}

const fn plural(count: f64, one: &'static str, many: &'static str) -> &'static str {
    if count == 1.0 { one } else { many }
}

/// The entries of a folder that are links and were skipped: "3 entries here are symlinks …".
#[must_use]
pub fn linked_entries(folder: &str, count: f64, style: FolderStyle) -> Vec<ValidationIssue> {
    if count == 0.0 {
        return Vec::new();
    }
    let noun = plural(
        count,
        "entry here is a symlink and was",
        "entries here are symlinks and were",
    );
    let message = format!(
        "{} {noun} not read — components are read without following symlinks. {}",
        number_to_string(count),
        style.followed_at()
    );
    vec![warning_in(folder, "directory", &message)]
}

/// Skill files that exist but are not regular files: "1 component here was not read — …".
#[must_use]
pub fn unread_components(folder: &str, count: f64, style: FolderStyle) -> Vec<ValidationIssue> {
    if count == 0.0 {
        return Vec::new();
    }
    let noun = plural(count, "component here was", "components here were");
    let message = format!(
        "{} {noun} not read — {NOT_REGULAR} {}",
        number_to_string(count),
        style.followed_at()
    );
    vec![warning_in(folder, "directory", &message)]
}

/// What every file the validator reads shares: it follows no symlink and stops at a size.
macro_rules! cap {
    () => {
        "are read without following symlinks and are capped at 1048576 bytes."
    };
}

/// The hooks folder is a link, or `hooks.json` is not a regular file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HooksLink {
    /// The folder is a link.
    pub folder: &'static str,
    /// `hooks.json` is not a regular file.
    pub file: &'static str,
}

/// The hooks folder is a link, or `hooks.json` is not a regular file.
pub const HOOKS_LINK: HooksLink = HooksLink {
    folder: concat!(
        "The hooks directory is a link (a symlink or a Windows junction) and was not read — hooks ",
        cap!(),
        " The plugin loader has neither limit and fails the whole plugin on bad hook config, so validate the real file separately."
    ),
    file: concat!(
        "hooks.json is not a regular file (a symlink, a FIFO, a directory) — hooks ",
        cap!(),
        " The plugin loader has neither limit and fails the whole plugin on bad hook config, so validate the real file separately."
    ),
};

/// The message of an MCP server file that is not a regular file.
pub const MCP_NOT_REGULAR: &str = concat!(
    "The file is not a regular file (a symlink, a FIFO, a directory) and was not read — MCP server files ",
    cap!(),
    " The plugin loader has neither limit, so validate the real file separately."
);

/// The most of an MCP server file the validator reads (oracle 2.1.325, spec J8.27).
pub const MCP_FILE_CAP_BYTES: f64 = 1_048_576.0;

/// The message of an MCP server file past the cap.
pub const MCP_PAST_CAP: &str = concat!(
    "The file is past the size cap and was not read — MCP server files ",
    cap!(),
    " The plugin loader has neither limit, so validate the real file separately."
);
