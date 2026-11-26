//! Port of `src/skills/scan/names.ts`, switched.
//!
//! Pure helpers over command names and host paths, for `resolveFolderSkills`.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::normalize::{NormalForm, normalize};
use crate::js::{Pattern, SPACE_CLASS, trim};

/// `/[/\\]+/`.
static SEPARATORS: Pattern = LazyLock::new(|| Regex::new(r"[/\\]+").ok());

/// The parts of a host path, whichever separator it uses.
#[must_use]
pub fn segments_of(path: &str) -> Vec<String> {
    let Some(separators) = SEPARATORS.as_ref() else {
        return Vec::new();
    };
    separators
        .split(path)
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_owned)
        .collect()
}

/// `/[\p{Cf}\p{Cc}]/gu`.
static INVISIBLE: Pattern = LazyLock::new(|| Regex::new(r"[\p{Cf}\p{Cc}]").ok());
/// `/\p{Pd}/gu`.
static DASH: Pattern = LazyLock::new(|| Regex::new(r"\p{Pd}").ok());
/// `/\s+/gu`.
static SPACES: Pattern = LazyLock::new(|| Regex::new(&format!("[{SPACE_CLASS}]+")).ok());

/// `text.replace(pattern, with)` for a global pattern.
fn replace_all(pattern: &Pattern, text: &str, with: &str) -> String {
    pattern.as_ref().map_or_else(
        || text.to_owned(),
        |regex| regex.replace_all(text, with).into_owned(),
    )
}

/// How synced skills' names are compared (skills.md, "When a synced skill name matches another
/// command"): "Claude Code ignores case, spacing, and invisible characters, and treats compatibility
/// forms such as fullwidth letters and dash variants as their plain equivalents". A look-alike letter
/// from another alphabet stays different.
#[must_use]
pub fn fold_name(name: &str) -> String {
    let normalized = normalize(name, NormalForm::Nfkc);
    let visible = replace_all(&INVISIBLE, &normalized, "");
    let dashes = replace_all(&DASH, &visible, "-");
    let spaced = replace_all(&SPACES, &dashes, " ");
    trim(&spaced).to_lowercase()
}

/// `root.every((part, index) => parts[index] === part)`.
fn starts_with(parts: &[String], root: &[String]) -> bool {
    root.iter()
        .enumerate()
        .all(|(index, part)| parts.get(index) == Some(part))
}

/// The directory `base` as a path below `cwd` (`apps/web`), `/`-separated: how a nested skill's
/// qualified name starts (skills.md, "How a skill gets its command name": "Subdirectory path relative
/// to the working directory"). A base that is not below `cwd` is its folder's name (judgment).
#[must_use]
pub fn relative_below(cwd: &str, base: &str) -> String {
    let root = segments_of(cwd);
    let parts = segments_of(base);
    let inside = starts_with(&parts, &root);
    let below = if inside {
        parts.get(root.len()..).unwrap_or_default()
    } else {
        parts
            .get(parts.len().saturating_sub(1)..)
            .unwrap_or_default()
    };
    below.join("/")
}

/// Whether `path` is inside `parent`, and not `parent` itself.
#[must_use]
pub fn is_strictly_below(parent: &str, path: &str) -> bool {
    let root = segments_of(parent);
    let parts = segments_of(path);
    parts.len() > root.len() && starts_with(&parts, &root)
}

crate::registry::exports! {
    "skills/scan/names" {
        "segmentsOf" => ok |path: String| segments_of(&path);
        "foldName" => ok |name: String| fold_name(&name);
        "relativeBelow" => ok |cwd: String, base: String| relative_below(&cwd, &base);
        "isStrictlyBelow" => ok |parent: String, path: String| is_strictly_below(&parent, &path);
    }
}
