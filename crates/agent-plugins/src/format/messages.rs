//! Port of `src/format/messages.ts`, switched.
//! The load messages of a plugin's component paths, as Claude Code shows them in the `/plugin`
//! Errors tab and `claude plugin list` (plugins/manifest-reference.md, "Containment and
//! existence" and "How each key combines with its default location"; errors.md, "Path escapes
//! plugin directory"). `component` is the field that declared the path (`commands`, `hooks`…).

use super::issues::{PluginIssue, error};

/// `<component> path escapes plugin directory: <path>`, with where a symlink really leads;
/// `with_marketplace` is the form for a plugin's own file in a local marketplace.
#[must_use]
pub fn path_escapes(
    component: &str,
    path: &str,
    resolves_to: Option<&str>,
    with_marketplace: bool,
) -> String {
    match resolves_to {
        None => format!("{component} path escapes plugin directory: {path}"),
        Some(target) => {
            let marketplace = if with_marketplace {
                " and its marketplace directory"
            } else {
                ""
            };
            format!(
                "{component} path escapes plugin directory: {path} — it resolves to {target}, outside the plugin directory{marketplace}"
            )
        }
    }
}

/// What `<component> path escapes plugin directory: <path>` adds when the path stays inside but
/// holds a backslash on macOS or Linux (errors.md, "Path escapes plugin directory").
pub const BACKSLASH_TAIL: &str =
    " — its path contains a backslash, which is not resolved reliably on this platform";

/// `<component> path not found: <path>`.
#[must_use]
pub fn path_not_found(component: &str, path: &str) -> String {
    format!("{component} path not found: {path}")
}

/// The guidance of a declared path that points at nothing (plugins/troubleshooting.md,
/// "commands path not found").
const PATH_NOT_FOUND_GUIDANCE: &str =
    "Check that the path in your manifest or marketplace config is correct";

/// The load error of a declared component path that points at nothing, with its guidance: the
/// docs name `commands`, `skills`, `agents` and `hooks`; the crate gives it every component path
/// the loader checks (judgment, spec/judgments.md). `mcpServers` keeps its own reader's issue.
#[must_use]
pub fn missing_path(component: &str, path: &str) -> PluginIssue {
    PluginIssue {
        guidance: Some(PATH_NOT_FOUND_GUIDANCE.to_owned()),
        ..error(component, &path_not_found(component, path))
    }
}

/// The error for a `skills` entry that is not a directory ("Path-only fields": "`skills` entries
/// must be directories").
#[must_use]
pub fn skills_entry_not_directory(path: &str) -> String {
    format!("skills entries must be directories: {path}")
}

/// The error for an `agents` entry that is not a file ("Path-only fields": "`agents` entries
/// must be `.md` files").
#[must_use]
pub fn agents_entry_not_file(path: &str) -> String {
    format!("agents entries must be .md files: {path}")
}

/// The warning for a default folder a replacing manifest key leaves unscanned.
#[must_use]
pub fn default_folder_ignored(folder: &str, key: &str) -> String {
    format!("Default {folder}/ folder is ignored because the manifest sets \"{key}\"")
}

crate::registry::exports! {
    "format/messages" {
        "pathNotFound" => ok |component: String, path: String| path_not_found(&component, &path);
        "missingPath" => ok |component: String, path: String| missing_path(&component, &path);
        "defaultFolderIgnored" => ok |folder: String, key: String| default_folder_ignored(&folder, &key);
    }
}
