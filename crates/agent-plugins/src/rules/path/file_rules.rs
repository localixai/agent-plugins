//! Port of `src/rules/path/file-rules.ts`, switched.
//!
//! A pattern the gitignore engine throws on (one too large for a regular expression) throws out of
//! the matchers here as it did in the TypeScript, so each that reaches the engine is fallible.

use crate::js::trim;
use crate::rules::path::anchor::{PathSpecifier, anchor_pattern, read_path_specifier};
use crate::rules::path::depth::depth_of;
use crate::rules::path::glob::{GlobEntry, compile_globs, gitignore_matches, is_usable_pattern};
use crate::rules::path::posix::{HostPaths, absolute_path, relative_within, resolve_path};
use crate::rules::path::tools::{
    coverage, path_text, pathless_calls, requested_path, searches_directory,
};
use crate::rules::restricts::restricts;
use crate::rules::types::{
    MatchContext, PathContext, PermissionBehavior, PermissionRule, PreparedCall, Specifier,
};

/// The text pattern of a path rule, or `undefined` for a bare or parameter rule.
#[must_use]
pub fn path_specifier_of(rule: &PermissionRule) -> Option<PathSpecifier> {
    match &rule.specifier {
        Some(Specifier::Text { pattern }) => Some(read_path_specifier(pattern)),
        _ => None,
    }
}

/// Every path a call is checked at: the requested one and the files it resolves to
/// (permissions.md, "Symlinks"). A deny or ask rule also reaches a path given by the real location
/// of a directory the rule was written through (`/private/etc/hosts` for `Read(//etc/**)`).
fn spellings(path: &str, paths: &PathContext) -> Vec<String> {
    let mut found = Vec::new();
    for (written, real) in paths
        .real_directories
        .iter()
        .flat_map(|map| map.entries.iter())
    {
        if let Some(rest) = relative_within(real, path, Some(paths.platform)) {
            found.push(resolve_path(written, &rest));
        }
    }
    found
}

fn call_paths(
    rule: &PermissionRule,
    call: &PreparedCall,
    paths: &PathContext,
) -> Option<Vec<String>> {
    let requested = requested_path(&call.call, paths)?;
    let host = HostPaths::from(paths);
    let resolved = call
        .resolved_paths
        .iter()
        .flatten()
        .map(|path| absolute_path(path, &host));
    let forms: Vec<String> = std::iter::once(requested).chain(resolved).collect();
    // A deny or ask rule also reaches the real location of a directory it was written through.
    if !restricts(rule.behavior) {
        return Some(forms);
    }
    let spelled: Vec<String> = forms
        .iter()
        .flat_map(|form| spellings(form, paths))
        .collect();
    Some(forms.into_iter().chain(spelled).collect())
}

