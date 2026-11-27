//! Port of `src/settings/workflows/enablement.ts`, switched.
//!
//! Whether dynamic workflows are on, read through the host's `PolicyView`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::env_flag::is_truthy_env;
use crate::js::{JsValue, is_truthy};
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::undefined_if_none;
use crate::settings::view::PolicyView;

/// What the host tells the workflow keys besides the layers: plain data, the kit reads no `process.env`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowsContext {
    /// The host's environment, for `CLAUDE_CODE_DISABLE_WORKFLOWS`.
    pub env: IndexMap<String, Option<String>>,
    /// The signed-in plan: workflows are off by default on Pro (settings-reference.md, `enableWorkflows`).
    /// The host applies the release rule of `workflowSizeGuideline` (Claude Code v2.1.271 or later).
    pub plan: WorkflowsPlan,
    /// The organization turned workflows off on the Claude Code admin settings page (workflows.md).
    #[serde(default)]
    pub org_workflows_off: Option<bool>,
}

/// The signed-in plan, as the workflow keys read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct WorkflowsPlan {
    /// The Pro plan.
    pub pro: bool,
}

/// `CLAUDE_CODE_DISABLE_WORKFLOWS`, equivalent to `disableWorkflows` for one session (env-vars.md).
pub const WORKFLOWS_ENV: &str = "CLAUDE_CODE_DISABLE_WORKFLOWS";

/// What can turn workflows off, in the order a decision lists them.
pub const WORKFLOWS_OFF_BY: [WorkflowsOffBy; 5] = [
    WorkflowsOffBy::DisableWorkflows,
    WorkflowsOffBy::Env,
    WorkflowsOffBy::OrgPolicy,
    WorkflowsOffBy::EnableWorkflows,
    WorkflowsOffBy::Plan,
];

/// One of `WORKFLOWS_OFF_BY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowsOffBy {
    /// `disableWorkflows`.
    DisableWorkflows,
    /// The environment variable.
    Env,
    /// The organization's policy.
    OrgPolicy,
    /// `enableWorkflows: false`.
    EnableWorkflows,
    /// The Pro plan.
    Plan,
}

impl WorkflowsOffBy {
    /// The reason's name as the package writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DisableWorkflows => "disableWorkflows",
            Self::Env => "env",
            Self::OrgPolicy => "orgPolicy",
            Self::EnableWorkflows => "enableWorkflows",
            Self::Plan => "plan",
        }
    }
}

/// What turned workflows on, when nothing turned them off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowsOnBy {
    /// `enableWorkflows: true`.
    EnableWorkflows,
    /// The plan's default.
    Plan,
}

/// What workflows take with them when they are off (workflows.md, "Turn workflows off"): `/workflows`,
/// the workflow commands and the `/workflow-authoring` skill are unavailable, the `ultracode` keyword
/// no longer triggers a run and the Ultracode toggle is removed from `/effort`. A run already in progress
/// keeps going: the kit holds no run state, so that is a duty of the host, which must not cancel a run
/// because this decision turned to off (spec/judgments.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "it mirrors the interface of the package, whose members are independent facts"
)]
pub struct WorkflowSurfaces {
    /// `/workflows`.
    pub workflows_view: bool,
    /// The workflow commands.
    pub workflow_commands: bool,
    /// The `/workflow-authoring` skill.
    pub workflow_authoring_skill: bool,
    /// The `ultracode` keyword.
    pub keyword: bool,
    /// The Ultracode toggle of `/effort`.
    pub ultracode_toggle: bool,
}

/// Whether workflows are on, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowsDecision {
    /// Whether they are on.
    pub enabled: bool,
    /// Empty when enabled; otherwise every reason that applies. Any one of them keeps workflows off.
    pub off_by: Vec<WorkflowsOffBy>,
    /// Why they are on: an explicit `enableWorkflows: true`, or the plan's default.
    #[serde(serialize_with = "undefined_if_none")]
    pub enabled_by: Option<WorkflowsOnBy>,
    /// What they take with them when off.
    pub surfaces: WorkflowSurfaces,
    /// Whether `/config` shows its Dynamic workflows row.
    pub config_row_visible: bool,
}

