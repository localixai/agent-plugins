//! Port of `src/loading/resolve/exported-environment.ts`, switched.
//!
//! The environment variables a component's process receives (the "Exported to the process"
//! column).

use super::field_map::{ComponentVariables, OPTION_VARIABLES, VARIABLE_FIELDS, VariableComponent};
use super::substitute::SubstitutionScope;
use super::substituter::VariableSource;
use super::user_config::option_environment;
use super::variables::{PLUGIN_VARIABLES, PluginVariable, plugin_variable_value};
use crate::js::{JsObject, promise_all};
use crate::ports::BoxFuture;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// One exporter of `EXPORTERS`: a path variable, or the options.
#[derive(Clone, Copy)]
enum Exporter {
    Path(PluginVariable),
    Options,
}

/// `EXPORTERS[variable]`.
fn exporter(variable: &str) -> Option<Exporter> {
    if variable == OPTION_VARIABLES {
        return Some(Exporter::Options);
    }
    PLUGIN_VARIABLES
        .into_iter()
        .find(|name| name.as_str() == variable)
        .map(Exporter::Path)
}

/// `VARIABLE_FIELDS[component]`.
const fn row(component: VariableComponent) -> ComponentVariables {
    match component {
        VariableComponent::HookCommand => VARIABLE_FIELDS.hook_command,
        VariableComponent::MonitorCommand => VARIABLE_FIELDS.monitor_command,
        VariableComponent::McpStdio => VARIABLE_FIELDS.mcp_stdio,
        VariableComponent::McpRemote => VARIABLE_FIELDS.mcp_remote,
        VariableComponent::Lsp => VARIABLE_FIELDS.lsp,
        VariableComponent::MarkdownContent => VARIABLE_FIELDS.markdown_content,
    }
}

/// The environment variables a component's process receives (the "Exported to the process" column):
/// the host adds them to the process it starts. Paths are as the host reports them, native: a
/// Windows process reads backslashes, except a Git Bash hook, which `resolveHookCommand` gives the
/// forward-slash form (spec J3.6). The data folder is asked for only by a component that exports it; pass the
/// `substituter` of the component's strings so it is asked once for both.
pub async fn exported_environment<L: Locations + Clone, E>(
    component: VariableComponent,
    scope: &SubstitutionScope<L, E>,
    substitution: Option<&dyn VariableSource>,
) -> JsObject {
    let variables = scope.variable_scope();
    let ask = |name: PluginVariable| -> BoxFuture<'_, String> {
        match substitution {
            Some(source) => source.variable(name),
            None => Box::pin(plugin_variable_value(name, &variables)),
        }
    };
    let parts = row(component)
        .exported
        .iter()
        .filter_map(|variable| exporter(variable))
        .map(|exporter| -> BoxFuture<'_, JsObject> {
            match exporter {
                Exporter::Path(name) => {
                    let asked = ask(name);
                    Box::pin(async move {
                        JsObject::from([(name.as_str().to_owned(), asked.await.into())])
                    })
                }
                Exporter::Options => {
                    Box::pin(std::future::ready(option_environment(&scope.user_config)))
                }
            }
        })
        .collect();
    promise_all(parts).await.into_iter().flatten().collect()
}

crate::registry::exports! {
    "loading/resolve/exported-environment" {
        "exportedEnvironment" => async ok |component: VariableComponent, scope: SubstitutionScope, substitution: Option<Port>|
            exported_environment(component, &scope, substitution.as_ref().map(|port| port as &dyn VariableSource)).await;
    }
}
