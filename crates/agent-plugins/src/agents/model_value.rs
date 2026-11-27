//! Port of `src/agents/model-value.ts`, switched.
//!
//! The forms of a `model` value (sub-agents.md, "Choose a model"): "Model alias: use one of the
//! available aliases: `sonnet`, `opus`, `haiku`, or `fable`", "Full model ID ... Accepts the same
//! values as the `--model` flag", and "inherit: use the same model as the main conversation". Shared
//! by the model order (`launch-model.ts`) and the writers and allowlist (`model-choice.ts`).

use serde::{Deserialize, Serialize};

use crate::format::model_id::model_id_names;
use crate::js::trim;

/// The family aliases an Agent tool call's `model` parameter takes.
pub const MODEL_ALIASES: [ModelAlias; 4] = [
    ModelAlias::Sonnet,
    ModelAlias::Opus,
    ModelAlias::Haiku,
    ModelAlias::Fable,
];

/// One of `MODEL_ALIASES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelAlias {
    /// `sonnet`.
    Sonnet,
    /// `opus`.
    Opus,
    /// `haiku`.
    Haiku,
    /// `fable`.
    Fable,
}

impl ModelAlias {
    /// The alias as written.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sonnet => "sonnet",
            Self::Opus => "opus",
            Self::Haiku => "haiku",
            Self::Fable => "fable",
        }
    }
}

/// The family alias of a model: the alias itself, or the one a model ID embeds (`claude-opus-5-5`,
/// `claude-opus-5-5[1m]` and `my-gateway/claude-opus-5-5` are `opus`), read as `availableModels`
/// reads a family entry (model-config.md, "Add a custom model option": "A custom ID that embeds a
/// family name ... counts as a specific entry for that family"). `None` when it embeds none, as
/// an inference profile ARN Claude Code "hasn't resolved to a backing model" (sub-agents.md, "Choose
/// a model"): the `mainFamily` of `resolveSubagentModel` is then left out.
#[must_use]
pub fn model_family(model: &str) -> Option<ModelAlias> {
    MODEL_ALIASES
        .into_iter()
        .find(|alias| model_id_names(model, alias.as_str()))
}

/// The three forms the frontmatter `model` field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelValueKind {
    /// One of the aliases.
    Alias,
    /// A full model ID.
    FullId,
    /// `inherit`.
    Inherit,
}

/// Whether the value is one of the aliases.
#[must_use]
pub fn is_model_alias(value: Option<&str>) -> bool {
    MODEL_ALIASES
        .iter()
        .any(|alias| Some(alias.as_str()) == value)
}

/// Which form a `model` value is: one of the aliases `sonnet`, `opus`, `haiku`, `fable`; `inherit`; or
/// anything else, a full model ID that "accepts the same values as the `--model` flag" (so the kit
/// does not judge whether such an ID exists).
#[must_use]
pub fn model_value_kind(value: &str) -> ModelValueKind {
    let text = trim(value);
    if text == "inherit" {
        return ModelValueKind::Inherit;
    }
    if is_model_alias(Some(text)) {
        ModelValueKind::Alias
    } else {
        ModelValueKind::FullId
    }
}

crate::registry::exports! {
    "agents/model-value" {
        "MODEL_ALIASES" => ok || MODEL_ALIASES;
        "modelFamily" => ok |model: String| model_family(&model);
        "modelValueKind" => ok |value: String| model_value_kind(&value);
    }
}
