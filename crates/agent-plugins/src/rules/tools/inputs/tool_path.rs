//! Port of `src/rules/tools/inputs/tool-path.ts`, switched.

use crate::js::{JsObject, JsValue};
use crate::rules::path::posix::{HostPaths, absolute_path, home_expanded, normalize_absolute};
use crate::rules::types::Platform;

/// `DRIVE_FOLDER`, `/^\/([a-z])(?:\/(.*))?$/`: a drive letter written as the first folder, and what
/// follows it (`.` does not match a line terminator, so a path holding one is no drive path).
fn drive_folder(path: &str) -> Option<(char, &str)> {
    let rest = path.strip_prefix('/')?;
    let mut characters = rest.chars();
    let letter = characters.next().filter(char::is_ascii_lowercase)?;
    let after = characters.as_str();
    if after.is_empty() {
        return Some((letter, ""));
    }
    let tail = after.strip_prefix('/')?;
    let has_terminator = tail.contains(['\n', '\r', '\u{2028}', '\u{2029}']);
    (!has_terminator).then_some((letter, tail))
}

/// `WINDOWS_LOOK`: how a platform writes the path.
fn windows_look(platform: Platform, path: String) -> String {
    match platform {
        Platform::Posix => path,
        Platform::Windows => match drive_folder(&path) {
            None => path.replace('/', "\\"),
            Some((letter, rest)) => format!(
                "{}:\\{}",
                letter.to_ascii_uppercase(),
                rest.replace('/', "\\")
            ),
        },
    }
}

fn is_unc(path: &str) -> bool {
    path.starts_with("\\\\") || path.starts_with("//")
}

/// The absolute path Claude Code hands a hook for a file tool (hooks.md, "PreToolUse input"): "expands
/// `~` and relative paths before hooks run", and "On Windows, the path arrives with backslash
/// separators". `.` and `..` fold, in an absolute path too, so no spelling hides the target; a UNC
/// path keeps its two leading backslashes (spec/judgments.md).
#[must_use]
pub fn expand_tool_path(path: &str, at: &HostPaths) -> String {
    let spelled = home_expanded(path, &at.home);
    if at.platform == Platform::Windows && is_unc(&spelled) {
        let folded = normalize_absolute(&spelled.replace('\\', "/")).replace('/', "\\");
        return format!("\\{folded}");
    }
    windows_look(at.platform, absolute_path(&spelled, at))
}

/// "For the file tools `Write`, `Edit`, and `Read`, `tool_input.file_path` is always absolute."
const FILE_TOOLS: [&str; 3] = ["Write", "Edit", "Read"];

/// A copy of a tool call's input as a hook receives it: the `file_path` of `Write`, `Edit` and `Read`
/// expanded (`expandToolPath`), every other field and tool untouched. An input that is not an object
/// has no fields, and a `file_path` that is not text is left as it was.
#[must_use]
pub fn expand_tool_input_path(tool_name: &str, input: &JsValue, at: &HostPaths) -> JsObject {
    let Some(members) = input.as_object() else {
        return JsObject::new();
    };
    let mut copy = members.clone();
    if FILE_TOOLS.contains(&tool_name)
        && let Some(path) = members.get("file_path").and_then(JsValue::as_str)
    {
        copy.insert(
            "file_path".to_owned(),
            JsValue::String(expand_tool_path(path, at)),
        );
    }
    copy
}

crate::registry::exports! {
    "rules/tools/inputs/tool-path" {
        "expandToolPath" => ok |path: String, at: HostPaths| expand_tool_path(&path, &at);
        "expandToolInputPath" => ok |tool_name: String, input: JsValue, at: HostPaths| expand_tool_input_path(&tool_name, &input, &at);
    }
}
