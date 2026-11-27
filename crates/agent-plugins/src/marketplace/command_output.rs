//! Port of `src/marketplace/command-output.ts`, switched.
//!
//! What a `command` source printed (plugins/marketplace-reference.md, "What the command must do",
//! "Output that fails the install or update"): "exactly one line on stdout, the absolute path of the
//! plugin directory", not a UNC path on Windows, and not the directory Claude Code was started in
//! or one of its parents. Pure text handling; the messages are the kit's (spec/judgments.md).

use serde::Serialize;

use crate::format::path_text::{PathPlatform as Platform, is_within, without_trailing_slashes};

/// The absolute path the command printed, or what is wrong with its output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PrintedPath {
    /// The path.
    Path {
        /// The one line.
        path: String,
    },
    /// What is wrong.
    Problem {
        /// The message.
        problem: String,
    },
}

/// `^[A-Za-z]:[\\/]`.
fn is_windows_drive(line: &str) -> bool {
    let mut characters = line.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.next() == Some(':')
        && matches!(characters.next(), Some('\\' | '/'))
}

/// `^(?:\\\\|\/\/)`.
fn is_unc(line: &str) -> bool {
    line.starts_with("\\\\") || line.starts_with("//")
}

/// The one line, or why stdout is not one. A single trailing line break is the line's own.
fn single_line(stdout: &str) -> Option<&str> {
    let line = stdout
        .strip_suffix("\r\n")
        .or_else(|| stdout.strip_suffix('\n'))
        .unwrap_or(stdout);
    if line.is_empty() || line.contains(['\r', '\n']) {
        None
    } else {
        Some(line)
    }
}

/// The absolute path the command printed, or what is wrong with its output.
#[must_use]
pub fn read_printed_path(stdout: &str, platform: Platform) -> PrintedPath {
    let Some(line) = single_line(stdout) else {
        return PrintedPath::Problem {
            problem:
                "The command must print exactly one line, the absolute path of the plugin directory"
                    .to_owned(),
        };
    };
    if platform == Platform::Windows && is_unc(line) {
        return PrintedPath::Problem {
            problem: format!(
                "The printed path \"{line}\" is a UNC (network) path, which is not accepted on Windows"
            ),
        };
    }
    let absolute = match platform {
        Platform::Windows => is_windows_drive(line),
        Platform::Posix => line.starts_with('/'),
    };
    if !absolute {
        return PrintedPath::Problem {
            problem: format!("The printed path \"{line}\" is not an absolute path"),
        };
    }
    PrintedPath::Path {
        path: line.to_owned(),
    }
}

/// Separators as `/`, no trailing one, and on Windows no case: how two paths of one platform are compared.
fn comparable(path: &str, platform: Platform) -> String {
    let slashed = match platform {
        Platform::Windows => path.replace('\\', "/").to_lowercase(),
        Platform::Posix => path.to_owned(),
    };
    without_trailing_slashes(&slashed).to_owned()
}

/// True when two spellings name one folder on `platform`: on Windows separators and capitals do not tell folders apart.
#[must_use]
pub fn same_path(a: &str, b: &str, platform: Platform) -> bool {
    comparable(a, platform) == comparable(b, platform)
}

/// True when `printed` is `sessionDirectory` or one of its parents ("the session's own directory").
#[must_use]
pub fn contains_session(
    printed: &str,
    session_directory: Option<&str>,
    platform: Platform,
) -> bool {
    let Some(session_directory) = session_directory else {
        return false;
    };
    let directory = comparable(printed, platform);
    let session = comparable(session_directory, platform);
    is_within(&directory, &session)
}

crate::registry::exports! {
    "marketplace/command-output" {
        "samePath" => ok |a: String, b: String, platform: Platform| same_path(&a, &b, platform);
    }
}
