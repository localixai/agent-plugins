//! Ported from `src/rules/tools/inputs/roles.ts`; only the core uses it now, so it has no wrapper.

use std::sync::LazyLock;

use indexmap::IndexMap;

use super::fields::{FieldRole, FieldSpec, Fields, TOOL_INPUT_FIELDS};

static FIELDS_OF_TOOL: LazyLock<IndexMap<&'static str, &'static Fields>> =
    LazyLock::new(|| TOOL_INPUT_FIELDS.entries().into_iter().collect());

/// The field a tool's table documents, or `None` for any other (a tool or a field without a table).
fn input_field_of(tool_name: &str, field: &str) -> Option<&'static FieldSpec> {
    FIELDS_OF_TOOL
        .get(tool_name)
        .and_then(|fields| fields.get(field))
}

/// permissions.md ("Match by input parameter"): "You can't match a tool's primary content field this
/// way ... Claude Code ignores it and emits a startup warning." The only role a parameter rule
/// ignores; the others are read as the parameter they are (a structured field never matches).
const fn ignored_in_parameter_rule(role: FieldRole) -> bool {
    match role {
        FieldRole::Primary => true,
        FieldRole::Subject | FieldRole::Param | FieldRole::Structured => false,
    }
}

/// Whether a parameter rule on `field` of `tool_name` is ignored: the field is the tool's primary one.
#[must_use]
pub fn is_ignored_in_parameter_rule(tool_name: &str, field: &str) -> bool {
    input_field_of(tool_name, field).is_some_and(|spec| ignored_in_parameter_rule(spec.role))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_primary_field_of_a_table_is_ignored() {
        assert!(is_ignored_in_parameter_rule("Bash", "command"));
        assert!(is_ignored_in_parameter_rule("Glob", "path"));
        assert!(!is_ignored_in_parameter_rule("Agent", "subagent_type"));
        assert!(!is_ignored_in_parameter_rule("Bash", "constructor"));
        assert!(!is_ignored_in_parameter_rule(
            "NotebookEdit",
            "notebook_path"
        ));
    }
}
