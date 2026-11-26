//! Port of `src/format/permission-rule-shape.ts`, switched.
//!
//! The shape of a permission rule's text, shared by the settings reader (a malformed rule is a
//! Settings Warning) and the rules parser (permissions.md, "Permission rule syntax"; errors.md,
//! "Malformed Tool(content) rule").

use serde::{Deserialize, Serialize};

use crate::js::trim;

/// The two parts of `Tool` or `Tool(content)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleParts {
    /// The tool's name.
    pub tool_name: String,
    /// What is between the parentheses, when the rule has them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// errors.md ("Malformed Tool(content) rule"), quoted.
pub const MALFORMED_RULE: &str = "Malformed Tool(content) rule. Rules take the form Tool or Tool(content) and must end at the closing \")\"; parentheses inside the content are literal";

/// errors.md: `Invalid permission rule "..." was skipped: <reason>`.
#[must_use]
pub fn invalid_rule_message(raw: &str, reason: &str) -> String {
    format!("Invalid permission rule \"{raw}\" was skipped: {reason}")
}

/// `Tool` or `Tool(content)`. Parentheses inside the content are literal, so the rule ends at its
/// last character, which must be the closing parenthesis (permissions.md, "Permission rule syntax");
/// `None` for text of no such shape.
#[must_use]
pub fn split_rule(raw: &str) -> Option<RuleParts> {
    let Some((before, after)) = raw.split_once('(') else {
        return if raw.is_empty() || raw.contains(')') {
            None
        } else {
            Some(RuleParts {
                tool_name: raw.to_owned(),
                content: None,
            })
        };
    };
    let tool_name = trim(before);
    if tool_name.is_empty() || tool_name.contains(')') {
        return None;
    }
    let content = after.strip_suffix(')')?;
    Some(RuleParts {
        tool_name: tool_name.to_owned(),
        content: Some(content.to_owned()),
    })
}

crate::registry::exports! {
    "format/permission-rule-shape" {
        "MALFORMED_RULE" => ok || MALFORMED_RULE;
        "invalidRuleMessage" => ok |raw: String, reason: String| invalid_rule_message(&raw, &reason);
        "splitRule" => ok |raw: String| split_rule(&raw);
    }
}
