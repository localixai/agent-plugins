//! Port of `src/settings/hooks/policy.ts`, switched.
//!
//! The keys that decide which hooks run, as one object over a view (docs/API.md, "Settings"): the
//! hooks engine asks `runs(origin)` before it runs any hook and `httpHook(url, allowedEnvVars)`
//! before it sends an HTTP request. The same decision as `hookSessionGates`, in the shape of
//! questions. The host holds each question as a function of the core's.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::custom_commands::{CommandGates, CommandSessionFacts, create_command_gates};
use super::hook_gate::HookGate;
use super::http_policy::HttpHookUrlPolicy;
use super::origins::PolicyHookSource;
use super::session_gates::hook_decision;
use crate::registry::{Function, Port};
use crate::settings::managed::parent::{ParentSettingsContext, effective_view};
use crate::settings::view::PolicyView;

/// What the host tells the hook policy besides the layers.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookPolicyContext {
    /// The parent-settings facts.
    #[serde(flatten)]
    pub parent: ParentSettingsContext,
    /// The session facts.
    #[serde(flatten)]
    pub facts: CommandSessionFacts,
}

/// The hook policy as the host holds it.
#[derive(Debug, Serialize)]
pub struct HookPolicy {
    /// The commands that follow the hooks.
    #[serde(flatten)]
    pub commands: CommandGates,
    /// Whether a hook of an origin runs.
    pub runs: Function,
    /// Whether an HTTP hook may send its request.
    #[serde(rename = "httpHook")]
    pub http_hook: Function,
}

struct Runs(Arc<HookGate>);

crate::registry::methods! {
    Runs {
        "call" => sync async ok |this, origin: PolicyHookSource| this.0.runs(&origin).await;
    }
}

struct HttpHook(HttpHookUrlPolicy);

crate::registry::methods! {
    HttpHook {
        "call" => ok |this, url: String, hook_allowed_env_vars: Vec<String>|
            this.0.check(&url, &hook_allowed_env_vars);
    }
}

/// The hook policy over a view the parent rules were already applied to (`resolvePolicy`).
pub async fn hook_policy_over(
    effective: Arc<dyn PolicyView>,
    facts: CommandSessionFacts,
) -> HookPolicy {
    let decision = hook_decision(effective).await;
    HookPolicy {
        commands: create_command_gates(&decision.gate, facts),
        runs: Function::new(Runs(Arc::clone(&decision.gate))),
        http_hook: Function::new(HttpHook(decision.http)),
    }
}

/// The hook policy over `view` under the parent-settings rules.
pub async fn create_hook_policy(
    view: Arc<dyn PolicyView>,
    context: &HookPolicyContext,
) -> HookPolicy {
    hook_policy_over(effective_view(view, &context.parent).await, context.facts).await
}

crate::registry::exports! {
    "settings/hooks/policy" {
        "hookPolicyOver" => async ok |effective: Port, facts: CommandSessionFacts|
            hook_policy_over(Arc::new(effective), facts).await;
        "createHookPolicy" => async ok |view: Port, context: HookPolicyContext|
            create_hook_policy(Arc::new(view), &context).await;
    }
}
