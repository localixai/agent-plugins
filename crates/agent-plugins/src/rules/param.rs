//! Port of `src/rules/param.ts`, switched.

use std::sync::LazyLock;

use regex::Regex;

use super::specifier_formats::SpecifierSyntax;
use super::tool_pattern::wildcard_reg_exp;
use super::tools::inputs::roles::is_ignored_in_parameter_rule;
use super::types::{PermissionBehavior, PermissionRule, PreparedCall, Specifier};
use crate::js::{self, JsValue};

/// The primary content field of each tool, as permissions.md ("Match by input parameter") lists
/// them: a parameter rule on one "would be bypassable by a compound command, so Claude Code ignores
/// it and emits a startup warning". The fields of the tools hooks.md tabulates are the `primary`
/// role of `TOOL_INPUT_FIELDS` (`tools/inputs`), each checked against the SDK's input type; the one
/// tool of the list without a table is `NotebookEdit`, below. `url` is `WebFetch`'s primary field,
/// and a `url` parameter of another tool stays an ordinary parameter.
const PRIMARY_FIELD_WITHOUT_TABLE: [(&str, &str); 1] = [("NotebookEdit", "notebook_path")];

/// Whether `field` is the primary content field of `tool_name`.
#[must_use]
pub fn is_primary_field(tool_name: &str, field: &str) -> bool {
    is_ignored_in_parameter_rule(tool_name, field)
        || PRIMARY_FIELD_WITHOUT_TABLE
            .iter()
            .any(|(tool, primary)| *tool == tool_name && *primary == field)
}

/// A parameter name: an identifier of two letters or more (`model`, `run_in_background`), or a flag
/// such as Grep's `-i`. A single letter without the dash is not one, so a Windows drive in
/// `Read(C:\Users)` stays a path (spec/judgments.md).
static PARAMETER_NAME: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^(?:-[A-Za-z][A-Za-z0-9_]*|[A-Za-z_][A-Za-z0-9_]+)$").ok());

/// `name:value` with "whitespace around the colon ignored", or `None` when the text does not
/// read as one for a tool that reads its specifier with `syntax`.
#[must_use]
pub fn read_param(content: &str, syntax: &SpecifierSyntax) -> Option<Specifier> {
    let (before, after) = content.split_once(':')?;
    let name = js::trim(before);
    let value = js::trim(after);
    if !PARAMETER_NAME
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(name))
    {
        return None;
    }
    if !syntax.allows_parameter(name) {
        return None;
    }
    if syntax.colon_star_suffix && value == "*" {
        return None;
    }
    Some(Specifier::Param {
        name: name.to_owned(),
        value: value.to_owned(),
    })
}

/// A parameter value as the text a rule compares with: scalars only.
fn scalar_text(value: &JsValue) -> Option<String> {
    match value {
        JsValue::String(text) => Some(text.clone()),
        JsValue::Number(number) => Some(js::number_to_string(*number)),
        JsValue::Bool(flag) => Some(flag.to_string()),
        JsValue::Undefined | JsValue::Null | JsValue::Array(_) | JsValue::Object(_) => None,
    }
}

/// Whether `text` is matched by the `*` pattern `value`.
fn wildcard_matches(value: &str, text: &str) -> bool {
    wildcard_reg_exp(value, None).is_ok_and(|mut pattern| pattern.test(text))
}

/// Whether a call sets the parameter to a value the rule's value matches: a direct field of the
/// input, a scalar, present, and equal to the rule's value where `*` stands for any sequence of
/// characters. The literal input is compared, before any normalisation (permissions.md, "Match by
/// input parameter"); a parameter the call omits never matches.
fn matches_param(name: &str, value: &str, input: &JsValue) -> bool {
    let Some(fields) = input.as_object() else {
        return false;
    };
    let Some(field) = fields.get(name) else {
        return false;
    };
    scalar_text(field).is_some_and(|text| wildcard_matches(value, &text))
}

/// The one parameter of the `Skill` tool: the name Claude calls the skill by.
const SKILL_PARAMETER: &str = "skill";

/// Only a deny rule matches a skill under every name it has: "A `Skill(skill:<name>)` deny rule
/// instead matches the skill under any of its names, such as its alias or display name"
/// (permissions.md, "Match by input parameter"; skills.md, "Restrict Claude's skill access").
const fn matches_every_skill_name(behavior: PermissionBehavior) -> bool {
    match behavior {
        PermissionBehavior::Allow | PermissionBehavior::Ask => false,
        PermissionBehavior::Deny => true,
    }
}

/// Whether a parameter rule covers a call: the literal input field (`matchesParam`) and, for a
/// `Skill(skill:<name>)` deny rule, any name the host reports for the called skill.
#[must_use]
pub fn matches_call_param(
    rule: &PermissionRule,
    specifier: &Specifier,
    call: &PreparedCall,
) -> bool {
    let Specifier::Param { name, value } = specifier else {
        return false;
    };
    if matches_param(name, value, &call.call.input) {
        return true;
    }
    if !matches_every_skill_name(rule.behavior) {
        return false;
    }
    if call.call.tool_name != "Skill" || name != SKILL_PARAMETER {
        return false;
    }
    let Ok(mut pattern) = wildcard_reg_exp(value, None) else {
        return false;
    };
    call.skill_names
        .iter()
        .flatten()
        .any(|skill| pattern.test(skill))
}

crate::registry::exports! {
    "rules/param" {
        "matchesCallParam" => ok |rule: PermissionRule, specifier: Specifier, call: PreparedCall|
            matches_call_param(&rule, &specifier, &call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::specifier_formats::specifier_syntax;
    use crate::rules::tools::rule_formats::RuleFormatId;

    #[test]
    fn a_parameter_is_a_name_of_two_letters_or_a_flag() {
        let syntax = specifier_syntax(RuleFormatId::Bare);
        let read = |text: &str| read_param(text, &syntax);
        assert_eq!(
            read(" model : opus "),
            Some(Specifier::Param {
                name: "model".to_owned(),
                value: "opus".to_owned()
            })
        );
        assert!(read("-i:true").is_some());
        assert_eq!(read(r"C:\Users"), None);
        assert_eq!(read("no colon"), None);
        assert!(is_primary_field("NotebookEdit", "notebook_path"));
        assert!(is_primary_field("WebFetch", "url"));
        assert!(!is_primary_field("Agent", "url"));
    }

    #[test]
    fn a_scalar_is_compared_as_its_text() {
        let input = |value: JsValue| {
            let mut fields = js::JsObject::new();
            fields.insert("timeout".to_owned(), value);
            JsValue::from(fields)
        };
        assert!(matches_param("timeout", "5*", &input(JsValue::from(50.0))));
        assert!(matches_param(
            "timeout",
            "true",
            &input(JsValue::Bool(true))
        ));
        assert!(!matches_param("timeout", "*", &input(JsValue::Null)));
        assert!(!matches_param("other", "*", &input(JsValue::from(1.0))));
    }
}
