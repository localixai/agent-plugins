//! Port of `src/loading/validate/paths.ts`, switched.

use crate::format::listing::list_sorted;
use crate::ports::Signal;
use crate::ports::files::{DirectoryEntry, PluginFiles};
use crate::registry::Port;

/// A manifest path (`./skills/a`, `./`) as a `PluginFiles` path: no `./`, no trailing slash, `.` for the root.
#[must_use]
pub fn plugin_path(path: &str) -> String {
    let mut rest = path;
    while let Some(stripped) = rest.strip_prefix("./") {
        rest = stripped;
    }
    let trimmed = rest.trim_end_matches('/');
    if trimmed.is_empty() {
        ".".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `name` below `dir`; a name that is `.` is `dir` itself.
#[must_use]
pub fn child_of(dir: &str, name: &str) -> String {
    if name == "." {
        return dir.to_owned();
    }
    if dir == "." {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// A folder's entries, or none when it does not exist.
pub async fn entries_of(
    files: &dyn PluginFiles,
    dir: &str,
    signal: Option<&Signal>,
) -> Vec<DirectoryEntry> {
    list_sorted(files, dir, signal).await.unwrap_or_default()
}

crate::registry::exports! {
    "loading/validate/paths" {
        "pluginPath" => ok |path: String| plugin_path(&path);
        "childOf" => ok |dir: String, name: String| child_of(&dir, &name);
        "entriesOf" => async ok |files: Port, dir: String, signal: Option<Signal>|
            entries_of(&files, &dir, signal.as_ref()).await;
    }
}