/// Allow rules need every path to match, deny and ask rules any one (permissions.md, "Symlinks").
fn combine(
    behavior: PermissionBehavior,
    forms: &[String],
    test: impl Fn(&str) -> Result<bool, String>,
) -> Result<bool, String> {
    match behavior {
        PermissionBehavior::Allow => {
            for form in forms {
                if !test(form)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        PermissionBehavior::Deny | PermissionBehavior::Ask => {
            for form in forms {
                if test(form)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

/// Everything one rule is matched with, besides the path.
struct Scope<'a> {
    rule: &'a PermissionRule,
    specifier: PathSpecifier,
    paths: &'a PathContext,
    /// `!` rules from the same source listed after the rule (`negation.rs`).
    carve_outs: &'a [PathSpecifier],
    directory: bool,
    /// Whose depth rule widens a single-directory pattern: the context's, else the rule's own.
    depth_of: PermissionBehavior,
}

fn matches_path(scope: &Scope<'_>, path: &str) -> Result<bool, String> {
    let Scope {
        rule,
        specifier,
        paths,
        ..
    } = scope;
    let anchored = anchor_pattern(specifier, &rule.source, paths);
    let (root, pattern) = (&anchored.root, &anchored.pattern);
    if !is_usable_pattern(pattern) {
        // `/` alone is the anchor itself: the slash only anchors the text, it is not the filesystem
        // root. A deny or ask rule keeps its exact path; an allow rule approves nothing.
        let exact = resolve_path(root, pattern.strip_prefix('/').unwrap_or(pattern));
        return Ok(restricts(rule.behavior)
            && relative_within(&exact, path, Some(paths.platform)).as_deref() == Some(""));
    }
    let Some(relative) = relative_within(root, path, Some(paths.platform)) else {
        return Ok(false);
    };
    // A `../` pattern names two directories, so the one-directory depth rule does not apply to it.
    let deep = if anchored.under_cwd {
        depth_of(scope.depth_of, pattern)
    } else {
        pattern.clone()
    };
    let mut entries = vec![GlobEntry {
        pattern: deep,
        negated: false,
    }];
    if anchored.under_cwd {
        for carve in scope.carve_outs {
            if is_usable_pattern(&carve.pattern) {
                entries.push(GlobEntry {
                    pattern: carve.pattern.clone(),
                    negated: true,
                });
            }
        }
    }
    gitignore_matches(
        &compile_globs(&entries, paths.platform),
        &relative,
        scope.directory,
    )
}

/// Whether the path rule covers the path `call` acts on, with the `!` rules that carve paths out
/// of it (empty when the rule is matched alone). A `!` rule itself never covers a path.
pub fn match_path_rule(
    rule: &PermissionRule,
    call: &PreparedCall,
    ctx: &MatchContext,
    carve_outs: &[PathSpecifier],
) -> Result<bool, String> {
    let Some(specifier) = path_specifier_of(rule).filter(|specifier| !specifier.negated) else {
        return Ok(false);
    };
    let Some(forms) = call_paths(rule, call, &ctx.paths) else {
        return Ok(false);
    };
    let scope = Scope {
        rule,
        specifier,
        paths: &ctx.paths,
        carve_outs,
        directory: searches_directory(&call.call.tool_name),
        depth_of: ctx.depth_of.unwrap_or(rule.behavior),
    };
    combine(rule.behavior, &forms, |form| matches_path(&scope, form))
}

/// Tools whose path rules "are accepted but never match": "Claude Code checks file permissions
/// against `Edit(path)` and `Read(path)` rules. It accepts a rule with a path other than `*` for
/// one of the tools below but doesn't match a read, search, or edit against it" (permissions.md;
/// it lists the legacy MultiEdit with Write and NotebookEdit). `Tool(*)` is read as every use of
/// the tool, because the docs ask for the bare name for that. An `LSP(path)` rule is in the Read
/// format of tools-reference.md: a deny or ask rule on it covers every call and an allow rule
/// none, the safe side (spec/judgments.md).
const UNMATCHED_FORMS: [&str; 5] = ["Write", "Glob", "Grep", "NotebookEdit", "MultiEdit"];

/// The matcher of the `read-path` and `edit-path` rule formats.
pub fn match_file_rule(
    rule: &PermissionRule,
    call: &PreparedCall,
    ctx: &MatchContext,
) -> Result<bool, String> {
    let Some(Specifier::Text { pattern }) = &rule.specifier else {
        return Ok(false);
    };
    let tool_name = rule.value.tool_name.as_str();
    let table = coverage();
    let covered = table
        .entries
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, coverage)| coverage.of(rule.behavior));
    let Some(covered) = covered else {
        return Ok(if UNMATCHED_FORMS.contains(&tool_name) {
            trim(pattern) == "*"
        } else {
            restricts(rule.behavior)
        });
    };
    if !covered.contains(&call.call.tool_name.as_str()) {
        return Ok(false);
    }
    // An LSP call is matched at its `filePath`; one without a path is covered, the safe side
    // (J7.210).
    if pathless_calls()
        .members
        .contains(&call.call.tool_name.as_str())
        && path_text(&call.call).is_none()
    {
        return Ok(true);
    }
    match_path_rule(rule, call, ctx, &[])
}

/// Whether `rule` covers a call of a tool other than its own: an `Edit` rule reaches Write, a
/// `Read` rule Grep, and so on (`COVERAGE`). Only a text path rule on a named tool is routed.
#[must_use]
pub fn routes_to_call(rule: &PermissionRule, tool_name: &str) -> bool {
    if rule.tool_glob || !matches!(rule.specifier, Some(Specifier::Text { .. })) {
        return false;
    }
    coverage()
        .entries
        .iter()
        .find(|(name, _)| *name == rule.value.tool_name)
        .is_some_and(|(_, coverage)| coverage.of(rule.behavior).contains(&tool_name))
}

crate::registry::exports! {
    "rules/path/file-rules" {
        "routesToCall" => ok |rule: PermissionRule, tool_name: String| routes_to_call(&rule, &tool_name);
    }
}
