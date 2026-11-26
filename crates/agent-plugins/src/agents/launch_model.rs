//! Port of `src/agents/launch-model.ts`, switched.
//!
//! Which model a subagent runs on, and at which effort (sub-agents.md, "Choose a model", "Run
//! every subagent on one model" and "Choose an effort level"). Both are small decisions over
//! values the host passes: the kit reads no environment variable. Turning an alias into a version,
//! the `availableModels` allowlist and the effort levels a model supports are the host's.

use serde::{Deserialize, Serialize};

use super::builtin_model::{BuiltinModelInput, builtin_model_alias};
use super::model_value::{ModelAlias, ModelValueKind, is_model_alias, model_value_kind};
use crate::js::{JsValue, trim};
use crate::protocol::effort::EFFORT_LEVELS;

/// Where the chosen model came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelSource {
    /// The Agent tool call's `model`.
    Invocation,
    /// The definition's `model`.
    Definition,
    /// `CLAUDE_CODE_SUBAGENT_MODEL`.
    Env,
    /// A built-in's rule.
    Builtin,
    /// The main conversation's model.
    Main,
}

/// What the model order is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentModelInput {
    /// The `model` parameter of this Agent tool call: an alias ("Claude can't pass a full model ID or `inherit`").
    #[serde(default)]
    pub invocation: Option<String>,
    /// The model an installed mod set in its `agent.spawn` hook: "Claude Code uses that model in place of
    /// the per-invocation parameter". Any model the mod names counts (it is no Claude call, so the alias
    /// limit of the parameter does not bind it); judgment: ignored while the model is forced, because the
    /// docs say only Claude's own call is withdrawn there.
    #[serde(default)]
    pub mod_model: Option<String>,
    /// The definition's `model`: an alias, a full model ID or `inherit`.
    #[serde(default)]
    pub definition: Option<String>,
    /// `CLAUDE_CODE_SUBAGENT_MODEL`.
    #[serde(default)]
    pub env_model: Option<String>,
    /// `CLAUDE_CODE_SUBAGENT_MODEL_FORCE` is `1`.
    #[serde(default)]
    pub force: Option<bool>,
    /// The main conversation's model, exactly (`[1m]` suffix included).
    pub main_model: String,
    /// The main model's family (`opus`, `sonnet`...); undefined when Claude Code cannot tell it on another provider.
    #[serde(default)]
    pub main_family: Option<String>,
    /// `ANTHROPIC_DEFAULT_OPUS_MODEL` is set, so `opus` means the model you set.
    #[serde(default)]
    pub opus_pinned: Option<bool>,
    /// Set when the subagent is a built-in (not a user or project agent of the same name, which keeps its
    /// own `model`): its name and how the session connects. Explore on a Fable main conversation runs on
    /// the `opus` alias over a subscription, Console or LLM gateway ("Built-in subagents").
    #[serde(default)]
    pub builtin: Option<BuiltinModelInput>,
}

/// The model chosen, where from, and what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubagentModel {
    /// An alias or a full model ID; the main conversation's model (`mainModel`) when `kind` is `inherit`.
    pub model: String,
    /// Where it came from.
    pub from: ModelSource,
    /// What `model` is, so a host that routes models itself (another provider, its own model picker)
    /// reads the choice as data: `inherit` runs the subagent on the main conversation's model, whatever
    /// the host routes that to (`inherit` in the definition, the end of the order, or a family alias that
    /// "resolves to the main conversation's model"); `alias` is a family alias the host resolves to a
    /// version (`sonnet`, `opus`, `haiku`, `fable`); `full-id` is any other value, as written (it
    /// "accepts the same values as the `--model` flag"). The `availableModels` check
    /// (`checkAgainstAllowlist`) takes `model`.
    pub kind: ModelValueKind,
}

const INHERIT: &str = "inherit";
/// The one alias that follows the main model when its family cannot be told.
const UNKNOWN_FAMILY_ALIAS: ModelAlias = ModelAlias::Opus;

fn written(value: Option<&str>) -> Option<&str> {
    let text = trim(value?);
    (!text.is_empty()).then_some(text)
}

/// Whether a family alias resolves to the main conversation's model: "The main conversation's
/// model belongs to that family", or its family cannot be told, which "covers only the `opus`
/// alias, and doesn't apply when you set `ANTHROPIC_DEFAULT_OPUS_MODEL`".
fn means_main_model(alias: &str, input: &SubagentModelInput) -> bool {
    if let Some(family) = &input.main_family {
        return family == alias;
    }
    alias == UNKNOWN_FAMILY_ALIAS.as_str() && input.opus_pinned != Some(true)
}

fn alias_or_main(alias: &str, input: &SubagentModelInput) -> String {
    if is_model_alias(Some(alias)) && means_main_model(alias, input) {
        input.main_model.clone()
    } else {
        alias.to_owned()
    }
}

