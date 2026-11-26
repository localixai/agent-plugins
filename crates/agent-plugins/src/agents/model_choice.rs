//! Port of `src/agents/model-choice.ts`, switched.
//!
//! What a `model` value is, who may write which kind, and what happens to a value the organization's
//! `availableModels` allowlist blocks (sub-agents.md, "Choose a model"). The order of the sources is
//! `resolveSubagentModel`'s; this file holds the rest of the section. Resolving an alias to a version
//! and reading the allowlist are the host's: it passes what the allowlist permits.

use serde::{Deserialize, Serialize};

use super::model_value::{ModelValueKind, is_model_alias, model_value_kind};
use crate::js::{JsValue, is_truthy, trim};
use crate::ports::BoxFuture;
use crate::registry::Port;

/// Who writes a `model` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Writer {
    /// A definition's frontmatter.
    Frontmatter,
    /// An Agent tool call.
    Invocation,
}

/// Which forms each writer may use: "Claude can't pass a full model ID or `inherit` on that call".
const fn accepts(writer: Writer) -> &'static [ModelValueKind] {
    match writer {
        Writer::Frontmatter => &[
            ModelValueKind::Alias,
            ModelValueKind::FullId,
            ModelValueKind::Inherit,
        ],
        Writer::Invocation => &[ModelValueKind::Alias],
    }
}

/// Whether the `model` field of a definition, or the `model` parameter of an Agent call, may hold the value.
#[must_use]
pub fn model_value_accepted(writer: Writer, value: &str) -> bool {
    accepts(writer).contains(&model_value_kind(value))
}

/// What the organization's `availableModels` allowlist says, as the host evaluates it.
pub trait ModelAllowlist: Send + Sync {
    /// The model may be used.
    fn permits<'a>(&'a self, model: &'a str) -> BoxFuture<'a, bool>;
    /// The newest version of a family the allowlist permits (the substitution rules of `/model`), or `undefined`.
    fn newest_permitted<'a>(&'a self, family: &'a str) -> BoxFuture<'a, Option<String>>;
}

impl ModelAllowlist for Port {
    fn permits<'a>(&'a self, model: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.method::<JsValue>("permits", vec![model.into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }

    fn newest_permitted<'a>(&'a self, family: &'a str) -> BoxFuture<'a, Option<String>> {
        let asked = self.method::<JsValue>("newestPermitted", vec![family.into()]);
        Box::pin(async move { asked.await.as_str().map(str::to_owned) })
    }
}

/// What was asked for, when it was replaced: the interactive warning "naming the requested model
/// and the model the subagent runs on".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Replaced {
    /// What was asked for.
    pub requested: String,
    /// What runs.
    pub running: String,
}

/// The model to run, and what it replaced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckedModel {
    /// The model to run.
    pub model: String,
    /// What was asked for, when it was replaced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced: Option<Replaced>,
}

/// What the allowlist check is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCheckInput {
    /// The model the order of `resolveSubagentModel` chose (an alias or a full ID).
    pub chosen: String,
    /// `CLAUDE_CODE_SUBAGENT_MODEL`, tried first when the chosen value is blocked.
    #[serde(default)]
    pub env_model: Option<String>,
    /// The inherited model: the main conversation's.
    pub inherited: String,
    /// Absent: no allowlist, nothing is blocked.
    #[serde(default)]
    pub allowlist: Option<Port>,
}

/// A blocked family alias runs on the newest version of its family the allowlist permits.
async fn family_version(value: &str, allowlist: &impl ModelAllowlist) -> Option<String> {
    let text = trim(value);
    if is_model_alias(Some(text)) {
        allowlist.newest_permitted(text).await
    } else {
        None
    }
}

/// The value itself when permitted, else its family's newest permitted version, else nothing.
async fn permitted(value: &str, allowlist: &impl ModelAllowlist) -> Option<String> {
    if allowlist.permits(value).await {
        return Some(value.to_owned());
    }
    family_version(value, allowlist).await
}

fn usable(env_model: Option<&str>) -> Option<&str> {
    let text = trim(env_model?);
    (!text.is_empty() && text != "inherit").then_some(text)
}

/// Applies the allowlist to the model a subagent was about to run on. A blocked family alias such as
/// `opus` becomes the newest version of that family the allowlist permits; "for any other blocked
/// value ... or when the allowlist permits no version of the family" the subagent runs on the
/// inherited model, "If you set `CLAUDE_CODE_SUBAGENT_MODEL`, Claude Code tries that model first,
/// under these same rules". The inherited model is never checked: it is what the session already runs.
pub async fn check_against_allowlist(
    chosen: &str,
    env_model: Option<&str>,
    inherited: &str,
    allowlist: Option<&impl ModelAllowlist>,
) -> CheckedModel {
    let Some(allowlist) = allowlist else {
        return CheckedModel {
            model: chosen.to_owned(),
            replaced: None,
        };
    };
    let direct = permitted(chosen, allowlist).await;
    let via_env = usable(env_model);
    let fallback = match via_env {
        None => None,
        Some(via_env) => permitted(via_env, allowlist).await,
    };
    let running = direct.or(fallback).unwrap_or_else(|| inherited.to_owned());
    if running == chosen {
        CheckedModel {
            model: chosen.to_owned(),
            replaced: None,
        }
    } else {
        CheckedModel {
            model: running.clone(),
            replaced: Some(Replaced {
                requested: chosen.to_owned(),
                running,
            }),
        }
    }
}

crate::registry::exports! {
    "agents/model-choice" {
        "modelValueAccepted" => ok |writer: Writer, value: String| model_value_accepted(writer, &value);
        "checkAgainstAllowlist" => async ok |input: ModelCheckInput|
            check_against_allowlist(&input.chosen, input.env_model.as_deref(), &input.inherited, input.allowlist.as_ref()).await;
    }
}
