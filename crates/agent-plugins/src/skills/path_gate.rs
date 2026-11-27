//! Port of `src/skills/path-gate.ts`, switched.
//!
//! Whether a skill's `paths` activate it for a file (skills.md, frontmatter reference, `paths`):
//! "Patterns are relative to Claude's current working directory, and files outside that directory
//! don't match. Uses the same glob format as path-specific rules" (memory.md, "Path-specific
//! rules"): the gitignore syntax of the path rules, plus brace expansion.
//!
//! The compiled lists are kept by their text (`COMPILED`), where the TypeScript keeps them by the
//! list's identity: the answer depends on the text alone. Brace expansion nested past
//! [`MAX_NESTING`] groups is the package's own error for its stack running out (docs/RUST.md,
//! "Where the crate answers differently"), where Bun's stack goes deeper.

use std::sync::LazyLock;

use serde::Deserialize;

use crate::format::memory_rule::MemoryRuleScope;
use crate::format::utf8::utf8_length;
use crate::js::json_parse::json_parse;
use crate::js::{JsValue, json_stringify};
use crate::rules::memo::{LIMIT, Memo, memoize};
use crate::rules::path::glob::{
    GlobEntry, Globs, compile_globs, gitignore_matches, is_usable_pattern,
};
use crate::rules::path::posix::{normalize_absolute, relative_within, resolve_path, to_posix_path};
use crate::rules::types::Platform;

/// How the gate reads paths.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PathGateOptions {
    /// The host's path flavour; Windows paths are matched in POSIX form (default `posix`).
    #[serde(default)]
    pub platform: Option<Platform>,
}

/// `Pick<InvocableFrontmatter, 'paths'>`.
#[derive(Clone, Debug, Deserialize)]
pub struct SkillPaths {
    /// The skill's `paths`.
    pub paths: Vec<String>,
}