/// What a source picks.
fn pick(from: ModelSource, input: &SubagentModelInput) -> Option<String> {
    match from {
        // Judgment: a value that is no alias is not something Claude can pass, so it counts as unset.
        ModelSource::Invocation => {
            if let Some(model) = written(input.mod_model.as_deref()) {
                return Some(alias_or_main(model, input));
            }
            let value = written(input.invocation.as_deref());
            value
                .filter(|value| is_model_alias(Some(value)))
                .map(|value| alias_or_main(value, input))
        }
        // "the subagent definition's `model` frontmatter, where `inherit` selects the main conversation's model"
        ModelSource::Definition => {
            let value = written(input.definition.as_deref())?;
            Some(if value == INHERIT {
                input.main_model.clone()
            } else {
                alias_or_main(value, input)
            })
        }
        // "Setting the variable to `inherit` is the same as leaving it unset."; an alias there is never replaced.
        ModelSource::Env => {
            let value = written(input.env_model.as_deref());
            value.filter(|value| *value != INHERIT).map(str::to_owned)
        }
        ModelSource::Builtin => {
            builtin_model_alias(input.builtin.as_ref(), input.main_family.as_deref())
                .map(str::to_owned)
        }
        ModelSource::Main => Some(input.main_model.clone()),
    }
}

const ORDER: [ModelSource; 5] = [
    ModelSource::Invocation,
    ModelSource::Definition,
    ModelSource::Env,
    ModelSource::Builtin,
    ModelSource::Main,
];
/// "While `CLAUDE_CODE_SUBAGENT_MODEL_FORCE` is on, Claude Code ignores the `model` field ... and Claude can't pass a model."
const FORCED_ORDER: [ModelSource; 3] = [ModelSource::Env, ModelSource::Builtin, ModelSource::Main];

/// The kind of a chosen model: the main model is `inherit`, whichever source chose it.
fn kind_of(model: &str, input: &SubagentModelInput) -> ModelValueKind {
    if model == input.main_model {
        ModelValueKind::Inherit
    } else {
        model_value_kind(model)
    }
}

/// "Setting `CLAUDE_CODE_SUBAGENT_MODEL` by itself doesn't change the model the built-in Explore and Plan subagents run on" (§5.8.2).
const UNMOVED_BY_VARIABLE: [&str; 2] = ["Explore", "Plan"];
const ORDER_WITHOUT_ENV: [ModelSource; 4] = [
    ModelSource::Invocation,
    ModelSource::Definition,
    ModelSource::Builtin,
    ModelSource::Main,
];

fn order_of(input: &SubagentModelInput) -> &'static [ModelSource] {
    if input.force == Some(true) {
        return &FORCED_ORDER;
    }
    let builtin = input.builtin.as_ref().map(|builtin| builtin.name.as_str());
    if builtin.is_some_and(|name| UNMOVED_BY_VARIABLE.contains(&name)) {
        &ORDER_WITHOUT_ENV
    } else {
        &ORDER
    }
}

/// The model a subagent runs on, in "the subagent model order" of sub-agents.md.
#[must_use]
pub fn resolve_subagent_model(input: &SubagentModelInput) -> SubagentModel {
    for &from in order_of(input) {
        if let Some(model) = pick(from, input) {
            let kind = kind_of(&model, input);
            return SubagentModel { model, from, kind };
        }
    }
    SubagentModel {
        model: input.main_model.clone(),
        from: ModelSource::Main,
        kind: ModelValueKind::Inherit,
    }
}

/// What `CLAUDE_CODE_EFFORT_LEVEL` says when "Claude Code send[s] no effort level".
const NO_EFFORT: [&str; 2] = ["auto", "unset"];

fn as_level(value: Option<&str>) -> Option<JsValue> {
    EFFORT_LEVELS
        .iter()
        .find(|level| Some(**level) == value)
        .map(|level| JsValue::from(*level))
}

/// What the effort order is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentEffortInput {
    /// `CLAUDE_CODE_EFFORT_LEVEL`: a level, or `auto` / `unset`.
    #[serde(default)]
    pub env: Option<String>,
    /// The `effort` parameter of this Agent tool call (a non-fork subagent only).
    #[serde(default)]
    pub invocation: Option<String>,
    /// The definition's `effort`: a level or a number.
    #[serde(default)]
    pub field: Option<JsValue>,
    /// The level picked in the session (`/effort`, `--effort`, settings).
    #[serde(default)]
    pub session: Option<String>,
    /// The subagent runs on the main conversation's model.
    pub same_model_as_main: bool,
}

/// The effort a subagent runs at: the environment variable, then the invocation parameter, then
/// the `effort` field, then the session level only "on the main conversation's model" ("On another
/// model, that level doesn't apply"). `undefined` sends no level. The variable set to `auto` or
/// `unset` sends none, whatever the others say, because it "takes precedence over both".
#[must_use]
pub fn resolve_subagent_effort(input: &SubagentEffortInput) -> Option<JsValue> {
    if NO_EFFORT
        .iter()
        .any(|value| Some(*value) == input.env.as_deref())
    {
        return None;
    }
    let session = if input.same_model_as_main {
        as_level(input.session.as_deref())
    } else {
        None
    };
    // Judgment: a variable or parameter that names no level is ignored.
    [
        as_level(input.env.as_deref()),
        as_level(input.invocation.as_deref()),
        input.field.clone(),
        session,
    ]
    .into_iter()
    .flatten()
    .find(|level| !level.is_undefined())
}

crate::registry::exports! {
    "agents/launch-model" {
        "resolveSubagentModel" => ok |input: SubagentModelInput| resolve_subagent_model(&input);
        "resolveSubagentEffort" => ok |input: SubagentEffortInput| resolve_subagent_effort(&input);
    }
}
