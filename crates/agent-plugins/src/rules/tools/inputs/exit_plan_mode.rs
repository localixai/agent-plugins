//! Port of `src/rules/tools/inputs/exit-plan-mode.ts`, switched.

use crate::js::{JsObject, JsValue, property_order};

/// What ExitPlanMode runs with (hooks.md, "ExitPlanMode"): `allowedPrompts` is "Deprecated. Claude
/// Code accepts the field but ignores it", so the effective input is the input without it. Anything
/// that is not an object has no fields (a judgment, spec/judgments.md).
#[must_use]
pub fn effective_exit_plan_mode_input(input: &JsValue) -> JsObject {
    let Some(members) = input.as_object() else {
        return JsObject::new();
    };
    property_order(members)
        .into_iter()
        .filter(|(field, _)| *field != "allowedPrompts")
        .map(|(field, value)| (field.clone(), value.clone()))
        .collect()
}

crate::registry::exports! {
    "rules/tools/inputs/exit-plan-mode" {
        "effectiveExitPlanModeInput" => ok |input: JsValue| effective_exit_plan_mode_input(&input);
    }
}
