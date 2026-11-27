//! Ported from `src/cli/toggle-target.ts`; only the core uses it now, so it has no wrapper.
//!
//! Which plugin and which scope `claude plugin enable` and `disable` act on (plugins/cli-reference.md,
//! "plugin enable", "plugin disable"): "Without `--scope`, the command checks your settings files in
//! the order local, project, user, and uses the first scope that mentions the plugin"; a `--scope`
//! where the plugin isn't declared "either writes an override or fails"; "If the plugin is already
//! enabled at the resolved scope, the command prints `Plugin "formatter" is already enabled` and exits
//! `1`. With `--json`, the result has `"failureCode": "already_in_goal_state"` and
//! `"alreadyInGoalState": true`". Pure over the graph facts: a scope "mentions" a plugin when its
//! `enabledPlugins` holds the id.
//!
//! The scopes of `graph.enabled` are read in the order the host wrote them, as `Object.values` reads
//! them, so the matching ids keep the package's order: the facts cross as a value and are read twice.

use serde::{Deserialize, Serialize};

use super::plugin_ref::{matching_ids, qualified_commands};
use super::result::{CommandOutput, Extras, ResultFields, command_failed};
use super::scope::{ChosenOrError, CommandScope, ScopeChoice, ScopeWrite, ScopedCommand};
use super::scope::{choose_scope, detect_scope, write_scope};
use crate::js::{JsObject, JsValue};
use crate::marketplace::dependency_graph::GraphFacts;
use crate::marketplace::install_scopes::{INSTALL_SCOPES, InstallScope, is_writable_scope};

/// `enable` or `disable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToggleCommand {
    /// `enable`.
    Enable,
    /// `disable`.
    Disable,
}

impl ToggleCommand {
    const fn name(self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Disable => "disable",
        }
    }

    const fn scoped(self) -> ScopedCommand {
        match self {
            Self::Enable => ScopedCommand::Enable,
            Self::Disable => ScopedCommand::Disable,
        }
    }
}

/// The command line.
#[derive(Clone, Debug, Deserialize)]
pub struct ToggleArguments {
    /// `name` or `name@marketplace` (`name@synced` for a plugin synced from claude.ai).
    pub plugin: String,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
}

/// What the host knows (`requiredByOrganization` is read by the plans, not here).
#[derive(Clone, Debug, Deserialize)]
pub struct ToggleFacts {
    /// The installed plugins and the settings.
    pub graph: GraphFacts,
}

/// What each command sets and how it words the already-done line.
struct Goal {
    value: bool,
    adjective: &'static str,
}

const fn goal_of(command: ToggleCommand) -> Goal {
    match command {
        ToggleCommand::Enable => Goal {
            value: true,
            adjective: "enabled",
        },
        ToggleCommand::Disable => Goal {
            value: false,
            adjective: "disabled",
        },
    }
}

/// The plugin and scope a command acts on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// The plugin as the user typed it, for the lines.
    pub argument: String,
    /// Its id.
    pub id: String,
    /// The scope written.
    pub scope: InstallScope,
    /// Whether the scope declares the plugin, or the write is an override at a scope that wins.
    pub r#override: bool,
}

/// A target, or the output that ends the command.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Resolved {
    /// The target.
    Target(Target),
    /// The end.
    End {
        /// The output.
        end: CommandOutput,
    },
}

/// A failed result with its code and fields.
#[must_use]
pub fn failure(
    command: ToggleCommand,
    line: &str,
    failure_code: &str,
    fields: ResultFields,
) -> CommandOutput {
    command_failed(
        command.name(),
        line,
        Extras {
            failure_code: Some(failure_code.to_owned()),
            fields: Some(fields),
        },
    )
}

/// The scopes of `graph.enabled` in the order the host wrote them.
pub fn enabled_order(raw_graph: &JsValue) -> Vec<InstallScope> {
    let Some(enabled) = raw_graph.get("enabled").and_then(JsValue::as_object) else {
        return Vec::new();
    };
    crate::js::property_order(enabled)
        .into_iter()
        .filter_map(|(key, _)| {
            INSTALL_SCOPES
                .into_iter()
                .find(|scope| scope.as_str() == key)
        })
        .collect()
}

