//! Port of `src/rules/tools/rule-formats.ts`, switched.
//!
//! How a tool reads what follows its name inside the parentheses of a rule: the "Rule format" table
//! of tools-reference.md ("Configure tools with permission rules and hooks"). One id per format;
//! every part that differs per format (matching, syntax details) is a registry typed over
//! `RuleFormatId`, so a format added here fails the typecheck until each registry handles it. Which
//! tool uses which format is the catalog's `ruleFormat` (`catalog.ts`).

use serde::{Deserialize, Serialize};

/// Every rule format, in the table's order.
pub const RULE_FORMATS: [RuleFormatId; 8] = [
    RuleFormatId::Command,
    RuleFormatId::PowershellCommand,
    RuleFormatId::ReadPath,
    RuleFormatId::EditPath,
    RuleFormatId::SkillName,
    RuleFormatId::SubagentType,
    RuleFormatId::Domain,
    RuleFormatId::Bare,
];

/// One of `RULE_FORMATS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(missing_docs, reason = "each variant is its kebab-case id")]
pub enum RuleFormatId {
    Command,
    PowershellCommand,
    ReadPath,
    EditPath,
    SkillName,
    SubagentType,
    Domain,
    Bare,
}

crate::registry::exports! {
    "rules/tools/rule-formats" {
        "RULE_FORMATS" => ok || RULE_FORMATS;
    }
}
