//! Port of `src/cli/disable-plan.ts`, switched.
//!
//! `claude plugin disable [plugin]` as a plan (plugins/cli-reference.md, "plugin disable"): the
//! settings file that gets `enabledPlugins[<id>]: false`, or why nothing is written. "The command
//! fails for a plugin that is still required": another enabled plugin depends on it (the failure
//! "names the dependents to disable first", `disableDecision`), or the organization requires it as a
//! synced plugin ("saves nothing"). `--all` "Can't be combined with a plugin name or `--scope`"; with
//! neither a name nor `--all` the command prints `Please specify a plugin name or use --all to
//! disable all plugins` and exits `1`.
//!
//! The facts cross as a value: the scopes of `graph.enabled` are read in the host's key order
//! (`toggle_target`).

use serde::Deserialize;

use super::enable_plan::{EnabledWrite, TogglePlan, nothing};
use super::result::{CommandOutput, Extras, ResultFields, command_succeeded, usage_error};
use super::toggle_target::{
    Resolved, ToggleArguments, ToggleCommand, already_in_goal_state, enabled_order, failure,
    resolve_target_of, success_line,
};
use crate::js::JsValue;
use crate::marketplace::dependency_graph::{
    DisableDecision, GraphFacts, disable_decision, is_enabled, winning_scope,
};
use crate::marketplace::install_scopes::{InstallScope, is_writable_scope};
use crate::registry::Set;

/// The command line.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DisableArguments {
    /// `name` or `name@marketplace`.
    #[serde(default)]
    pub plugin: Option<String>,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
    /// `--all`.
    #[serde(default)]
    pub all: Option<bool>,
}

/// The members of `ToggleFacts` the disable reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisableFacts {
    graph: GraphFacts,
    #[serde(default)]
    required_by_organization: Option<Set<String>>,
}

impl DisableFacts {
    fn required(&self, id: &str) -> bool {
        self.required_by_organization
            .as_ref()
            .is_some_and(|set| set.members.iter().any(|one| one == id))
    }
}

/// `Please specify a plugin name or use --all to disable all plugins`.
pub const NAME_OR_ALL: &str = "Please specify a plugin name or use --all to disable all plugins";

fn fields<const N: usize>(members: [(&str, JsValue); N]) -> ResultFields {
    members
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn one_blocked(id: &str, facts: &DisableFacts) -> Option<CommandOutput> {
    if facts.required(id) {
        let line = format!(
            "Plugin \"{id}\" is required by your organization and can't be disabled here. Contact your admin to change it."
        );
        let found = fields([("pluginId", id.into())]);
        return Some(failure(
            ToggleCommand::Disable,
            &line,
            "required_by_organization",
            found,
        ));
    }
    match disable_decision(id, &facts.graph) {
        DisableDecision::Allowed { .. } => None,
        DisableDecision::Refused {
            dependents,
            command,
            message,
            ..
        } => {
            let found = fields([
                ("pluginId", id.into()),
                ("dependents", dependents.join(",").into()),
            ]);
            let line = format!("{message}. Run: {command}");
            Some(failure(
                ToggleCommand::Disable,
                &line,
                "has_dependents",
                found,
            ))
        }
    }
}

fn disable_one(
    args: &ToggleArguments,
    raw: &JsValue,
    facts: &DisableFacts,
) -> Result<TogglePlan, String> {
    let target = match resolve_target_of(ToggleCommand::Disable, args, raw)? {
        Resolved::End { end } => return Ok(nothing(end)),
        Resolved::Target(target) => target,
    };
    if let Some(already) = already_in_goal_state(ToggleCommand::Disable, &target, &facts.graph) {
        return Ok(nothing(already));
    }
    if let Some(blocked) = one_blocked(&target.id, facts) {
        return Ok(nothing(blocked));
    }
    let found = fields([
        ("pluginId", target.id.as_str().into()),
        ("scope", target.scope.as_str().into()),
    ]);
    let extras = Extras {
        failure_code: None,
        fields: Some(found),
    };
    let output = command_succeeded(
        "disable",
        &success_line(ToggleCommand::Disable, &target),
        extras,
    );
    let writes = vec![EnabledWrite {
        scope: target.scope,
        id: target.id,
        enabled: false,
    }];
    Ok(TogglePlan { output, writes })
}

/// Every id `graph.enabled` holds, once each, in the order `Object.values` and `Object.keys` read them.
fn enabled_ids(graph: &GraphFacts, order: &[InstallScope]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for scope in order {
        let written = match scope {
            InstallScope::User => &graph.enabled.user,
            InstallScope::Project => &graph.enabled.project,
            InstallScope::Local => &graph.enabled.local,
            InstallScope::Managed => &graph.enabled.managed,
        };
        for id in written.iter().flat_map(|by_id| by_id.keys()) {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
    }
    ids
}

/// Every plugin that is enabled, can be changed, and is not required, each disabled where it is enabled.
fn disable_all(facts: &DisableFacts, order: &[InstallScope]) -> TogglePlan {
    let writes: Vec<EnabledWrite> = enabled_ids(&facts.graph, order)
        .into_iter()
        .filter_map(|id| {
            let scope = winning_scope(&id, &facts.graph)?;
            let live = is_enabled(&id, &facts.graph);
            (live && !facts.required(&id) && is_writable_scope(scope)).then_some(EnabledWrite {
                scope,
                id,
                enabled: false,
            })
        })
        .collect();
    let message = match writes.len() {
        0 => "No enabled plugins to disable".to_owned(),
        1 => "Successfully disabled 1 plugin".to_owned(),
        count => format!("Successfully disabled {count} plugins"),
    };
    let disabled: Vec<JsValue> = writes
        .iter()
        .map(|write| write.id.as_str().into())
        .collect();
    let extras = Extras {
        failure_code: None,
        fields: Some(fields([("disabled", JsValue::Array(disabled))])),
    };
    TogglePlan {
        output: command_succeeded("disable", &message, extras),
        writes,
    }
}

/// What `claude plugin disable` writes and prints.
///
/// # Errors
///
/// The facts are not the facts' shape.
pub fn plan_disable(args: &DisableArguments, raw: &JsValue) -> Result<TogglePlan, String> {
    let facts = DisableFacts::deserialize(raw.clone()).map_err(|problem| problem.to_string())?;
    if args.all == Some(true) {
        if args.plugin.is_some() || args.scope.is_some() {
            return Ok(nothing(usage_error(
                "--all cannot be combined with a plugin name or --scope",
            )));
        }
        let order = enabled_order(raw.get("graph").unwrap_or(&JsValue::Undefined));
        return Ok(disable_all(&facts, &order));
    }
    let Some(plugin) = args.plugin.clone() else {
        return Ok(nothing(usage_error(NAME_OR_ALL)));
    };
    let one = ToggleArguments {
        plugin,
        scope: args.scope.clone(),
    };
    disable_one(&one, raw, &facts)
}

crate::registry::exports! {
    "cli/disable-plan" {
        "NAME_OR_ALL" => ok || NAME_OR_ALL;
        "planDisable" => fallible |args: DisableArguments, facts: JsValue| plan_disable(&args, &facts);
    }
}