/// Whether one reason holds. `true` in any file turns workflows off for everyone those files reach;
/// `false` "is the same as unset" and does not lift a `true` elsewhere (spec/judgments.md).
async fn off_by_reading(
    reason: WorkflowsOffBy,
    view: &dyn PolicyView,
    context: &WorkflowsContext,
) -> bool {
    match reason {
        WorkflowsOffBy::DisableWorkflows => view
            .each_from_highest("disableWorkflows")
            .await
            .iter()
            .any(|holder| is_truthy(&holder.value)),
        // An on/off variable: any on spelling (env-vars.md, "Variables"; spec J3.16).
        WorkflowsOffBy::Env => {
            is_truthy_env(context.env.get(WORKFLOWS_ENV).and_then(Option::as_deref))
        }
        WorkflowsOffBy::OrgPolicy => context.org_workflows_off == Some(true),
        WorkflowsOffBy::EnableWorkflows => {
            view.get("enableWorkflows").await == Some(JsValue::Bool(false))
        }
        WorkflowsOffBy::Plan => {
            view.get("enableWorkflows")
                .await
                .is_none_or(|value| value.is_undefined())
                && context.plan.pro
        }
    }
}

const fn surfaces(available: bool) -> WorkflowSurfaces {
    WorkflowSurfaces {
        workflows_view: available,
        workflow_commands: available,
        workflow_authoring_skill: available,
        keyword: available,
        ultracode_toggle: available,
    }
}

/// "Claude Code hides the `/config` row while a source other than your user settings sets
/// `enableWorkflows`, or sets `disableWorkflows` to `true`" (settings-reference.md, `enableWorkflows`).
/// Read with the one subject, a source other than user settings (spec/judgments.md).
async fn config_row_visible(view: &dyn PolicyView) -> bool {
    let enables = view
        .each_from_highest("enableWorkflows")
        .await
        .iter()
        .any(|holder| holder.scope != PolicyScope::User);
    let disables = view
        .each_from_highest("disableWorkflows")
        .await
        .iter()
        .any(|holder| holder.scope != PolicyScope::User && is_truthy(&holder.value));
    !enables && !disables
}

/// Whether dynamic workflows are on: off by `disableWorkflows`, by `CLAUDE_CODE_DISABLE_WORKFLOWS=1`,
/// by the organization's policy or by `enableWorkflows: false`; otherwise on, except on the Pro plan
/// where they are off unless `enableWorkflows` is `true`. Nothing that turns them off can be undone by
/// `enableWorkflows: true` or by the other off switch (settings-reference.md, `disableWorkflows`,
/// `enableWorkflows`).
pub async fn workflows_decision(
    view: &dyn PolicyView,
    context: &WorkflowsContext,
) -> WorkflowsDecision {
    let mut off_by: Vec<WorkflowsOffBy> = Vec::new();
    for reason in WORKFLOWS_OFF_BY {
        if off_by_reading(reason, view, context).await {
            off_by.push(reason);
        }
    }
    let enabled = off_by.is_empty();
    let explicit = view.get("enableWorkflows").await == Some(JsValue::Bool(true));
    let enabled_by = if !enabled {
        None
    } else if explicit {
        Some(WorkflowsOnBy::EnableWorkflows)
    } else {
        Some(WorkflowsOnBy::Plan)
    };
    WorkflowsDecision {
        enabled,
        off_by,
        enabled_by,
        surfaces: surfaces(enabled),
        config_row_visible: config_row_visible(view).await,
    }
}

crate::registry::exports! {
    "settings/workflows/enablement" {
        "WORKFLOWS_ENV" => ok || WORKFLOWS_ENV;
        "WORKFLOWS_OFF_BY" => ok || WORKFLOWS_OFF_BY.map(WorkflowsOffBy::name);
        "workflowsDecision" => async ok |view: Port, context: WorkflowsContext|
            workflows_decision(&view, &context).await;
    }
}
