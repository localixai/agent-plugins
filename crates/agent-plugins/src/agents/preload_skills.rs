//! Port of `src/agents/preload-skills.ts`, switched.
//!
//! The skills a subagent preloads into its context at startup (sub-agents.md, "Preload skills into
//! subagents"). The kit plans; loading skill content and building the system prompt are the host's.

use serde::{Deserialize, Serialize};

use crate::js::{JsValue, is_truthy};
use crate::ports::BoxFuture;
use crate::registry::Port;

/// sub-agents.md, "Preload skills into subagents": "up to the first 32 distinct names".
const PRELOAD_LIMIT: usize = 32;

/// What the host knows about its skills; narrow so the skills package need not be imported.
pub trait SkillCatalog: Send + Sync {
    /// The skill exists and is enabled (a missing one and one the organization disabled are alike).
    fn has<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool>;
    /// Claude can invoke it: not `disable-model-invocation: true` ("the same set of skills").
    fn is_model_invocable<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool>;
}

/// The host's catalog, each method asked of it (its answer read as JavaScript's truthiness).
impl SkillCatalog for Port {
    fn has<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.method::<JsValue>("has", vec![name.into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }

    fn is_model_invocable<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.method::<JsValue>("isModelInvocable", vec![name.into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }
}

/// Why a listed skill is not injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillSkipReason {
    /// No such skill, or it is disabled.
    Missing,
    /// `disable-model-invocation: true`.
    NotModelInvocable,
    /// Past the first 32 distinct names.
    BeyondLimit,
}

/// A listed skill that is not injected, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkippedSkill {
    /// Its name.
    pub name: String,
    /// Why.
    pub reason: SkillSkipReason,
}

/// The skills a subagent preloads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreloadPlan {
    /// The skills whose full content is injected at startup, in the list's order.
    pub load: Vec<String>,
    /// Listed skills that are not injected; Claude Code "skips it and logs a warning".
    pub skipped: Vec<SkippedSkill>,
}

/// The skills a subagent preloads from its `skills` field. Only the first 32 distinct names count:
/// a name past them is skipped, and so is a listed name that is missing or not model-invocable (that
/// one still uses up its place among the 32, as the docs count names "in the list"). A repeated name
/// is read once and not reported (judgment). Which skills the subagent may invoke through the Skill
/// tool is the `tools` field's, not this one's.
pub async fn plan_preloaded_skills(
    names: Option<&[String]>,
    catalog: &impl SkillCatalog,
) -> PreloadPlan {
    let mut distinct: Vec<&String> = Vec::new();
    for name in names.unwrap_or_default() {
        if !distinct.contains(&name) {
            distinct.push(name);
        }
    }
    let mut load = Vec::new();
    let mut skipped = Vec::new();
    for (index, name) in distinct.into_iter().enumerate() {
        let skip = |reason| SkippedSkill {
            name: name.clone(),
            reason,
        };
        if index >= PRELOAD_LIMIT {
            skipped.push(skip(SkillSkipReason::BeyondLimit));
        } else if !catalog.has(name).await {
            skipped.push(skip(SkillSkipReason::Missing));
        } else if !catalog.is_model_invocable(name).await {
            skipped.push(skip(SkillSkipReason::NotModelInvocable));
        } else {
            load.push(name.clone());
        }
    }
    PreloadPlan { load, skipped }
}

crate::registry::exports! {
    "agents/preload-skills" {
        "planPreloadedSkills" => async ok |names: Option<Vec<String>>, catalog: Port|
            plan_preloaded_skills(names.as_deref(), &catalog).await;
    }
}
