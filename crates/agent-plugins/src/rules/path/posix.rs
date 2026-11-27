//! Ported from `src/rules/path/posix.ts`; only the core uses it now, so it has no wrapper.
//!
//! Path arithmetic on the kit's one path form: absolute, `/`-separated, no `.` or `..` segments.
//! Pure string work, because the kit never calls the host's `path` module (docs/ARCHITECTURE.md, L3)
//! and path rules match on POSIX form everywhere (permissions.md, "Where a pattern anchors").

use serde::{Deserialize, Serialize};

use crate::js::slice_code_units;

/// `PathContext['platform']`.
pub type Platform = crate::rules::types::Platform;

/// `C:\Users\alice` becomes `/c/Users/alice`: "On Windows, paths are normalized to POSIX form before
/// matching" (permissions.md). A UNC path `\\server\share` keeps its two leading slashes' meaning
/// through the later folding to `/server/share`; networks are told apart on the raw spelling
/// (`network.ts`).
///
/// `DRIVE_PREFIX`, `/^([A-Za-z]):(?=[\\/]|$)/`: the drive letter, when the path starts with one.
fn drive_prefix(path: &str) -> Option<char> {
    let mut characters = path.chars();
    let drive = characters.next().filter(char::is_ascii_alphabetic)?;
    let colon = characters.next() == Some(':');
    let follows = characters.next();
    (colon && matches!(follows, None | Some('\\' | '/'))).then_some(drive)
}

fn normalize(platform: Platform, path: &str) -> String {
    match platform {
        Platform::Posix => path.to_owned(),
        Platform::Windows => {
            let slashed = path.replace('\\', "/");
            match drive_prefix(&slashed) {
                Some(drive) => format!(
                    "/{}{}",
                    drive.to_ascii_lowercase(),
                    slashed.get(2..).unwrap_or_default()
                ),
                None => slashed,
            }
        }
    }
}

/// A path in POSIX form (`NORMALIZERS[platform]`).
#[must_use]
pub fn to_posix_path(path: &str, platform: Platform) -> String {
    normalize(platform, path)
}

/// Folds `.`, `..` and repeated slashes of a POSIX path; the result is absolute.
#[must_use]
pub fn normalize_absolute(path: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            kept.pop();
        } else {
            kept.push(segment);
        }
    }
    format!("/{}", kept.join("/"))
}

/// `path` against `base` (a normalised absolute path), normalised; an absolute `path` ignores `base`.
#[must_use]
pub fn resolve_path(base: &str, path: &str) -> String {
    if path.starts_with('/') {
        normalize_absolute(path)
    } else {
        normalize_absolute(&format!("{base}/{path}"))
    }
}

/// A path with a leading `~` spelled as the home directory: `~` alone or before a separator; `~alice`
/// is a folder of that name (spec/judgments.md).
#[must_use]
pub fn home_expanded(path: &str, home: &str) -> String {
    match path.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['\\', '/']) => format!("{home}{rest}"),
        Some(_) | None => path.to_owned(),
    }
}

/// `Pick<PathContext, 'cwd' | 'home' | 'platform'>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPaths {
    /// The session's current directory.
    pub cwd: String,
    /// The home directory.
    pub home: String,
    /// How the host writes paths.
    pub platform: Platform,
}

impl From<&crate::rules::types::PathContext> for HostPaths {
    /// The `cwd`, `home` and `platform` of a `PathContext`, as the TypeScript passes one where a
    /// `Pick` of it is asked.
    fn from(paths: &crate::rules::types::PathContext) -> Self {
        Self {
            cwd: paths.cwd.clone(),
            home: paths.home.clone(),
            platform: paths.platform,
        }
    }
}

/// A host path in POSIX form, `~` expanded and resolved against the session's current directory:
/// the one reading of a path the rules, the working directories and the modes share.
#[must_use]
pub fn absolute_path(path: &str, paths: &HostPaths) -> String {
    let cwd = normalize_absolute(&to_posix_path(&paths.cwd, paths.platform));
    resolve_path(
        &cwd,
        &to_posix_path(&home_expanded(path, &paths.home), paths.platform),
    )
}

/// The spelling two paths are compared in. Windows names compare without case and drop the dots and
/// spaces that end them (`.GIT`, `.git.` and `.git ` are `.git`); POSIX names compare as written,
/// macOS included (spec/judgments.md).
fn fold(platform: Platform, path: &str) -> String {
    match platform {
        Platform::Posix => path.to_owned(),
        Platform::Windows => path
            .split('/')
            .map(|segment| {
                if segment == "." || segment == ".." {
                    segment
                } else {
                    segment.trim_end_matches(['.', ' '])
                }
            })
            .collect::<Vec<_>>()
            .join("/")
            .to_lowercase(),
    }
}

/// `FOLDS[platform](path)`.
#[must_use]
pub fn fold_path(path: &str, platform: Platform) -> String {
    fold(platform, path)
}

/// The text of `path` below `root` (`''` for the root itself), or `undefined` when it is not below;
/// on Windows both are folded first (`foldPath`), and so is the text returned.
#[must_use]
pub fn relative_within(root: &str, path: &str, platform: Option<Platform>) -> Option<String> {
    let platform = platform.unwrap_or(Platform::Posix);
    let base = fold_path(root, platform);
    let below = fold_path(path, platform);
    if base == "/" {
        return Some(slice_code_units(&below, 1, None));
    }
    if below == base {
        return Some(String::new());
    }
    below.strip_prefix(&format!("{base}/")).map(str::to_owned)
}

/// The directory a path is in: `/` for a path with no slash past its first character.
#[must_use]
pub fn dirname_of(path: &str) -> String {
    match path.rfind('/') {
        None | Some(0) => "/".to_owned(),
        Some(cut) => path.get(..cut).unwrap_or_default().to_owned(),
    }
}

/// Whether a segment holds a gitignore wildcard (or an escape), so a literal prefix ends before it.
#[must_use]
pub fn has_glob_syntax(segment: &str) -> bool {
    segment.contains(['*', '?', '[', '\\'])
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { foldPath, toPosixPath, relativeWithin } from "./src/rules/path/posix.ts";
    //   console.log(JSON.stringify([foldPath("/A/.GIT. /x \u{3000}/..", "windows"),
    //   toPosixPath("C:", "windows"), toPosixPath("c:x", "windows"), relativeWithin("/", "\u{1F600}x")]))'
    // `[. ]+$` takes ASCII dots and spaces only; the drive needs a separator or the end after it.
    #[test]
    fn windows_paths_fold_as_the_typescript_folds_them() {
        assert_eq!(
            fold_path("/A/.GIT. /x \u{3000}/..", Platform::Windows),
            "/a/.git/x \u{3000}/.."
        );
        assert_eq!(to_posix_path("C:", Platform::Windows), "/c");
        assert_eq!(to_posix_path("c:x", Platform::Windows), "c:x");
        assert_eq!(
            relative_within("/", "\u{1F600}x", None).as_deref(),
            Some("\u{FFFD}x")
        );
    }
}
