//! Port of `src/hooks/output/worktree-path.ts`, switched.
//!
//! What a `WorktreeCreate` hook returned, read as the worktree path (hooks.md, "WorktreeCreate
//! output"): a command hook prints it "as the last non-empty line of stdout" and "Claude Code strips
//! ANSI escape codes before reading that line"; an HTTP hook returns `hookSpecificOutput.worktreePath`.
//! "If the hook fails or produces no path, worktree creation fails." A relative path is resolved
//! "against the directory the hook ran in, collapsing any `.` or `..` segments in it"; "Claude Code
//! refuses an absolute path that contains `.` or `..` segments, and any path that passes through a
//! symlink below the repository root", and "the error names the rejected component". Whether the
//! resulting path is a directory the session can enter is the host's: it is the one that can look.
//!
//! Text is read in UTF-16 code units, as the TypeScript indexes it.

use serde::{Deserialize, Serialize};

use crate::js::trim;
use crate::registry::{Callback, Port};

const ESC: u16 = 0x1b;
const BEL: u16 = 0x07;

/// The index after the CSI sequence whose parameters start at `from`: up to one final byte, `@` to `~`.
fn csi_end(text: &[u16], from: usize) -> usize {
    let mut cursor = from;
    while let Some(&code) = text.get(cursor) {
        cursor += 1;
        if (0x40..=0x7e).contains(&code) {
            break;
        }
    }
    cursor
}

/// The index after the OSC string that starts at `from`: up to BEL, or the terminator `ESC \`.
fn osc_end(text: &[u16], from: usize) -> usize {
    let mut cursor = from;
    while let Some(&code) = text.get(cursor) {
        if code == BEL {
            return cursor + 1;
        }
        if code == ESC && text.get(cursor + 1) == Some(&u16::from(b'\\')) {
            return cursor + 2;
        }
        cursor += 1;
    }
    cursor
}

/// The index after the escape sequence that starts at `at` (an `ESC`), as a terminal reads it.
fn escape_end(text: &[u16], at: usize) -> usize {
    let kind = text.get(at + 1).copied();
    if kind == Some(u16::from(b'[')) {
        return csi_end(text, at + 2);
    }
    if kind == Some(u16::from(b']')) {
        return osc_end(text, at + 2);
    }
    (at + 2).min(text.len())
}

/// `text` without its ANSI escape sequences (CSI, OSC and two-character escapes).
#[must_use]
pub fn strip_ansi(text: &str) -> String {
    let text: Vec<u16> = text.encode_utf16().collect();
    let mut kept: Vec<u16> = Vec::new();
    let mut cursor = 0;
    while let Some(&unit) = text.get(cursor) {
        if unit == ESC {
            cursor = escape_end(&text, cursor);
        } else {
            kept.push(unit);
            cursor += 1;
        }
    }
    String::from_utf16_lossy(&kept)
}

/// The last non-empty line of `stdout` after the ANSI escapes are stripped, or `None`.
#[must_use]
pub fn last_path_line(stdout: &str) -> Option<String> {
    strip_ansi(stdout)
        .split('\n')
        .map(|line| trim(line.strip_suffix('\r').unwrap_or(line)).to_owned())
        .rfind(|line| !line.is_empty())
}

/// The path, or why creation fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WorktreePath {
    /// The path the session uses.
    Path {
        /// The resolved path.
        path: String,
    },
    /// Why there is none.
    Failure {
        /// The reason creation fails.
        failure: String,
    },
}

/// What the host knows about where the hook ran.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeLocation {
    /// The directory the hook ran in: a relative path is resolved against it.
    pub ran_in: String,
    /// The repository root: only a symlink below it is refused.
    #[serde(default)]
    pub repository_root: Option<String>,
    /// Whether a path is itself a symbolic link. The kit reads no file system, so the host answers;
    /// without it no path is checked for links.
    #[serde(default)]
    pub is_symlink: Option<Callback>,
}

/// A [`WorktreeLocation`] as it crosses: data, or, when it holds `isSymlink` (a function), a port
/// whose members are read (docs/RUST.md, "Effects").
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LocationGiven {
    /// The host's record, holding a function.
    Host(Port),
    /// Plain data.
    Data(WorktreeLocation),
}

impl LocationGiven {
    /// The location's members, read from the host's record when it is one.
    async fn read(self) -> WorktreeLocation {
        match self {
            Self::Data(location) => location,
            Self::Host(record) => {
                let ran_in = record.member("ranIn");
                let repository_root = record.member("repositoryRoot");
                let is_symlink = record.member("isSymlink");
                WorktreeLocation {
                    ran_in: ran_in.await,
                    repository_root: repository_root.await,
                    is_symlink: is_symlink.await,
                }
            }
        }
    }
}

/// What a merged `WorktreeCreate` outcome holds of the hooks' answers: its effects.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeEffects {
    /// The merged path.
    #[serde(default)]
    pub worktree_path: Option<String>,
}

/// What a merged `WorktreeCreate` outcome holds of the hooks' answers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorktreeAnswer {
    /// Whether the dispatch went ahead.
    pub proceed: bool,
    /// The stdout of each command hook that exited 0.
    pub stdout: Vec<String>,
    /// The merged effects.
    pub effects: WorktreeEffects,
}

