//! Port of `src/format/core/command.ts`, switched.
//!
//! The `command` of a stdio MCP server or a hook handler (core §7.2.1, §7.3.1): one executable token,
//! never a shell command string, and either a bare executable name, resolved by the platform's
//! search, or a plugin-relative path that begins with `./`, resolved against the plugin root and
//! contained in it (core §4.1 item 4). No placeholder is expanded in it (core §9.2).
//!
//! Judgment (spec/judgments.md): a bare name is one token when it holds no whitespace and no path
//! separator, so `npx tsx`, `bin/server`, `/usr/bin/node`, `../server` and `${PLUGIN_ROOT}/server`
//! are refused; a `./` path is refused when it climbs with `..` or holds a backslash as written.

use crate::js::JsValue;

use crate::format::path_text::{PathPlatform, escapes_as_written, root_relative};
use crate::js::is_whitespace;

const RELATIVE: &str = "./";

/// The root-relative path a `./` command names, or `None` for a bare name.
#[must_use]
pub fn relative_command_path(command: &str) -> Option<String> {
    command
        .strip_prefix(RELATIVE)
        .map(|rest| root_relative(rest, PathPlatform::Posix))
}

fn relative_problem(command: &str) -> Option<&'static str> {
    let rest = command.strip_prefix(RELATIVE).unwrap_or(command);
    if rest.chars().any(is_whitespace) {
        return Some("must be one executable token, not a shell command");
    }
    if escapes_as_written(rest, PathPlatform::Posix) {
        return Some("is a plugin-relative path that leaves the plugin root");
    }
    if root_relative(rest, PathPlatform::Posix) == "." {
        Some("is a plugin-relative path that names no file")
    } else {
        None
    }
}

fn bare_problem(command: &str) -> Option<&'static str> {
    if command.chars().any(is_whitespace) {
        return Some("must be one executable token, not a shell command");
    }
    if command.contains("${") {
        return Some("is not expanded: a placeholder cannot appear in \"command\"");
    }
    if command.contains(['\\', '/']) || command == "." || command == ".." {
        return Some("must be a bare executable name or a path that begins with \"./\"");
    }
    None
}

/// Why a `command` value breaks core §7.2.1 / §7.3.1, or `None` when it is a valid token. The
/// value is `unknown` in the package, so it is a JSON value here.
#[must_use]
pub fn command_problem(command: &JsValue) -> Option<&'static str> {
    let Some(text) = command.as_str().filter(|text| !text.is_empty()) else {
        return Some("must be a non-empty string");
    };
    if text.starts_with(RELATIVE) {
        relative_problem(text)
    } else {
        bare_problem(text)
    }
}

crate::registry::exports! {
    "format/core/command" {
        "commandProblem" => ok |command: JsValue| command_problem(&command);
    }
}
