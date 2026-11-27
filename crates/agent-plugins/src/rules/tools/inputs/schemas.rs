//! Ported from `src/rules/tools/inputs/schemas.ts`; only the core uses it now, so it has no wrapper.
//!
//! The zod object of each tool's input table.

use std::sync::LazyLock;

use indexmap::IndexMap;

use super::fields::{Fields, TOOL_INPUT_FIELDS};
use crate::zod::{self, Schema};

/// The zod object of a tool's table: its fields as documented, any other field passed through.
fn schema_of(fields: &Fields) -> Schema {
    zod::loose_object(
        fields
            .iter()
            .map(|(name, spec)| (name.clone(), spec.schema.clone()))
            .collect(),
    )
}

/// One schema per tool of hooks.md ("PreToolUse input"). A field the table does not list passes
/// through: the SDK's input types carry fields the table leaves out (spec/judgments.md).
pub static INPUT_SCHEMAS: LazyLock<IndexMap<&'static str, Schema>> = LazyLock::new(|| {
    let fields = &*TOOL_INPUT_FIELDS;
    IndexMap::from([
        ("Bash", schema_of(&fields.bash)),
        ("PowerShell", schema_of(&fields.power_shell)),
        ("Write", schema_of(&fields.write)),
        ("Edit", schema_of(&fields.edit)),
        ("Read", schema_of(&fields.read)),
        ("Glob", schema_of(&fields.glob)),
        ("Grep", schema_of(&fields.grep)),
        ("WebFetch", schema_of(&fields.web_fetch)),
        ("WebSearch", schema_of(&fields.web_search)),
        ("Agent", schema_of(&fields.agent)),
        ("AskUserQuestion", schema_of(&fields.ask_user_question)),
        ("ExitPlanMode", schema_of(&fields.exit_plan_mode)),
    ])
});
