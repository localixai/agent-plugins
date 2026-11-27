//! The part of `src/skills/execution.ts` that decides the model and effort a skill sets for its
//! turn (`resolveTurnOverrides`), kept apart for the file budget (RS5).

use serde::{Deserialize, Serialize};

use super::Runnable;
use crate::js::JsValue;
use crate::protocol::effort::EFFORT_LEVELS;
use crate::registry::{Callback, Port};

/// What the host knows about which models may run (`availableModels`, the permission mode): a record
/// of functions, so it crosses as a port (`allowed`, and the optional `supportedInMode` and
/// `substitute`).
type ModelAvailability = Port;

/// What the host knows of the session a skill's turn runs in.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSession {
    /// Which models may run; absent: any.
    #[serde(default)]
    pub models: Option<ModelAvailability>,
    /// `CLAUDE_CODE_EFFORT_LEVEL`, from `readExecutionEnvironment`.
    #[serde(default)]
    pub effort_environment: Option<String>,
    /// `maxEffortLevel` or an organization cap: "still limits the level the skill ... runs at".
    #[serde(default)]
    pub max_effort: Option<String>,
    /// The levels the active model supports; an unsupported level "falls back to the highest supported
    /// level at or below" (`xhigh` runs as `high`). Absent: all. Judgment: none at or below sends no level.
    #[serde(default)]
    pub supported_efforts: Option<Vec<String>>,
}

/// Why the skill's `model` was not used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKept {
    /// The allowlist excludes it.
    Excluded,
    /// The permission mode does not support it.
    UnsupportedInMode,
}

/// Where a level came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortSource {
    /// The skill's `effort`.
    Skill,
    /// `CLAUDE_CODE_EFFORT_LEVEL`.
    Environment,
}

/// The level a turn runs at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffortChoice {
    /// Absent when the environment says to send no level, or the model supports none at or below it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Where it came from.
    pub source: EffortSource,
}

/// The rest of the turn, or the forked subagent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// The rest of the turn.
    Turn,
    /// The forked subagent.
    Subagent,
}

/// The model and effort a skill sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnOverrides {
    /// The rest of the turn, or the forked subagent.
    pub target: Target,
    /// The model to run on; absent keeps the session's (or, for a fork, the inherited) model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Why the skill's `model` was not used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_kept: Option<ModelKept>,
    /// A forked skill's blocked family alias was replaced by a permitted version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_substituted: Option<bool>,
    /// The level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<EffortChoice>,
}

/// What a model rule decides: `model`, `modelKept` and `modelSubstituted`.
#[derive(Default)]
struct ModelOutcome {
    model: Option<String>,
    model_kept: Option<ModelKept>,
    model_substituted: Option<bool>,
}

const fn kept(reason: ModelKept) -> ModelOutcome {
    ModelOutcome {
        model: None,
        model_kept: Some(reason),
        model_substituted: None,
    }
}

fn chosen(model: String) -> ModelOutcome {
    ModelOutcome {
        model: Some(model),
        ..ModelOutcome::default()
    }
}

/// `models.allowed(model)`, or `true` with no models (`ANY_MODEL`).
async fn allowed(models: Option<Port>, model: &str) -> bool {
    match models {
        None => true,
        Some(models) => models.method::<bool>("allowed", vec![model.into()]).await,
    }
}

/// `models.<name>?.(model)`: the optional callback's answer, `undefined` when it is absent.
async fn optional_call(models: Option<Port>, name: &str, model: &str) -> JsValue {
    let Some(models) = models else {
        return JsValue::Undefined;
    };
    match models.member::<Option<Callback>>(name).await {
        Some(callback) => callback.call::<JsValue>(vec![model.into()]).await,
        None => JsValue::Undefined,
    }
}

/// "Skill or command override: Claude Code ignores the override ... and the skill or command runs on
/// the session model"; a skill "that runs in a subagent follows the subagent behavior instead": a
/// substitute, else the inherited model. Judgment: the auto-mode rule is stated for the turn's
/// override, so a fork is checked against the allowlist only.
async fn model_rule(target: Target, model: &str, models: Option<Port>) -> ModelOutcome {
    match target {
        Target::Turn => {
            if !allowed(models, model).await {
                return kept(ModelKept::Excluded);
            }
            if optional_call(models, "supportedInMode", model).await == JsValue::Bool(false) {
                return kept(ModelKept::UnsupportedInMode);
            }
            chosen(model.to_owned())
        }
        Target::Subagent => {
            if allowed(models, model).await {
                return chosen(model.to_owned());
            }
            let substitute = optional_call(models, "substitute", model).await;
            if substitute.is_undefined() {
                return kept(ModelKept::Excluded);
            }
            ModelOutcome {
                model: substitute.as_str().map(str::to_owned),
                model_kept: None,
                model_substituted: Some(true),
            }
        }
    }
}

fn level_index(level: &str) -> Option<usize> {
    EFFORT_LEVELS.iter().position(|one| *one == level)
}

/// `indexOf` is `-1` for a level that is none, below every level: `None` orders below `Some`.
fn capped(level: &str, max: Option<&str>) -> String {
    match max {
        Some(max) if level_index(level) > level_index(max) => max.to_owned(),
        _ => level.to_owned(),
    }
}

/// The highest level the model supports at or below `level`, or `None` when there is none.
fn supported(level: &str, levels: Option<&[String]>) -> Option<String> {
    let Some(levels) = levels else {
        return Some(level.to_owned());
    };
    let end = level_index(level).map_or(0, |at| at + 1);
    EFFORT_LEVELS
        .get(..end)
        .unwrap_or_default()
        .iter()
        .rev()
        .find(|candidate| levels.iter().any(|one| one == **candidate))
        .map(|candidate| (*candidate).to_owned())
}

/// The cap lowers the level, then the model's support lowers it again.
fn settle(level: &str, source: EffortSource, session: &TurnSession) -> EffortChoice {
    let settled = supported(
        &capped(level, session.max_effort.as_deref()),
        session.supported_efforts.as_deref(),
    );
    EffortChoice {
        level: settled,
        source,
    }
}

/// The environment variable wins over the skill; the cap and the model's support lower either.
fn choose_effort(skill_level: &str, session: &TurnSession) -> EffortChoice {
    match session.effort_environment.as_deref() {
        Some("none") => EffortChoice {
            level: None,
            source: EffortSource::Environment,
        },
        Some(from_environment) => settle(from_environment, EffortSource::Environment, session),
        None => settle(skill_level, EffortSource::Skill, session),
    }
}

/// The skill's own `model`: `inherit` and no value both keep the model the session would use.
fn requested_model(skill: &Runnable) -> Option<&str> {
    skill.model.as_deref().filter(|model| *model != "inherit")
}

/// The model and effort a skill sets for the rest of the turn, or for its forked subagent. Absent
/// fields override nothing: the session's own resolution order applies.
pub async fn resolve_turn_overrides(skill: &Runnable, session: &TurnSession) -> TurnOverrides {
    let target = if skill.context.is_undefined() {
        Target::Turn
    } else {
        Target::Subagent
    };
    let model = match requested_model(skill) {
        None => ModelOutcome::default(),
        Some(requested) => model_rule(target, requested, session.models).await,
    };
    let effort = skill
        .effort
        .as_deref()
        .map(|level| choose_effort(level, session));
    TurnOverrides {
        target,
        model: model.model,
        model_kept: model.model_kept,
        model_substituted: model.model_substituted,
        effort,
    }
}
