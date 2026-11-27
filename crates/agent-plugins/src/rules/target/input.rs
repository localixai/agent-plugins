//! Ported from `src/rules/target/input.ts`; only the core uses it now, so it has no wrapper.

use crate::js::JsValue;

/// The input fields the target matchers read: the SDK's own field names, so a rename in `AgentInput`,
/// `WebFetchInput` or `ArtifactInput` stops compiling here (E2). `skill` is the one the SDK has no
/// input type for (permissions.md, "Match by input parameter": `Skill(skill:<name>)`). In the core a
/// field is its name.
pub type TargetField = str;

/// A string field of a tool's input, or `undefined` when the input has no such string.
#[must_use]
pub fn input_string<'a>(input: &'a JsValue, field: &TargetField) -> Option<&'a str> {
    input.as_object()?.get(field)?.as_str()
}
