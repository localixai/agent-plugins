//! Port of `src/settings/general/agent-keys.ts`, switched.
//!
//! `agent` and `disableAgentView`, read through the host's `PolicyView`.

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::env_flag::is_truthy_env;
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// `agent` and `disableAgentView` (settings-reference.md, `agent`, `disableAgentView`). `--agent`
/// and `--teammate-mode` for one session are `SESSION_OVERRIDES` rows;
/// `CLAUDE_CODE_DISABLE_AGENT_VIEW` is paired with its key here.
pub async fn read_agent_setting(view: &dyn PolicyView) -> Option<String> {
    view.get("agent")
        .await
        .and_then(|value| value.as_str().map(str::to_owned))
}

/// `CLAUDE_CODE_DISABLE_AGENT_VIEW`, equivalent to `disableAgentView` for one session.
const AGENT_VIEW_ENV: &str = "CLAUDE_CODE_DISABLE_AGENT_VIEW";

/// What turned background agents and agent view off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentViewOffBy {
    /// The setting.
    DisableAgentView,
    /// The variable.
    Env,
}

/// Whether background agents and agent view are off, and what turned them off.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentViewDecision {
    /// Whether they are off.
    pub disabled: bool,
    /// Everything that turned it off: any one of them keeps it off.
    pub off_by: Vec<AgentViewOffBy>,
}

/// Whether background agents and agent view are off: "whichever of the two turns it off, the other
/// can't turn it back on". The variable takes the usual on values (env-vars.md: "`1`, `true`,
/// `yes`, or `on` ... in any casing"); an off value, an empty one or an unlisted one leaves it
/// unset.
pub async fn agent_view_disabled(
    view: &dyn PolicyView,
    env: &IndexMap<String, Option<String>>,
) -> AgentViewDecision {
    let mut off_by = Vec::new();
    if view
        .get("disableAgentView")
        .await
        .and_then(|value| value.as_bool())
        == Some(true)
    {
        off_by.push(AgentViewOffBy::DisableAgentView);
    }
    if is_truthy_env(env.get(AGENT_VIEW_ENV).and_then(Option::as_deref)) {
        off_by.push(AgentViewOffBy::Env);
    }
    AgentViewDecision {
        disabled: !off_by.is_empty(),
        off_by,
    }
}

crate::registry::exports! {
    "settings/general/agent-keys" {
        "readAgentSetting" => async ok |view: Port| read_agent_setting(&view).await;
        "agentViewDisabled" => async ok |view: Port, env: IndexMap<String, Option<String>>|
            agent_view_disabled(&view, &env).await;
    }
}
