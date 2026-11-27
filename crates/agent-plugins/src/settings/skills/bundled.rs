//! Port of `src/settings/skills/bundled.ts`, switched.
//!
//! Whether the bundled skills are off: `disableBundledSkills`, read through the host's
//! `PolicyView`, and its variable.

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::env_flag::is_truthy_env;
use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// What turned the bundled skills off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BundledOffBy {
    /// `disableBundledSkills`.
    Setting,
    /// The environment variable.
    Env,
}

/// Whether the bundled skills are off, and what turned them off.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledSkillsDecision {
    /// Bundled skills and workflows are removed entirely.
    pub disabled: bool,
    /// Both when both are on: whichever turns them off, the other cannot turn them back on.
    pub decided_by: Vec<BundledOffBy>,
    /// Built-in commands such as `/init` stay typable but are hidden from the model when disabled.
    pub built_in_commands_listed_to_model: bool,
    /// Built-in commands, `/doctor` included, stay typable whatever this decides: the setting hides
    /// them from the model only. Hiding `/doctor` takes `DISABLE_DOCTOR_COMMAND`, which is not this key.
    pub built_in_commands_typable: bool,
}

/// `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS`, the name of the variable the host's environment carries.
pub const BUNDLED_SKILLS_ENV: &str = "CLAUDE_CODE_DISABLE_BUNDLED_SKILLS";

/// `disableBundledSkills` and `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS` (settings-reference.md,
/// `disableBundledSkills`: "whichever of the two turns them off, the other can't turn them back
/// on"). The setting is the one the files combine to; `env` is the host's environment as data.
pub async fn bundled_skills(
    view: &dyn PolicyView,
    env: &IndexMap<String, Option<String>>,
) -> BundledSkillsDecision {
    let mut decided_by = Vec::new();
    if view.get("disableBundledSkills").await == Some(JsValue::Bool(true)) {
        decided_by.push(BundledOffBy::Setting);
    }
    // "Set to `1`": an on/off variable with the full spelling set (J3.16).
    if is_truthy_env(env.get(BUNDLED_SKILLS_ENV).and_then(Option::as_deref)) {
        decided_by.push(BundledOffBy::Env);
    }
    let disabled = !decided_by.is_empty();
    BundledSkillsDecision {
        disabled,
        decided_by,
        built_in_commands_listed_to_model: !disabled,
        built_in_commands_typable: true,
    }
}

crate::registry::exports! {
    "settings/skills/bundled" {
        "BUNDLED_SKILLS_ENV" => ok || BUNDLED_SKILLS_ENV;
        "bundledSkills" => async ok |view: Port, env: IndexMap<String, Option<String>>|
            bundled_skills(&view, &env).await;
    }
}
