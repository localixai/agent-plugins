//! Port of `src/settings/workflows/enablement.ts` at sha256:69b7f0f1d911.
//!
//! It takes the host's `PolicyView`, so it stays
//! TypeScript in the package: only its data names are exported.

use std::collections::HashMap;

use crate::format::env_flag::is_truthy_env;
use crate::js::{JsValue, is_truthy};
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// What the host tells the workflow keys besides the layers: plain data, the kit reads no `process.env`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowsContext {
    /// The host's environment, for `CLAUDE_CODE_DISABLE_WORKFLOWS`.
    pub env: HashMap<String, String>,
    /// The signed-in plan: workflows are off by default on Pro (settings-reference.md, `enableWorkflows`).
    /// The host applies the release rule of `workflowSizeGuideline` (Claude Code v2.1.271 or later).
    pub pro: bool,
    /// The organization turned workflows off on the Claude Code admin settings page (workflows.md).
    pub org_workflows_off: Option<bool>,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowsDecision {
    /// Whether they are on.
    pub enabled: bool,
    /// Empty when enabled; otherwise every reason that applies. Any one of them keeps workflows off.
    pub off_by: Vec<WorkflowsOffBy>,
    /// Why they are on: an explicit `enableWorkflows: true`, or the plan's default.
    pub enabled_by: Option<WorkflowsOnBy>,
    /// What they take with them when off.
    pub surfaces: WorkflowSurfaces,
    /// Whether `/config` shows its Dynamic workflows row.
    pub config_row_visible: bool,
}

/// Whether one reason holds. `true` in any file turns workflows off for everyone those files reach;
/// `false` "is the same as unset" and does not lift a `true` elsewhere (spec/judgments.md).
fn off_by_reading(
    reason: WorkflowsOffBy,
    view: &dyn PolicyView,
    context: &WorkflowsContext,
) -> bool {
    match reason {
        WorkflowsOffBy::DisableWorkflows => view
            .each_from_highest("disableWorkflows")
            .iter()
            .any(|holder| is_truthy(&holder.value)),
        // An on/off variable: any on spelling (env-vars.md, "Variables"; spec J3.16).
        WorkflowsOffBy::Env => is_truthy_env(context.env.get(WORKFLOWS_ENV).map(String::as_str)),
        WorkflowsOffBy::OrgPolicy => context.org_workflows_off == Some(true),
        WorkflowsOffBy::EnableWorkflows => {
            view.get("enableWorkflows") == Some(JsValue::Bool(false))
        }
        WorkflowsOffBy::Plan => {
            view.get("enableWorkflows")
                .is_none_or(|value| value.is_undefined())
                && context.pro
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
fn config_row_visible(view: &dyn PolicyView) -> bool {
    let enables = view
        .each_from_highest("enableWorkflows")
        .iter()
        .any(|holder| holder.scope != PolicyScope::User);
    let disables = view
        .each_from_highest("disableWorkflows")
        .iter()
        .any(|holder| holder.scope != PolicyScope::User && is_truthy(&holder.value));
    !enables && !disables
}

/// Whether dynamic workflows are on: off by `disableWorkflows`, by `CLAUDE_CODE_DISABLE_WORKFLOWS=1`,
/// by the organization's policy or by `enableWorkflows: false`; otherwise on, except on the Pro plan
/// where they are off unless `enableWorkflows` is `true`. Nothing that turns them off can be undone by
/// `enableWorkflows: true` or by the other off switch (settings-reference.md, `disableWorkflows`,
/// `enableWorkflows`).
#[must_use]
pub fn workflows_decision(view: &dyn PolicyView, context: &WorkflowsContext) -> WorkflowsDecision {
    let off_by: Vec<WorkflowsOffBy> = WORKFLOWS_OFF_BY
        .into_iter()
        .filter(|reason| off_by_reading(*reason, view, context))
        .collect();
    let enabled = off_by.is_empty();
    let explicit = view.get("enableWorkflows") == Some(JsValue::Bool(true));
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
        config_row_visible: config_row_visible(view),
    }
}

crate::registry::exports! {
    "settings/workflows/enablement" {
        "WORKFLOWS_ENV" => ok || WORKFLOWS_ENV;
        "WORKFLOWS_OFF_BY" => ok || WORKFLOWS_OFF_BY.map(WorkflowsOffBy::name);
    }
}
