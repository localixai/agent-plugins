//! Port of `src/format/path-text.ts`, switched.
//!
//! Path text: the one owner of what a path says as written (absolute, climbing, where it resolves)
//! and of the `PluginFiles` spelling (plugins/loading.md, "Paths that escape the plugin
//! directory"; plugins/manifest-reference.md, "Path rules"). Pure over strings: nothing is looked
//! up. A backslash is a separator on Windows only; "on macOS and Linux" it is part of a name.

use serde::{Deserialize, Serialize};

/// Whose path rules apply: a backslash separates on `windows` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathPlatform {
    /// macOS and Linux: `/` separates; a backslash is part of a name.
    #[default]
    Posix,
    /// Windows: both `/` and a backslash separate.
    Windows,
}

/// A path from a file system root: `/x`, `\x`, or a drive (`C:`, `C:/x`, `C:\x`).
#[must_use]
pub fn is_absolute_path(path: &str) -> bool {
    let mut characters = path.chars();
    match characters.next() {
        Some('\\' | '/') => true,
        Some(first) => first.is_ascii_alphabetic() && characters.next() == Some(':'),
        None => false,
    }
}

/// The segments of `path`, split on the platform's separators.
fn segments_of(path: &str, platform: PathPlatform) -> Vec<&str> {
    match platform {
        PathPlatform::Windows => path.split(['\\', '/']).collect(),
        PathPlatform::Posix => path.split('/').collect(),
    }
}

/// The named segments of a relative path once `.`, empty segments and `..` are resolved, or
/// `None` when it climbs above where it starts.
#[must_use]
pub fn resolve_segments(path: &str, platform: PathPlatform) -> Option<Vec<String>> {
    let mut kept: Vec<String> = Vec::new();
    for part in segments_of(path, platform) {
        if part == ".." && kept.is_empty() {
            return None;
        }
        if part == ".." {
            kept.pop();
        } else if !part.is_empty() && part != "." {
            kept.push(part.to_owned());
        }
    }
    Some(kept)
}

/// Whether a path names a place outside the folder it is relative to: absolute, or climbing above it.
#[must_use]
pub fn leaves_folder(path: &str, platform: PathPlatform) -> bool {
    is_absolute_path(path) || resolve_segments(path, platform).is_none()
}

/// A backslash in a component path, which "on macOS and Linux" is refused even inside the plugin.
#[must_use]
pub fn refused_backslash(path: &str, platform: PathPlatform) -> bool {
    platform == PathPlatform::Posix && path.contains('\\')
}

/// Whether a component path leaves the plugin as written (plugins/loading.md, "Paths that escape the
/// plugin directory"): it climbs with `..`, is absolute, or, "on macOS and Linux, contains a
/// backslash anywhere in it, even when the path stays inside the plugin". On Windows a backslash is
/// a separator, so `..\x` climbs there too. Any `..` segment counts, even one that comes back down.
#[must_use]
pub fn escapes_as_written(path: &str, platform: PathPlatform) -> bool {
    if is_absolute_path(path) {
        return true;
    }
    if refused_backslash(path, platform) {
        return true;
    }
    segments_of(path, platform).contains(&"..")
}

