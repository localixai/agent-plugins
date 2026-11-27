//! Port of `src/settings/workflows/policy.ts`, switched.
//!
//! The workflow keys of the settings as one object over a view (docs/API.md, "Settings"). The host
//! holds each question as a function of the core's.

use std::sync::Arc;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use super::enablement::{WorkflowsContext, WorkflowsDecision, workflows_decision};
use super::keyword::{PromptInput, keyword_trigger};
use super::size::{WorkflowSizeContext, workflow_size};
use crate::js::JsValue;
use crate::registry::{Function, Port};
use crate::settings::view::PolicyView;

/// What the host tells the workflow keys besides the layers: one record both read (`plan` is
/// each one's).
#[derive(Clone, Debug)]
pub struct WorkflowPolicyContext {
    /// The enablement's facts.
    pub workflows: WorkflowsContext,
    /// The size guideline's facts.
    pub size: WorkflowSizeContext,
}

impl<'de> Deserialize<'de> for WorkflowPolicyContext {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = JsValue::deserialize(deserializer)?;
        Ok(Self {
            workflows: WorkflowsContext::deserialize(value.clone()).map_err(de::Error::custom)?,
            size: WorkflowSizeContext::deserialize(value).map_err(de::Error::custom)?,
        })
    }
}

/// The workflow policy as the host holds it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSettingsPolicy {
    /// Whether dynamic workflows are on, and why.
    pub enablement: Function,
    /// They are on.
    pub workflows_enabled: bool,
    /// Whether a prompt's keyword starts a workflow.
    pub keyword: Function,
    /// The size guideline.
    pub size: Function,
}

struct Enablement(WorkflowsDecision);

crate::registry::methods! {
    Enablement {
        "call" => ok |this| this.0.clone();
    }
}

struct Keyword(Arc<dyn PolicyView>, Arc<WorkflowPolicyContext>);

crate::registry::methods! {
    Keyword {
        "call" => sync async ok |this, prompt: PromptInput|
            keyword_trigger(&*this.0, &this.1.workflows, &prompt).await;
    }
}

struct Size(Arc<dyn PolicyView>, Arc<WorkflowPolicyContext>);

crate::registry::methods! {
    Size {
        "call" => sync async ok |this| workflow_size(&*this.0, &this.1.size).await;
    }
}

/// The policy over `view`.
pub async fn create_workflow_settings_policy(
    view: Arc<dyn PolicyView>,
    context: WorkflowPolicyContext,
) -> WorkflowSettingsPolicy {
    let enablement = workflows_decision(&*view, &context.workflows).await;
    let context = Arc::new(context);
    WorkflowSettingsPolicy {
        workflows_enabled: enablement.enabled,
        enablement: Function::new(Enablement(enablement)),
        keyword: Function::new(Keyword(Arc::clone(&view), Arc::clone(&context))),
        size: Function::new(Size(view, context)),
    }
}

crate::registry::exports! {
    "settings/workflows/policy" {
        "createWorkflowSettingsPolicy" => async ok |view: Port, context: WorkflowPolicyContext|
            create_workflow_settings_policy(Arc::new(view), context).await;
    }
}
