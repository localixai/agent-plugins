//! Port of `src/rules/tool-pattern.ts`, switched.
//!
//! A rule's tool-name glob and the tool-name position of a rule. The package keeps the last 256
//! patterns it compiled (`rules/memo.ts`); the core compiles a pattern on each call, which answers
//! the same.

use super::target::mcp::{deny_covers_cowork_tool, mcp_tool_rule_matches};
use super::types::PermissionRule;
use crate::format::wildcard::wildcard_source;
use crate::js::regexp::{RegExp, SyntaxError};

/// A pattern and its flags compiled, from the key `<flags>\0<pattern>`.
fn compiled(key: &str) -> Result<RegExp, SyntaxError> {
    let (flags, pattern) = key.split_once('\0').unwrap_or(("", key));
    RegExp::new(&format!("^{}$", wildcard_source(pattern, None)), flags)
}

/// A `*` pattern as a regular expression that must match the whole text: `*` stands for any
/// sequence of characters, spaces and slashes included, and every other character is literal.
pub fn wildcard_reg_exp(pattern: &str, flags: Option<&str>) -> Result<RegExp, SyntaxError> {
    compiled(&format!("{}\0{pattern}", flags.unwrap_or_default()))
}

/// Whether an allow rule's tool-name glob names one server: permissions.md ("Tool name wildcards")
/// accepts one "only after a literal `mcp__<server>__` prefix", so `mcp__puppeteer__*` and
/// `mcp__github__get_*` do and `*`, `B*` and `mcp__*` do not. The server is the text between
/// `mcp__` and the next `__`, and it must hold no `*`.
#[must_use]
pub fn is_anchored_allow_glob(tool_name: &str) -> bool {
    let Some(star) = tool_name.find('*') else {
        return true;
    };
    let before = tool_name.get(..star).unwrap_or_default();
    RegExp::new("^mcp__.+__", "").is_ok_and(|mut prefix| prefix.test(before))
}

/// Whether the tool-name position of `rule` covers `toolName`: a glob matching it whole, else the
/// name itself, an `mcp__` server name covering its tools (`mcpToolRuleMatches`), or a deny rule that
/// names `Bash` or `WebFetch` whole covering the Cowork twin of that tool.
#[must_use]
pub fn tool_name_matches(rule: &PermissionRule, tool_name: &str) -> bool {
    if rule.tool_glob {
        return wildcard_reg_exp(&rule.value.tool_name, None)
            .is_ok_and(|mut glob| glob.test(tool_name));
    }
    mcp_tool_rule_matches(&rule.value.tool_name, tool_name, rule.behavior)
        || deny_covers_cowork_tool(rule, tool_name)
}

crate::registry::exports! {
    "rules/tool-pattern" {
        "wildcardRegExp" => fallible |pattern: String, flags: Option<String>| wildcard_reg_exp(&pattern, flags.as_deref()).map(|compiled| crate::registry::RegExp { source: compiled.source().to_owned(), flags: compiled.flags().to_owned() });
        "isAnchoredAllowGlob" => ok |tool_name: String| is_anchored_allow_glob(&tool_name);
        "toolNameMatches" => ok |rule: PermissionRule, tool_name: String| tool_name_matches(&rule, &tool_name);
    }
}

#[cfg(test)]
mod tests {
    use super::{is_anchored_allow_glob, wildcard_reg_exp};

    #[test]
    fn matches_a_glob_whole() {
        let mut glob = wildcard_reg_exp("mcp__github__*", None).unwrap();
        assert!(glob.test("mcp__github__get_issue"));
        assert!(!glob.test("x mcp__github__get_issue"));
        let mut folded = wildcard_reg_exp("Bash", Some("i")).unwrap();
        assert!(folded.test("bash"));
        assert!(is_anchored_allow_glob("mcp__github__get_*"));
        assert!(!is_anchored_allow_glob("mcp__*"));
        assert!(!is_anchored_allow_glob("mcp__a\nb__*"));
    }
}
