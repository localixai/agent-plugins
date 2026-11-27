//! Port of `src/format/memory-rule.ts`, switched.
//!
//! A memory rule file, `.claude/rules/**/*.md` (memory.md, "Organize rules with `.claude/rules/`"):
//! "`paths` is the only field Claude Code reads from a rule; any other field is ignored without an
//! error. Claude Code removes the frontmatter before loading the rule into context" ("Rule frontmatter
//! reference"). `paths` "Accepts a YAML list or a comma-separated string" and takes brace expansion,
//! "the same glob format" a skill's `paths` uses, so it is read by the skill reader's `paths` field.
//! Finding the files (recursive, symlinks, `claudeMdExcludes`, `--setting-sources`) is the host's.

use serde::{Deserialize, Serialize};

use super::frontmatter::parse_frontmatter;
use super::issues::{PluginIssue, warning};
use super::skill_field_readers::FieldOutcome;
use super::skill_fields::field_reader;

/// `user`: `~/.claude/rules/`; `project`: a `.claude/rules/` folder ("Path-specific rules").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryRuleScope {
    /// The user's rules.
    User,
    /// A project's rules.
    Project,
}

/// One rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRule {
    /// The file as the host names it; a project rule's path holds its `.claude/rules/` folder.
    pub path: String,
    /// Where it was found.
    pub scope: MemoryRuleScope,
    /// Empty: the rule "loaded at launch"; else it loads when Claude works with a matching file.
    pub paths: Vec<String>,
    /// The text after the frontmatter: what loads into context.
    pub body: String,
}

/// One rule and what was wrong with its file.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemoryRuleRead {
    /// The rule.
    pub rule: MemoryRule,
    /// Warnings.
    pub issues: Vec<PluginIssue>,
}

/// One rule file. "If the YAML between the markers doesn't parse, Claude Code ignores the frontmatter
/// and loads the rule as if it had no `paths`": a warning, and the rule with no `paths`.
#[must_use]
pub fn read_memory_rule(path: &str, text: &str, scope: MemoryRuleScope) -> MemoryRuleRead {
    let parsed = parse_frontmatter(text);
    let read = parsed
        .fields
        .get("paths")
        .map_or_else(FieldOutcome::default, |value| {
            field_reader("paths", value, "paths")
        });
    let mut issues = Vec::new();
    if let Some(problem) = &parsed.problem {
        let place = parsed
            .line
            .map_or_else(String::new, |line| format!(" (line {line})"));
        let message = format!("frontmatter {problem}{place}; the rule loads with no paths");
        issues.push(warning(path, &message));
    }
    for found in &read.problems {
        issues.push(warning(path, &format!("{} {}", found.key, found.reason)));
    }
    let paths = read
        .values
        .get("paths")
        .and_then(|paths| paths.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    MemoryRuleRead {
        rule: MemoryRule {
            path: path.to_owned(),
            scope,
            paths,
            body: parsed.body,
        },
        issues,
    }
}

crate::registry::exports! {
    "format/memory-rule" {
        "readMemoryRule" => ok |path: String, text: String, scope: MemoryRuleScope| read_memory_rule(&path, &text, scope);
    }
}
