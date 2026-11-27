//! Port of `src/rules/path/glob.ts`, switched.
//!
//! The gitignore engine of the path rules: "Read and Edit rules both use gitignore pattern syntax"
//! (permissions.md, "How path patterns match"), here the core's port of the `ignore` package
//! (`crate::ignore`), over a path relative to the rule's anchor. Case: Windows paths are compared
//! without it, POSIX paths with it (a judgment, the docs are silent).
//!
//! Decision: what `compileGlobs` answers crosses the boundary as the platform and the lines it was
//! compiled from (`Globs`), not as the engine, which no value crossing can be; given back to
//! `gitignoreMatches`, it is the engine the memo keeps for those lines, so a list is compiled once
//! while the memo holds it, as in the TypeScript.

use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ignore::Ignore;
use crate::js::json_parse::json_parse;
use crate::js::{JsValue, json_stringify, trim};
use crate::rules::memo::{LIMIT, Memo, memoize};
use crate::rules::types::Platform;

/// Whether a platform's paths are compared with their case.
const fn case_sensitive(platform: Platform) -> bool {
    match platform {
        Platform::Posix => true,
        Platform::Windows => false,
    }
}

/// A pattern, and whether it is a `!` negation of what came before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobEntry {
    /// The pattern.
    pub pattern: String,
    /// Whether it is a `!` negation.
    pub negated: bool,
}

/// Whether gitignore would keep the pattern as a rule at all: a blank line and a `#` comment are
/// not rules. "A deny or ask rule whose path isn't usable as a gitignore pattern still guards that
/// exact path. An allow rule with an unusable pattern doesn't approve anything." (permissions.md)
#[must_use]
pub fn is_usable_pattern(pattern: &str) -> bool {
    let text = trim(pattern);
    !text.is_empty() && !text.starts_with('#') && text != "/"
}

/// The engine for one platform and pattern list, keyed by both; built once per list.
static COMPILED_POSIX: LazyLock<Memo<Arc<Ignore>>> = LazyLock::new(|| {
    memoize(
        |key| Arc::new(Ignore::new(!case_sensitive(Platform::Posix)).add(&lines_of(key))),
        LIMIT,
    )
});
static COMPILED_WINDOWS: LazyLock<Memo<Arc<Ignore>>> = LazyLock::new(|| {
    memoize(
        |key| Arc::new(Ignore::new(!case_sensitive(Platform::Windows)).add(&lines_of(key))),
        LIMIT,
    )
});

/// `COMPILED[platform]`.
fn compiled(platform: Platform) -> &'static Memo<Arc<Ignore>> {
    match platform {
        Platform::Posix => &COMPILED_POSIX,
        Platform::Windows => &COMPILED_WINDOWS,
    }
}

/// The pattern lines a key holds (`compile_globs` writes it as JSON, so a line may hold anything).
fn lines_of(key: &str) -> Vec<String> {
    match json_parse(key) {
        Ok(JsValue::Array(lines)) => lines
            .into_iter()
            .filter_map(|line| line.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// The compiled patterns of one platform and list (`Ignore` in the TypeScript).
#[derive(Clone, Debug)]
pub struct Globs {
    platform: Platform,
    lines: Vec<String>,
    engine: Arc<Ignore>,
}

impl Globs {
    /// The engine the memo keeps for `lines` on `platform`.
    fn of(platform: Platform, lines: Vec<String>) -> Self {
        let values = lines.iter().cloned().map(JsValue::String).collect();
        let key = json_stringify(&JsValue::Array(values)).unwrap_or_default();
        let engine = compiled(platform).get(&key);
        Self {
            platform,
            lines,
            engine,
        }
    }
}

/// How `Globs` crosses: the platform and the lines.
#[derive(Serialize, Deserialize)]
struct GlobsWire {
    platform: Platform,
    lines: Vec<String>,
}

impl Serialize for Globs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GlobsWire {
            platform: self.platform,
            lines: self.lines.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Globs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = GlobsWire::deserialize(deserializer)?;
        Ok(Self::of(wire.platform, wire.lines))
    }
}

/// The patterns compiled: a `!` entry is written as a negated line.
#[must_use]
pub fn compile_globs(entries: &[GlobEntry], platform: Platform) -> Globs {
    let lines = entries
        .iter()
        .map(|GlobEntry { pattern, negated }| {
            if *negated {
                format!("!{pattern}")
            } else {
                pattern.clone()
            }
        })
        .collect();
    Globs::of(platform, lines)
}

/// A name no pattern is written for: a search tool's `path` is a directory, which gitignore only
/// matches through the files below it (`docs/**` does not match `docs`), so a directory is also
/// tried as the parent of this name (a judgment, spec/judgments.md).
const DIRECTORY_PROBE: &str = ".claude-kit-directory-probe";

/// Whether the compiled patterns select `relative`. `''` and a parent climb (`..`, `../x`) never
/// reach the engine, which throws on them; a name that merely starts with two dots (`..hidden`) is
/// a file. The `Err` is what the engine throws (a pattern too large for a regular expression).
pub fn gitignore_matches(globs: &Globs, relative: &str, directory: bool) -> Result<bool, String> {
    if relative.is_empty() || relative == ".." || relative.starts_with("../") {
        return Ok(false);
    }
    Ok(globs.engine.ignores(relative)?
        || (directory
            && globs
                .engine
                .ignores(&format!("{relative}/{DIRECTORY_PROBE}"))?))
}

crate::registry::exports! {
    "rules/path/glob" {
        "isUsablePattern" => ok |pattern: String| is_usable_pattern(&pattern);
        "compileGlobs" => ok |entries: Vec<GlobEntry>, platform: Platform| compile_globs(&entries, platform);
        "gitignoreMatches" => fallible |globs: Globs, relative: String, directory: bool| gitignore_matches(&globs, &relative, directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pattern: &str, negated: bool) -> GlobEntry {
        GlobEntry {
            pattern: pattern.to_owned(),
            negated,
        }
    }

    #[test]
    fn a_directory_is_matched_through_a_name_below_it() {
        let globs = compile_globs(&[entry("docs/**", false)], Platform::Posix);
        assert_eq!(gitignore_matches(&globs, "docs", false), Ok(false));
        assert_eq!(gitignore_matches(&globs, "docs", true), Ok(true));
        assert_eq!(gitignore_matches(&globs, "../docs", true), Ok(false));
    }

    #[test]
    fn windows_paths_compare_without_case_and_a_negation_carves() {
        let entries = [entry("*.TS", false), entry("b.ts", true)];
        let globs = compile_globs(&entries, Platform::Windows);
        assert_eq!(gitignore_matches(&globs, "a.ts", false), Ok(true));
        assert_eq!(gitignore_matches(&globs, "b.ts", false), Ok(false));
        let posix = compile_globs(&entries, Platform::Posix);
        assert_eq!(gitignore_matches(&posix, "a.ts", false), Ok(false));
    }

    #[test]
    fn a_blank_line_a_comment_and_a_lone_slash_are_no_patterns() {
        assert!(is_usable_pattern("a"));
        assert!(!is_usable_pattern("  "));
        assert!(!is_usable_pattern(" #a"));
        assert!(!is_usable_pattern("/"));
        assert_eq!(lines_of("[\"a\",1,null]"), ["a"]);
    }
}
