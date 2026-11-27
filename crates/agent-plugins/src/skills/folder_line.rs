//! Port of `src/skills/folder-line.ts`, switched.

/// The line a skill's text opens with, its folder after it.
pub const BASE_DIRECTORY: &str = "Base directory for this skill: ";

/// The text the model receives: a skill's opens with its folder and a blank line, a command's (no
/// folder) is the body alone (Claude Code 2.1.319, the spec's model-content oracle; the docs do not
/// show the line).
#[must_use]
pub fn with_folder_line(text: &str, dir: Option<&str>) -> String {
    match dir {
        None => text.to_owned(),
        Some(dir) => format!("{BASE_DIRECTORY}{dir}\n\n{text}"),
    }
}

crate::registry::exports! {
    "skills/folder-line" {
        "BASE_DIRECTORY" => ok || BASE_DIRECTORY;
        "withFolderLine" => ok |text: String, dir: Option<String>| with_folder_line(&text, dir.as_deref());
    }
}