/// `/^[A-Za-z]:[\\/]/`.
fn windows_drive(path: &str) -> bool {
    let mut characters = path.chars();
    matches!(
        (characters.next(), characters.next(), characters.next()),
        (Some(drive), Some(':'), Some('\\' | '/')) if drive.is_ascii_alphabetic()
    )
}

/// Whether the path starts at a root: `/` or a drive.
#[must_use]
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || windows_drive(path)
}

fn segments_of(path: &str) -> Vec<&str> {
    path.split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect()
}

fn join_with(separator: &str, root: &str, parts: &[&str]) -> String {
    format!("{root}{}", parts.join(separator))
}

/// The leading root of an absolute path (`/` or `C:\`), kept as written: `path.slice(0, 3)`, whole
/// characters, as a drive's three are.
fn root_of(path: &str) -> &str {
    if path.starts_with('/') {
        return "/";
    }
    path.char_indices()
        .nth(3)
        .map_or(path, |(end, _)| path.get(..end).unwrap_or(path))
}

fn separator_of(path: &str) -> &'static str {
    if path.contains('\\') && !path.contains('/') {
        "\\"
    } else {
        "/"
    }
}

/// `relative` resolved against `base`, the `.` and `..` segments collapsed (`..` stops at the root).
#[must_use]
pub fn resolve_relative(base: &str, relative: &str) -> String {
    let rooted = if is_absolute(base) {
        base.get(root_of(base).len()..).unwrap_or_default()
    } else {
        base
    };
    let mut parts = segments_of(rooted);
    for segment in segments_of(relative) {
        if segment == ".." {
            parts.pop();
        } else if segment != "." {
            parts.push(segment);
        }
    }
    let root = if is_absolute(base) { root_of(base) } else { "" };
    join_with(separator_of(base), root, &parts)
}

/// The first component of `path` below `root` that is a symbolic link.
async fn first_symlink_below(
    path: &str,
    repository_root: &str,
    is_symlink: Callback,
) -> Option<String> {
    let root_parts = segments_of(repository_root);
    let parts = segments_of(path);
    let inside = root_parts
        .iter()
        .enumerate()
        .all(|(index, part)| parts.get(index) == Some(part));
    if !inside {
        return None;
    }
    let separator = separator_of(path);
    let root = root_of(path);
    for length in (root_parts.len() + 1)..=parts.len() {
        let prefix = join_with(separator, root, parts.get(..length)?);
        if is_symlink.call::<bool>(vec![prefix.as_str().into()]).await {
            return parts.get(length - 1).map(|part| (*part).to_owned());
        }
    }
    None
}

/// The candidate text of the hooks' answers. The merge has already picked the last answer in handler
/// order, an HTTP hook's `worktreePath` and a command hook's last line alike (Judgment J7.143), into
/// `effects.worktreePath`; an answer built without the merge falls back to the last command line.
#[must_use]
pub fn worktree_candidate(answer: &WorktreeAnswer) -> Option<String> {
    if let Some(merged) = &answer.effects.worktree_path
        && !trim(merged).is_empty()
    {
        return Some(trim(merged).to_owned());
    }
    answer
        .stdout
        .iter()
        .map(|stdout| last_path_line(stdout))
        .rfind(Option::is_some)
        .flatten()
}

/// The worktree path a `WorktreeCreate` outcome asks the session to use, or why creation fails.
///
/// Judgment (spec/judgments.md): the docs name the refusals and say the error names the rejected
/// component, not the words.
pub async fn resolve_worktree_path(
    answer: &WorktreeAnswer,
    location: &WorktreeLocation,
) -> WorktreePath {
    let failure = |failure: String| WorktreePath::Failure { failure };
    if !answer.proceed {
        return failure("The WorktreeCreate hook failed, so no worktree was created.".to_owned());
    }
    let Some(given) = worktree_candidate(answer) else {
        return failure("The WorktreeCreate hook returned no worktree path.".to_owned());
    };
    if is_absolute(&given) {
        let dotted = segments_of(&given)
            .into_iter()
            .find(|segment| *segment == "." || *segment == "..");
        if let Some(dotted) = dotted {
            return failure(format!(
                "The worktree path \"{given}\" contains a \"{dotted}\" segment; return a normalized path."
            ));
        }
    }
    let path = if is_absolute(&given) {
        given
    } else {
        resolve_relative(&location.ran_in, &given)
    };
    if let (Some(repository_root), Some(is_symlink)) =
        (&location.repository_root, location.is_symlink)
    {
        let link = first_symlink_below(&path, repository_root, is_symlink).await;
        if let Some(link) = link {
            return failure(format!(
                "The worktree path \"{path}\" passes through the symlink \"{link}\" below the repository root."
            ));
        }
    }
    WorktreePath::Path { path }
}

crate::registry::exports! {
    "hooks/output/worktree-path" {
        "stripAnsi" => ok |text: String| strip_ansi(&text);
        "lastPathLine" => ok |stdout: String| last_path_line(&stdout);
        "resolveWorktreePath" => async ok |answer: WorktreeAnswer, location: LocationGiven|
            resolve_worktree_path(&answer, &location.read().await).await;
    }
}