/// `Pick<MemoryRule, 'path' | 'scope'>`, with `paths` for `ruleActivatesFor`.
#[derive(Clone, Debug, Deserialize)]
pub struct RulePaths {
    /// The rule file.
    pub path: String,
    /// Whose rule it is.
    pub scope: MemoryRuleScope,
    /// The rule's `paths`; `ruleBase` does not read them.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// What Bun throws when brace groups nest deeper than its stack.
const TOO_DEEP: &str = "Maximum call stack size exceeded.";

/// Brace groups one pattern may hold before the expansion gives up as the package's stack would.
pub const MAX_NESTING: usize = 128;

/// The index of the `}` closing the `{` at `open`, or `None` when it never closes.
fn closing_brace(pattern: &str, open: usize) -> Option<usize> {
    let mut depth = 0_i64;
    for (at, byte) in pattern.bytes().enumerate().skip(open) {
        if byte == b'{' {
            depth += 1;
        }
        if byte == b'}' {
            depth -= 1;
        }
        if depth == 0 {
            return Some(at);
        }
    }
    None
}

/// The comma-separated alternatives of a brace body, splitting only at depth 0.
fn alternatives(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i64;
    let mut start = 0;
    for (at, byte) in body.bytes().enumerate() {
        if byte == b'{' {
            depth += 1;
        }
        if byte == b'}' {
            depth -= 1;
        }
        if byte == b',' && depth == 0 {
            parts.push(body.get(start..at).unwrap_or_default().to_owned());
            start = at + 1;
        }
    }
    parts.push(body.get(start..).unwrap_or_default().to_owned());
    parts
}

/// "A rule's whole `paths` list shares one budget of 1,000 expanded patterns and 4 MiB, and patterns
/// without braces don't count against it" (memory.md, "Path-specific rules").
const MAX_EXPANDED_PATTERNS: usize = 1000;
const MAX_EXPANDED_BYTES: usize = 4 * 1024 * 1024;

/// `src/**\/*.{ts,tsx}` to `src/**\/*.ts` and `src/**\/*.tsx` ("use brace expansion to match multiple
/// extensions in one pattern", memory.md), or `None` when it would give more than `limit`
/// patterns (it stops at the first one over, so the cost is bounded by the limit). Judgment
/// (spec/judgments.md): a group with no comma, or one that never closes, is literal text, as in a shell.
fn expand_braces(pattern: &str, limit: usize, depth: usize) -> Result<Option<Vec<String>>, String> {
    if depth > MAX_NESTING {
        return Err(TOO_DEEP.to_owned());
    }
    let open = pattern.find('{');
    let close = open.and_then(|open| closing_brace(pattern, open));
    let (Some(open), Some(close)) = (open, close) else {
        return Ok((limit >= 1).then(|| vec![pattern.to_owned()]));
    };
    let options = alternatives(pattern.get(open + 1..close).unwrap_or_default());
    let head = pattern.get(..open).unwrap_or_default();
    let tail = pattern.get(close + 1..).unwrap_or_default();
    if options.len() < 2 {
        let rest = expand_braces(tail, limit, depth + 1)?;
        let only = options.first().map_or("", String::as_str);
        return Ok(rest.map(|rest| {
            rest.iter()
                .map(|text| format!("{head}{{{only}}}{text}"))
                .collect()
        }));
    }
    let mut expanded: Vec<String> = Vec::new();
    for option in &options {
        let left = limit.saturating_sub(expanded.len());
        let Some(part) = expand_braces(&format!("{head}{option}{tail}"), left, depth + 1)? else {
            return Ok(None);
        };
        expanded.extend(part);
    }
    Ok(Some(expanded))
}

fn byte_length(patterns: &[String]) -> usize {
    patterns.iter().map(|pattern| utf8_length(pattern)).sum()
}

/// The patterns of a `paths` list with their braces expanded, within the list's shared budget. A
/// pattern that would go over it is kept as written, and its literal braces match no files.
fn expand_paths(patterns: &[String]) -> Result<Vec<String>, String> {
    let mut patterns_left = MAX_EXPANDED_PATTERNS;
    let mut bytes_left = MAX_EXPANDED_BYTES;
    let mut all = Vec::new();
    for pattern in patterns {
        if !pattern.contains('{') {
            all.push(pattern.clone());
            continue;
        }
        let expanded = expand_braces(pattern, patterns_left, 0)?;
        match expanded {
            Some(expanded) if byte_length(&expanded) <= bytes_left => {
                patterns_left -= expanded.len().min(patterns_left);
                bytes_left -= byte_length(&expanded);
                all.extend(expanded);
            }
            _ => all.push(pattern.clone()),
        }
    }
    Ok(all)
}

/// Whether the glob engine can compile the pattern at all: a pattern too large for a regular expression
/// is invalid, which "matches nothing" while "the rule's other patterns keep working" (memory.md).
fn compiles(pattern: &str, platform: Platform) -> bool {
    let entry = GlobEntry {
        pattern: pattern.to_owned(),
        negated: false,
    };
    gitignore_matches(&compile_globs(&[entry], platform), "probe", false).is_ok()
}

/// The matcher of a list given as its JSON text (the memo's key).
fn matcher(key: &str, platform: Platform) -> Result<Globs, String> {
    let paths: Vec<String> = match json_parse(key) {
        Ok(JsValue::Array(items)) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    };
    let entries: Vec<GlobEntry> = expand_paths(&paths)?
        .into_iter()
        .filter(|pattern| is_usable_pattern(pattern))
        .filter(|pattern| compiles(pattern, platform))
        .map(|pattern| GlobEntry {
            pattern,
            negated: false,
        })
        .collect();
    Ok(compile_globs(&entries, platform))
}

/// A `paths` list compiled once per platform: a skill's list is the same for its whole life.
static COMPILED_POSIX: LazyLock<Memo<Result<Globs, String>>> =
    LazyLock::new(|| memoize(|key| matcher(key, Platform::Posix), LIMIT));
static COMPILED_WINDOWS: LazyLock<Memo<Result<Globs, String>>> =
    LazyLock::new(|| memoize(|key| matcher(key, Platform::Windows), LIMIT));

fn matcher_of(paths: &[String], platform: Platform) -> Result<Globs, String> {
    let values = paths.iter().cloned().map(JsValue::String).collect();
    let key = json_stringify(&JsValue::Array(values)).unwrap_or_default();
    match platform {
        Platform::Posix => COMPILED_POSIX.get(&key),
        Platform::Windows => COMPILED_WINDOWS.get(&key),
    }
}

/// Whether a `paths` list matches `filePath`, its patterns relative to `base`: an empty list matches
/// every file; a file outside `base` (a sibling worktree, a parent) never matches. The one matcher of
/// a skill's and a memory rule's `paths`.
fn paths_match(
    paths: &[String],
    file_path: &str,
    base: &str,
    platform: Platform,
) -> Result<bool, String> {
    if paths.is_empty() {
        return Ok(true);
    }
    let root = normalize_absolute(&to_posix_path(base, platform));
    let file = resolve_path(&root, &to_posix_path(file_path, platform));
    let relative = relative_within(&root, &file, None);
    let Some(relative) = relative.filter(|relative| !relative.is_empty()) else {
        return Ok(false);
    };
    gitignore_matches(&matcher_of(paths, platform)?, &relative, false)
}

/// Whether the skill is active for `filePath` when the session's directory is `cwd`. A skill with no
/// `paths` is always active; a file outside `cwd` (a sibling worktree, a parent) never matches.
pub fn skill_activates_for(
    skill: &SkillPaths,
    file_path: &str,
    cwd: &str,
    options: &PathGateOptions,
) -> Result<bool, String> {
    paths_match(
        &skill.paths,
        file_path,
        cwd,
        options.platform.unwrap_or(Platform::Posix),
    )
}

const RULES_FOLDER: &str = "/.claude/rules/";

/// The folder a rule's patterns are relative to (memory.md, "Path-specific rules"): for a project
/// rule "the directory that contains the rule's `.claude` folder" (the last `.claude/rules/` in its
/// path, so a nested folder's rule is relative to its own subdirectory); for a user rule, and a
/// project rule whose path names no such folder, the session's primary working directory `cwd`.
#[must_use]
pub fn rule_base(rule: &RulePaths, cwd: &str, options: &PathGateOptions) -> String {
    let path = to_posix_path(&rule.path, options.platform.unwrap_or(Platform::Posix));
    match path.rfind(RULES_FOLDER) {
        Some(at) if rule.scope == MemoryRuleScope::Project => {
            let folder = path.get(..at).unwrap_or_default();
            if folder.is_empty() { "/" } else { folder }.to_owned()
        }
        _ => cwd.to_owned(),
    }
}

/// Whether a memory rule loads for `filePath`, which Claude read, wrote or edited: "A path-scoped
/// rule loads when Claude uses the Read, Write, or Edit tool on a matching file". A rule with no
/// `paths` is "loaded at launch", so it matches every file.
pub fn rule_activates_for(
    rule: &RulePaths,
    file_path: &str,
    cwd: &str,
    options: &PathGateOptions,
) -> Result<bool, String> {
    let platform = options.platform.unwrap_or(Platform::Posix);
    paths_match(
        &rule.paths,
        file_path,
        &rule_base(rule, cwd, options),
        platform,
    )
}

crate::registry::exports! {
    "skills/path-gate" {
        "skillActivatesFor" => fallible |skill: SkillPaths, file_path: String, cwd: String, options: Option<PathGateOptions>| skill_activates_for(&skill, &file_path, &cwd, &options.unwrap_or_default());
        "ruleBase" => ok |rule: RulePaths, cwd: String, options: Option<PathGateOptions>| rule_base(&rule, &cwd, &options.unwrap_or_default());
        "ruleActivatesFor" => fallible |rule: RulePaths, file_path: String, cwd: String, options: Option<PathGateOptions>| rule_activates_for(&rule, &file_path, &cwd, &options.unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_expand_within_the_budget_and_too_deep_is_the_stack_error() {
        let paths = vec!["src/*.{ts,tsx}".to_owned(), "a{b}c".to_owned()];
        assert_eq!(
            expand_paths(&paths),
            Ok(vec![
                "src/*.ts".to_owned(),
                "src/*.tsx".to_owned(),
                "a{b}c".to_owned()
            ])
        );
        let deep = "{x}".repeat(MAX_NESTING + 2);
        assert_eq!(expand_paths(&[deep]), Err(TOO_DEEP.to_owned()));
    }
}