/// A component path as a `PluginFiles` path: relative to the root, `/`-separated, no `.` or empty
/// segment, no trailing slash; the plugin root is `.` ("Both `"."` and `"./"` denote the plugin
/// root"). `.//x` is `x`, as a file system resolves it under the root: the result never starts
/// with `/`, so a port that resolves it against the root stays there. On Windows a backslash
/// separates like `/`. `..` is kept; `locate` rejects it before any read. (The TypeScript
/// defaults `platform` to `posix`; here `PathPlatform::default()` is that.)
#[must_use]
pub fn root_relative(path: &str, platform: PathPlatform) -> String {
    let parts: Vec<&str> = segments_of(path, platform)
        .into_iter()
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

/// `base` and a child name, as a root-relative path.
#[must_use]
pub fn child_path(base: &str, name: &str) -> String {
    if base == "." {
        name.to_owned()
    } else {
        format!("{base}/{name}")
    }
}

/// A character `.` does not match in a JavaScript regular expression without the `s` flag.
const fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `path.replace(/(?<=.)\/+$/, '')`: the trailing slashes go when a character stands before them
/// (a line terminator does not count); when none does, a run of two or more keeps its first
/// slash, because the match then starts at the second, which the first one precedes.
fn without_slashes_after_a_character(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let run = path.len() - trimmed.len();
    match trimmed.chars().next_back() {
        Some(before) if !is_line_terminator(before) => trimmed.to_owned(),
        _ if run >= 2 => format!("{trimmed}/"),
        _ => path.to_owned(),
    }
}

/// An absolute path in `/` spelling: a Windows path's backslashes as `/`, no trailing slash.
fn forward(path: &str, platform: PathPlatform) -> String {
    match platform {
        PathPlatform::Windows => without_slashes_after_a_character(&path.replace('\\', "/")),
        PathPlatform::Posix => without_slashes_after_a_character(path),
    }
}

/// Where `path` leads from the folder `root` (an absolute, `/`-spelled path): an absolute `path`
/// as itself in `/` spelling, a relative one joined to `root` with `.` and `..` resolved.
#[must_use]
pub fn resolve_from(root: &str, path: &str, platform: PathPlatform) -> String {
    if is_absolute_path(path) {
        return forward(path, platform);
    }
    let base = forward(root, platform);
    let mut parts: Vec<&str> = base.split('/').collect();
    for part in segments_of(path, platform) {
        if part == ".." {
            parts.pop();
        } else if part != "." && !part.is_empty() {
            parts.push(part);
        }
    }
    parts.join("/")
}

/// `path` with no trailing `/` (a root `/` becomes empty).
#[must_use]
pub fn without_trailing_slashes(path: &str) -> &str {
    path.trim_end_matches('/')
}

/// True when `path` is `root` or inside it, by whole path segments (both in one `/` spelling).
#[must_use]
pub fn is_within(root: &str, path: &str) -> bool {
    let base = without_trailing_slashes(root);
    let candidate = without_trailing_slashes(path);
    candidate == base || candidate.starts_with(&format!("{base}/"))
}

crate::registry::exports! {
    "format/path-text" {
        "isAbsolutePath" => ok |path: String| is_absolute_path(&path);
        "resolveSegments" => ok |path: String, platform: PathPlatform| resolve_segments(&path, platform);
        "leavesFolder" => ok |path: String, platform: PathPlatform| leaves_folder(&path, platform);
        "refusedBackslash" => ok |path: String, platform: PathPlatform| refused_backslash(&path, platform);
        "escapesAsWritten" => ok |path: String, platform: PathPlatform| escapes_as_written(&path, platform);
        "rootRelative" => ok |path: String, platform: Option<PathPlatform>| root_relative(&path, platform.unwrap_or_default());
        "childPath" => ok |base: String, name: String| child_path(&base, &name);
        "resolveFrom" => ok |root: String, path: String, platform: PathPlatform| resolve_from(&root, &path, platform);
        "withoutTrailingSlashes" => ok |path: String| without_trailing_slashes(&path).to_owned();
        "isWithin" => ok |root: String, path: String| is_within(&root, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_slashes_go_only_behind_a_character() {
        let cases = [
            ("/", "/"),
            ("//", "/"),
            ("a/", "a"),
            ("a//", "a"),
            ("\n/", "\n/"),
            ("\n//", "\n/"),
            ("/a/", "/a"),
        ];
        for (path, expected) in cases {
            assert_eq!(forward(path, PathPlatform::Posix), expected, "{path:?}");
        }
        assert_eq!(forward("C:\\x\\", PathPlatform::Windows), "C:/x");
    }
}
