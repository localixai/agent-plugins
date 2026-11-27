//! Port of `src/host/session-workflows.ts`, switched.
//!
//! The plugin workflows a session offers as commands. The state is the host's object, so the
//! wrapper passes what this reads of it: each active plugin's id and workflows, the settings view
//! (a port) and the host's workflow context, with the session's environment port.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::ports::environment::Environment;
use crate::registry::Port;
use crate::settings::workflows::enablement::{WorkflowsContext, WorkflowsPlan, workflows_decision};

/// A plugin workflow the session offers as `/<plugin>:<meta.name>` (workflows.md, "Distribute a
/// workflow in a plugin"). The kit lists it; the host runs the script at `path` (below the plugin's
/// root) with its Workflow tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowCommand {
    /// `/<plugin>:<meta.name>` without the slash.
    pub name: String,
    /// `meta.description`.
    pub description: String,
    /// The plugin's id.
    pub plugin_id: String,
    /// The script, below the plugin's root.
    pub path: String,
}

/// What this reads of a workflow's `meta`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct WorkflowMetaFacts {
    /// `description`.
    #[serde(default)]
    pub description: Option<String>,
}

/// What this reads of a plugin's workflow (`PluginWorkflow`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowFacts {
    /// Relative to the plugin root.
    pub path: String,
    /// `<plugin>:<meta.name>`, when `meta` reads.
    #[serde(default)]
    pub qualified_name: Option<String>,
    /// The `meta`, when it reads.
    #[serde(default)]
    pub meta: Option<WorkflowMetaFacts>,
}

/// One active plugin, as this reads it: its id and its workflows.
#[derive(Clone, Debug, Deserialize)]
pub struct ActiveWorkflows {
    /// The plugin's id.
    pub id: String,
    /// Its workflows, in the folder listing's order.
    pub workflows: Vec<WorkflowFacts>,
}

/// The host's `HostOptions.workflows`: the decision's context without the environment.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowsHostContext {
    /// The signed-in plan.
    pub plan: WorkflowsPlan,
    /// The organization turned workflows off.
    #[serde(default)]
    pub org_workflows_off: Option<bool>,
}

/// The plugin context of the decision: not Pro and no organization switch unless the host says so.
const NOT_PRO: WorkflowsHostContext = WorkflowsHostContext {
    plan: WorkflowsPlan { pro: false },
    org_workflows_off: None,
};

/// Whether the workflow commands are available (workflows.md, "Turn workflows off").
async fn commands_on(
    view: &Port,
    context: Option<WorkflowsHostContext>,
    environment: &Port,
) -> bool {
    let context = context.unwrap_or(NOT_PRO);
    let env: IndexMap<String, Option<String>> = environment
        .snapshot()
        .await
        .into_iter()
        .map(|(name, value)| (name, Some(value)))
        .collect();
    let context = WorkflowsContext {
        env,
        plan: context.plan,
        org_workflows_off: context.org_workflows_off,
    };
    workflows_decision(view, &context)
        .await
        .surfaces
        .workflow_commands
}

/// The workflow commands of the plugins that run, in their loading order. Oracle 2.1.319
/// (spec/judgments.md): a script whose `meta` does not read (not the first statement, no description)
/// is not one, and of two scripts with one `meta.name` the first in the folder listing is kept.
pub async fn workflow_commands_of(
    active: Vec<ActiveWorkflows>,
    view: &Port,
    context: Option<WorkflowsHostContext>,
    environment: &Port,
) -> Vec<WorkflowCommand> {
    // The decision reads the whole environment; with no workflow to list it would change nothing.
    if !active.iter().any(|plugin| !plugin.workflows.is_empty()) {
        return Vec::new();
    }
    if !commands_on(view, context, environment).await {
        return Vec::new();
    }
    let mut seen = std::collections::HashSet::new();
    let mut commands = Vec::new();
    for plugin in active {
        for workflow in plugin.workflows {
            let description = workflow.meta.and_then(|meta| meta.description);
            let (Some(name), Some(description)) = (workflow.qualified_name, description) else {
                continue;
            };
            if !seen.insert(name.clone()) {
                continue;
            }
            commands.push(WorkflowCommand {
                name,
                description,
                plugin_id: plugin.id.clone(),
                path: workflow.path,
            });
        }
    }
    commands
}

crate::registry::exports! {
    "host/session-workflows" {
        "workflowCommandsOf" => async ok |active: Vec<ActiveWorkflows>, view: Port, context: Option<WorkflowsHostContext>, environment: Port|
            workflow_commands_of(active, &view, context, &environment).await;
    }
}
