//! Port of `src/cli/enable-plan.ts`, switched.
//!
//! `claude plugin enable <plugin>` as a plan (plugins/cli-reference.md, "plugin enable"; plugins/
//! install.md, "Plugins with dependencies"): which settings file gets `enabledPlugins[<id>]: true`
//! and which dependencies are enabled with it, or why nothing is. "When the plugin declares
//! dependencies, Claude Code enables them too. The command fails" when one is not installed (it
//! prints the `claude plugin install` command for each missing one), is blocked by the organization's
//! plugin policy, or "is set to `false` at a scope with higher precedence than the target scope". The
//! dependency walk is `enablePlan`'s.
//!
//! The facts cross as a value: the target is read in the host's key order (`toggle_target`).

use serde::{Deserialize, Serialize};

use super::result::{CommandOutput, Extras, ResultFields, command_succeeded};
use super::toggle_target::{
    Resolved, ToggleArguments, ToggleCommand, ToggleFacts, already_in_goal_state, failure,
    resolve_target_of, success_line,
};
use crate::js::JsValue;
use crate::marketplace::dependency_enable::{EnableBlock, EnableIssue, enable_plan};
use crate::marketplace::install_scopes::InstallScope;

/// One `enabledPlugins` entry the host writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledWrite {
    /// The settings file.
    pub scope: InstallScope,
    /// The plugin.
    pub id: String,
    /// The value written.
    pub enabled: bool,
}

/// What a toggle command writes and prints.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TogglePlan {
    /// What it prints.
    pub output: CommandOutput,
    /// The settings entries to write, dependencies first; empty when the command failed.
    pub writes: Vec<EnabledWrite>,
}

/// The plan that writes nothing.
#[must_use]
pub const fn nothing(output: CommandOutput) -> TogglePlan {
    TogglePlan {
        output,
        writes: Vec::new(),
    }
}

/// The failure codes of a refused enable: the kit's, since the docs name none (spec/judgments.md).
const fn block_code(block: EnableBlock) -> &'static str {
    match block {
        EnableBlock::NotInstalled => "dependency_not_installed",
        EnableBlock::PolicyBlocked => "dependency_blocked",
        EnableBlock::DisabledAtHigherScope => "dependency_disabled_at_higher_scope",
    }
}

fn refusal(issues: &[EnableIssue], argument: &str) -> CommandOutput {
    let mut lines = vec![format!("Failed to enable plugin \"{argument}\":")];
    lines.extend(issues.iter().map(|one| one.issue.message.clone()));
    let code = block_code(
        issues
            .first()
            .map_or(EnableBlock::NotInstalled, |one| one.block),
    );
    let dependencies: Vec<JsValue> = issues
        .iter()
        .map(|one| one.dependency.as_str().into())
        .collect();
    let fields: ResultFields = [("dependencies".to_owned(), JsValue::Array(dependencies))]
        .into_iter()
        .collect();
    failure(ToggleCommand::Enable, &lines.join("\n"), code, fields)
}

/// What `claude plugin enable` writes and prints.
///
/// # Errors
///
/// The facts are not the facts' shape.
pub fn plan_enable(args: &ToggleArguments, raw: &JsValue) -> Result<TogglePlan, String> {
    let target = match resolve_target_of(ToggleCommand::Enable, args, raw)? {
        Resolved::End { end } => return Ok(nothing(end)),
        Resolved::Target(target) => target,
    };
    let facts = ToggleFacts::deserialize(raw.clone()).map_err(|problem| problem.to_string())?;
    if let Some(already) = already_in_goal_state(ToggleCommand::Enable, &target, &facts.graph) {
        return Ok(nothing(already));
    }
    let dependencies = enable_plan(&target.id, target.scope, &facts.graph);
    if !dependencies.ok {
        return Ok(nothing(refusal(&dependencies.issues, &args.plugin)));
    }
    let fields: ResultFields = [
        ("pluginId".to_owned(), target.id.as_str().into()),
        ("scope".to_owned(), target.scope.as_str().into()),
    ]
    .into_iter()
    .collect();
    let extras = Extras {
        failure_code: None,
        fields: Some(fields),
    };
    let output = command_succeeded(
        "enable",
        &success_line(ToggleCommand::Enable, &target),
        extras,
    );
    let writes = dependencies
        .enable
        .into_iter()
        .chain(std::iter::once(target.id.clone()))
        .map(|id| EnabledWrite {
            scope: target.scope,
            id,
            enabled: true,
        })
        .collect();
    Ok(TogglePlan { output, writes })
}

crate::registry::exports! {
    "cli/enable-plan" {
        "planEnable" => fallible |args: ToggleArguments, facts: JsValue| plan_enable(&args, &facts);
    }
}
