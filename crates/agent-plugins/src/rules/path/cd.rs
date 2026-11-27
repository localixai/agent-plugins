//! Port of `src/rules/path/cd.ts`, switched.
//!
//! `Cd(path)` rules (permissions.md, "Cd"): "Path patterns share the `//`, `~/`, and `/` anchors
//! from Read and Edit rules, but matching is anchored to the whole directory path rather than
//! gitignore-style. `*` matches exactly one path segment and `**` matches across segments. A
//! trailing `/**` also matches its named root."
//!
//! The expression is JavaScript's `RegExp` (`js::regexp`), so one JavaScriptCore refuses (a
//! pattern too large) throws as it did, and the matcher is fallible.

use serde::{Deserialize, Serialize};

use crate::format::wildcard::{escape_reg_exp, wildcard_source};
use crate::js::regexp::RegExp;
use crate::rules::path::anchor::{anchor_pattern, read_path_specifier};
use crate::rules::path::posix::{HostPaths, absolute_path};
use crate::rules::restricts::restricts;
use crate::rules::types::{MatchContext, PermissionRule, Specifier};

/// One path segment of a pattern as regular-expression source; `**` is handled between segments.
fn segment_source(segment: &str) -> String {
    wildcard_source(segment, Some("[^/]*"))
}

/// A directory pattern as a regular expression over the whole POSIX path (root and pattern
/// joined).
fn directory_expression(root: &str, pattern: &str) -> Result<RegExp, String> {
    let segments: Vec<&str> = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let base = if root == "/" {
        String::new()
    } else {
        escape_reg_exp(root)
    };
    let parts: Vec<String> = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let last = index + 1 == segments.len();
            if *segment != "**" {
                format!("/{}", segment_source(segment))
            } else if last {
                // `**` spans zero or more segments; at the end it also covers the named root.
                "(?:/.*)?".to_owned()
            } else {
                "(?:/[^/]+)*".to_owned()
            }
        })
        .collect();
    RegExp::new(&format!("^{base}{}$", parts.concat()), "").map_err(|error| error.to_string())
}

/// A directory a `/cd` moves to: where it resolves, and every spelling it was reached by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdTarget {
    /// Where the directory resolves.
    pub resolved: String,
    /// Every spelling it was reached by.
    pub spellings: Vec<String>,
}

/// Whether a `Cd(<pattern>)` rule covers the target: "Deny rules check every spelling of the
/// target, including each symlink hop it resolves through"; an allow rule needs "the resolved
/// target directory" to match. A bare `Cd` rule has no pattern and is the host's to read.
pub fn match_cd_rule(
    rule: &PermissionRule,
    target: &CdTarget,
    ctx: &MatchContext,
) -> Result<bool, String> {
    let Some(Specifier::Text { pattern }) = &rule.specifier else {
        return Ok(false);
    };
    let parsed = read_path_specifier(pattern);
    if parsed.negated {
        return Ok(false);
    }
    let anchored = anchor_pattern(&parsed, &rule.source, &ctx.paths);
    let pattern = anchored
        .pattern
        .strip_prefix('/')
        .unwrap_or(&anchored.pattern);
    let expression = directory_expression(&anchored.root, pattern)?;
    let candidates: Vec<&String> = if restricts(rule.behavior) {
        std::iter::once(&target.resolved)
            .chain(&target.spellings)
            .collect()
    } else {
        vec![&target.resolved]
    };
    let host = HostPaths::from(&ctx.paths);
    Ok(candidates
        .into_iter()
        .any(|path| expression.test_shared(&absolute_path(path, &host))))
}

crate::registry::exports! {
    "rules/path/cd" {
        "matchCdRule" => fallible |rule: PermissionRule, target: CdTarget, ctx: MatchContext| match_cd_rule(&rule, &target, &ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::directory_expression;

    #[test]
    fn a_trailing_globstar_also_matches_its_root() {
        let expression = directory_expression("/work", "src/**").unwrap();
        assert!(expression.test_shared("/work/src"));
        assert!(expression.test_shared("/work/src/a/b"));
        assert!(!expression.test_shared("/work/srcs"));
        let one = directory_expression("/", "*/x").unwrap();
        assert!(one.test_shared("/a/x"));
        assert!(!one.test_shared("/a/b/x"));
    }
}