/// Every plugin id the settings or the install records mention.
fn known_ids(graph: &GraphFacts, order: &[InstallScope]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    let keys = order.iter().flat_map(|scope| {
        let written = match scope {
            InstallScope::User => &graph.enabled.user,
            InstallScope::Project => &graph.enabled.project,
            InstallScope::Local => &graph.enabled.local,
            InstallScope::Managed => &graph.enabled.managed,
        };
        written.iter().flat_map(|by_id| by_id.keys().cloned())
    });
    for id in graph
        .installed
        .iter()
        .map(|record| record.id.clone())
        .chain(keys)
    {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// The scopes whose settings mention the id; when none does, the scopes it is installed at.
fn declared_at(id: &str, graph: &GraphFacts) -> Vec<InstallScope> {
    let mentioned: Vec<InstallScope> = INSTALL_SCOPES
        .into_iter()
        .filter(|scope| graph.enabled.value(*scope, id).is_some())
        .collect();
    if !mentioned.is_empty() {
        return mentioned;
    }
    graph
        .installed
        .iter()
        .filter(|record| record.id == id)
        .map(|record| record.scope)
        .collect()
}

fn fields(members: [(&str, JsValue); 1]) -> ResultFields {
    members
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn not_installed(command: ToggleCommand, argument: &str) -> CommandOutput {
    let line = format!("Plugin \"{argument}\" is not installed");
    failure(
        command,
        &line,
        "not_installed",
        fields([("pluginId", argument.into())]),
    )
}

fn ambiguous(command: ToggleCommand, argument: &str, ids: &[String]) -> CommandOutput {
    let line = format!(
        "\"{argument}\" matches plugins from more than one marketplace; run one of: {}",
        qualified_commands(command.name(), ids).join(", ")
    );
    failure(command, &line, "ambiguous_plugin", JsObject::new())
}

struct ScopeRequest<'a> {
    command: ToggleCommand,
    argument: &'a str,
    id: String,
    /// The scope `--scope` names; absent when it is omitted.
    chosen: Option<CommandScope>,
}

/// The scope to write: the one asked for (a declaring scope or an override), or the first that mentions the plugin.
fn scope_for(request: ScopeRequest<'_>, graph: &GraphFacts) -> Resolved {
    let ScopeRequest {
        command,
        argument,
        id,
        chosen,
    } = request;
    let declared: Vec<InstallScope> = declared_at(&id, graph)
        .into_iter()
        .filter(|scope| is_writable_scope(*scope))
        .collect();
    let target = |scope: InstallScope, r#override: bool| {
        Resolved::Target(Target {
            argument: argument.to_owned(),
            id: id.clone(),
            scope,
            r#override,
        })
    };
    let Some(chosen) = chosen else {
        return match detect_scope(command.scoped(), &declared) {
            None => Resolved::End {
                end: not_installed(command, argument),
            },
            Some(found) => target(found, false),
        };
    };
    match write_scope(argument, chosen, &declared) {
        ScopeWrite::Refused { message } => Resolved::End {
            end: failure(command, &message, "wrong_scope", JsObject::new()),
        },
        ScopeWrite::NotInstalled => Resolved::End {
            end: not_installed(command, argument),
        },
        ScopeWrite::Write { scope } | ScopeWrite::Override { scope } => {
            target(scope, !declared.contains(&scope))
        }
    }
}

/// The plugin and scope `command` acts on, or the output that ends it.
#[must_use]
pub fn resolve_target(
    command: ToggleCommand,
    args: &ToggleArguments,
    facts: &ToggleFacts,
    order: &[InstallScope],
) -> Resolved {
    let chosen = match choose_scope(command.scoped(), args.scope.as_deref()) {
        ChosenOrError::Error(output) => return Resolved::End { end: output },
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => Some(scope),
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            None
        }
    };
    let ids = matching_ids(&args.plugin, &known_ids(&facts.graph, order));
    let Some(id) = ids.first().cloned() else {
        return Resolved::End {
            end: not_installed(command, &args.plugin),
        };
    };
    if ids.len() > 1 {
        return Resolved::End {
            end: ambiguous(command, &args.plugin, &ids),
        };
    }
    let request = ScopeRequest {
        command,
        argument: &args.plugin,
        id,
        chosen,
    };
    scope_for(request, &facts.graph)
}

/// The already-done failure, when the resolved scope already holds the value the command writes.
#[must_use]
pub fn already_in_goal_state(
    command: ToggleCommand,
    target: &Target,
    graph: &GraphFacts,
) -> Option<CommandOutput> {
    let goal = goal_of(command);
    if graph.enabled.value(target.scope, &target.id) != Some(goal.value) {
        return None;
    }
    let line = format!(
        "Plugin \"{}\" is already {}",
        target.argument, goal.adjective
    );
    let fields: ResultFields = [
        ("alreadyInGoalState", JsValue::Bool(true)),
        ("pluginId", target.id.as_str().into()),
        ("scope", target.scope.as_str().into()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    Some(failure(command, &line, "already_in_goal_state", fields))
}

/// `Successfully enabled plugin: formatter (scope: project)`.
#[must_use]
pub fn success_line(command: ToggleCommand, target: &Target) -> String {
    format!(
        "Successfully {} plugin: {} (scope: {})",
        goal_of(command).adjective,
        target.argument,
        target.scope.as_str()
    )
}

/// `resolveTarget` over the facts as the host passed them (the module comment); `Err` when they are
/// not the facts' shape.
///
/// # Errors
///
/// The facts' decoding problem.
pub fn resolve_target_of(
    command: ToggleCommand,
    args: &ToggleArguments,
    raw: &JsValue,
) -> Result<Resolved, String> {
    let facts = ToggleFacts::deserialize(raw.clone()).map_err(|problem| problem.to_string())?;
    let order = enabled_order(raw.get("graph").unwrap_or(&JsValue::Undefined));
    Ok(resolve_target(command, args, &facts, &order))
}
