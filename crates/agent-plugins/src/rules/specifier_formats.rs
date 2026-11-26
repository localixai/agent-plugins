//! Port of `src/rules/specifier-formats.ts`, switched.
//!
//! The format of a tool's rule specifier, and what each format changes about reading one before
//! any matching (permissions.md, "Tool-specific permission rules"). The SDK's input types are the
//! field lists; the TypeScript's drift guards check them at compile time.

use indexmap::IndexMap;
use serde::Serialize;

use crate::registry::Set;
use crate::rules::tools::catalog::catalog_entry;
use crate::rules::tools::rule_formats::{RULE_FORMATS, RuleFormatId};

/// The tools the SDK no longer lists but a rule may still name: the legacy `MultiEdit` edits files
/// like `Edit` (spec/judgments.md); it is still not a known tool, so its rules draw `matches no
/// tool` (J13.34).
const LEGACY_FORMAT_OF_TOOL: [(&str, RuleFormatId); 1] = [("MultiEdit", RuleFormatId::EditPath)];

/// The format of a tool's rule specifier: its catalog entry (`tools/catalog.ts`); any other tool
/// takes only its bare name.
#[must_use]
pub fn rule_format_of(tool_name: &str) -> RuleFormatId {
    catalog_entry(tool_name)
        .map(|entry| entry.rule_format)
        .or_else(|| {
            LEGACY_FORMAT_OF_TOOL
                .iter()
                .find(|(name, _)| *name == tool_name)
                .map(|(_, format)| *format)
        })
        .unwrap_or(RuleFormatId::Bare)
}

/// The input fields of the command tools: a `name:value` specifier of a command rule is a
/// parameter rule only for one of these, because `git:* push` or `npm:run` is command text and the
/// docs do not say otherwise (spec/judgments.md). Bash and Monitor share a rule format; their
/// fields are the SDK's `BashInput` and `MonitorInput`, and the guards below fail the typecheck
/// when either drifts. The SDK has no PowerShell input type, so PowerShell reads Bash's fields
/// (judgment).
const BASH_INPUT_FIELDS: [&str; 5] = [
    "command",
    "timeout",
    "description",
    "run_in_background",
    "dangerouslyDisableSandbox",
];

const MONITOR_INPUT_FIELDS: [&str; 4] = ["description", "timeout_ms", "command", "ws"];

/// The input fields of `Agent`: a `name:value` specifier of an Agent rule is a parameter rule only
/// for one of these, so a plugin-qualified subagent (`Agent(acme:reviewer)`) stays a name. The
/// SDK's `AgentInput` is the list, and the guard fails the typecheck when it drifts.
const AGENT_INPUT_FIELDS: [&str; 10] = [
    "description",
    "prompt",
    "subagent_type",
    "model",
    "effort",
    "run_in_background",
    "name",
    "team_name",
    "mode",
    "isolation",
];

/// The one parameter of `Skill` the docs name (permissions.md, "Match by input parameter"):
/// without it `Skill(deploy:deploy)`, the plugin form of a synced skill (skills.md), would read as
/// a parameter rule on `deploy`. The SDK has no input type for the tool (spec/judgments.md).
const SKILL_INPUT_FIELDS: [&str; 1] = ["skill"];

/// A `Set` of names, in their first order, a repeat dropped.
fn name_set<'a>(names: impl IntoIterator<Item = &'a &'a str>) -> Set<String> {
    let mut members: Vec<String> = Vec::new();
    for name in names {
        if !members.iter().any(|member| member == name) {
            members.push((*name).to_owned());
        }
    }
    Set { members }
}

fn command_input_fields() -> Set<String> {
    name_set(BASH_INPUT_FIELDS.iter().chain(&MONITOR_INPUT_FIELDS))
}

fn powershell_input_fields() -> Set<String> {
    name_set(&BASH_INPUT_FIELDS)
}

/// What a format changes about reading a specifier, before any matching.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecifierSyntax {
    /// `domain:` opens a domain pattern (permissions.md, "WebFetch").
    pub domain_prefix: bool,
    /// `name:*` is a trailing wildcard, the `:*` suffix (permissions.md, "Wildcard patterns").
    pub colon_star_suffix: bool,
    /// `Tool(*)` is the bare tool name: documented for Bash ("`Bash(*)` is equivalent to `Bash`").
    pub star_is_bare: bool,
    /// The only names `name:value` may use as a parameter rule; `None` is any name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_names: Option<Set<String>>,
}

impl SpecifierSyntax {
    /// Whether `name:value` may use `name` as a parameter rule.
    #[must_use]
    pub fn allows_parameter(&self, name: &str) -> bool {
        self.parameter_names
            .as_ref()
            .is_none_or(|names| names.members.iter().any(|member| member == name))
    }
}

const PLAIN: SpecifierSyntax = SpecifierSyntax {
    domain_prefix: false,
    colon_star_suffix: false,
    star_is_bare: false,
    parameter_names: None,
};

fn command() -> SpecifierSyntax {
    SpecifierSyntax {
        colon_star_suffix: true,
        parameter_names: Some(command_input_fields()),
        ..PLAIN
    }
}

fn powershell() -> SpecifierSyntax {
    SpecifierSyntax {
        parameter_names: Some(powershell_input_fields()),
        ..command()
    }
}

/// `SPECIFIER_SYNTAX[format]`.
#[must_use]
pub fn specifier_syntax(format: RuleFormatId) -> SpecifierSyntax {
    match format {
        RuleFormatId::Command => SpecifierSyntax {
            star_is_bare: true,
            ..command()
        },
        RuleFormatId::PowershellCommand => powershell(),
        RuleFormatId::ReadPath | RuleFormatId::EditPath | RuleFormatId::Bare => PLAIN,
        RuleFormatId::SkillName => SpecifierSyntax {
            parameter_names: Some(name_set(&SKILL_INPUT_FIELDS)),
            ..PLAIN
        },
        RuleFormatId::SubagentType => SpecifierSyntax {
            parameter_names: Some(name_set(&AGENT_INPUT_FIELDS)),
            ..PLAIN
        },
        RuleFormatId::Domain => SpecifierSyntax {
            domain_prefix: true,
            ..PLAIN
        },
    }
}

/// `SPECIFIER_SYNTAX`: each format's syntax, in the order of `RULE_FORMATS`.
#[must_use]
pub fn specifier_syntax_table() -> IndexMap<RuleFormatId, SpecifierSyntax> {
    RULE_FORMATS
        .iter()
        .map(|format| (*format, specifier_syntax(*format)))
        .collect()
}

crate::registry::exports! {
    "rules/specifier-formats" {
        "ruleFormatOf" => ok |tool_name: String| rule_format_of(&tool_name);
        "SPECIFIER_SYNTAX" => ok specifier_syntax_table;
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleFormatId, rule_format_of, specifier_syntax};

    #[test]
    fn a_legacy_tool_keeps_its_format_and_any_other_is_bare() {
        assert_eq!(rule_format_of("MultiEdit"), RuleFormatId::EditPath);
        assert_eq!(rule_format_of("Nope"), RuleFormatId::Bare);
    }

    #[test]
    fn powershell_reads_bash_fields_only() {
        let syntax = specifier_syntax(RuleFormatId::PowershellCommand);
        assert!(syntax.allows_parameter("run_in_background"));
        assert!(!syntax.allows_parameter("ws"));
        assert!(!syntax.star_is_bare);
        assert!(specifier_syntax(RuleFormatId::Command).allows_parameter("ws"));
    }
}
