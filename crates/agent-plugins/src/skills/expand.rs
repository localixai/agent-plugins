//! Port of `src/skills/expand.ts`, switched.
//!
//! Expanding a skill or a command into what the model receives and what the host must apply
//! (skills.md, "Inject dynamic context", "Available string substitutions", "Pass arguments to
//! skills", "Pre-approve tools for a skill", "Run skills in a subagent"). The order is the
//! documented one: the placeholders are found in the original text, arguments are substituted, then
//! `${CLAUDE_*}`, then the commands run, and their output is never scanned again. Effects go
//! through the ports; a plugin error is an issue, never a throw.

use serde::Serialize;

use super::execution::{
    Execution, ExecutionContext, TurnOverrides, TurnSession, read_execution_environment,
    resolve_execution, resolve_turn_overrides,
};
use super::expand_policy::{grant_applies, hooks_register};
use super::expand_shell::{
    ExpandPorts, ExpandRequest, Rendered, RequestGiven, Run, Source, mode_of, render,
};
use super::file_references::{
    FileReference, ReferenceBase, attaches_file_references, read_file_references,
};
use super::folder_line::with_folder_line;
use super::shell_scan::{Injection, find_injections};
use super::substitute::{ArgumentInput, append_arguments, substitute_argument_pieces};
use super::variables::{
    AllowedToolsSubstituted, SkillSession, SkillVariablePorts, SkillVariables, build_variables,
    substitute_allowed_tools, substitute_variables,
};
use crate::format::issues::PluginIssue;
use crate::js::value::JsValue;
use crate::js::{code_unit_length, promise_all, slice_code_units, slice_index};
use crate::ports::BoxFuture;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// The text cut at its placeholders: text at even indices, the command at the odd ones.
fn cut(body: &str, injections: &[Injection]) -> Vec<String> {
    let length = code_unit_length(body);
    let mut pieces = Vec::new();
    let mut at = 0;
    for found in injections {
        let start = slice_index(found.start, length);
        pieces.push(slice_code_units(body, at, Some(start)));
        pieces.push(found.command.clone());
        at = slice_index(found.end, length);
    }
    pieces.push(slice_code_units(body, at, None));
    pieces
}

struct Substituted {
    pieces: Vec<String>,
    /// The pieces with their arguments substituted only: what a command's shell text is made from.
    templates: Vec<String>,
    issues: Vec<PluginIssue>,
}

/// Arguments, then `${CLAUDE_*}`, over every piece (commands included: judgment in spec/judgments.md).
async fn substitute_pieces(
    pieces: &[String],
    skill: &Source,
    request: &ExpandRequest,
    vars: &SkillVariables,
) -> Substituted {
    let input = ArgumentInput {
        raw_arguments: request.arguments.clone(),
        names: skill.own.arguments.clone(),
    };
    let with_arguments = substitute_argument_pieces(pieces, &input);
    let done = promise_all(
        with_arguments
            .iter()
            .map(|piece| -> BoxFuture<'_, _> { Box::pin(substitute_variables(piece, vars, "")) })
            .collect(),
    )
    .await;
    Substituted {
        pieces: done.iter().map(|piece| piece.text.clone()).collect(),
        templates: with_arguments,
        issues: done.into_iter().flat_map(|piece| piece.issues).collect(),
    }
}

/// The text with each placeholder replaced by its rendering.
fn assemble(pieces: &[String], replacements: &[String]) -> String {
    let mut text = pieces.first().cloned().unwrap_or_default();
    for (index, replacement) in replacements.iter().enumerate() {
        text += replacement;
        text += pieces.get(2 * index + 2).map_or("", String::as_str);
    }
    text
}

/// `${CLAUDE_EFFORT}` is the level the skill runs at: the one chosen for it, else the session's.
fn effort_of(turn: &TurnOverrides, request: &ExpandRequest) -> Option<String> {
    let chosen = turn.effort.as_ref().and_then(|effort| effort.level.clone());
    chosen.or_else(|| request.effort.clone())
}

async fn variables_for(
    skill: &Source,
    request: &ExpandRequest,
    ports: ExpandPorts,
    turn: &TurnOverrides,
) -> SkillVariables {
    let effort = effort_of(turn, request);
    let session = request.session.clone().unwrap_or_default();
    let session = SkillSession {
        plugin_id: session.plugin_id,
        user_config: session.user_config,
        windows: session.windows,
        isolated_session: session.isolated_session,
        skill_root: session.skill_root,
        shell_paths: session.shell_paths,
        session_id: request.session_id.clone(),
        effort,
        signal: Some(request.signal.clone()),
    };
    let ports = SkillVariablePorts {
        locations: ports.locations,
        environment: ports.environment,
    };
    build_variables(&skill.variables, &session, &ports).await
}

/// Whether the session runs in an isolated container.
fn isolated(request: &ExpandRequest) -> bool {
    request
        .session
        .as_ref()
        .and_then(|session| session.isolated_session)
        == Some(true)
}

/// The files the skill's own text names, after its arguments, variables and command output (oracle
/// 2.1.322: Claude Code attaches what `$ARGUMENTS` and a `!` command's output name); not the folder
/// line the kit adds (judgment, spec/judgments.md).
async fn references_of(
    text: &str,
    skill: &Source,
    request: &ExpandRequest,
    ports: ExpandPorts,
) -> Vec<FileReference> {
    if !attaches_file_references(&skill.references, isolated(request)) {
        return vec![];
    }
    let ExpandPorts { locations, .. } = ports;
    let cwd = match &request.cwd {
        Some(cwd) => cwd.clone(),
        None => locations.project_dir().await,
    };
    read_file_references(text, &ReferenceBase { cwd, locations }).await
}

/// What the model receives and what the host must apply, or why the invocation stops.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Expansion {
    /// The skill expanded.
    Expanded(Box<Expanded>),
    /// It did not.
    Failed {
        /// Always false.
        ok: bool,
        /// Why.
        issues: Vec<PluginIssue>,
    },
}

/// An expanded skill.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expanded {
    /// Always true.
    pub ok: bool,
    /// What the model receives.
    pub text: String,
    /// The files the text names with `@`, which the host attaches; the references stay in `text`.
    pub references: Vec<FileReference>,
    /// `allowed-tools` with `${CLAUDE_*}` substituted in its Bash rules: pre-approved for this turn.
    pub grants: Vec<String>,
    /// The `allowed-tools` the managed settings withheld.
    pub ignored_grants: Vec<String>,
    /// `disallowed-tools`: removed from the pool for this turn.
    pub disallowed: Vec<String>,
    /// How it runs.
    pub execution: Execution,
    /// The turn's model and effort.
    pub turn: TurnOverrides,
    /// Registered when the skill is invoked and kept for the rest of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<JsValue>,
    /// Warnings that did not stop the expansion.
    pub issues: Vec<PluginIssue>,
}

/// A core package's skill (Agent Plugins Extended §18.1.5 item 5, J18.14): the body as written, no
/// argument placeholder, `${CLAUDE_*}` variable or injected command expanded, the arguments appended
/// as `ARGUMENTS: <the arguments>`; `allowed-tools` as written, under the same managed policy.
fn core_expansion(
    skill: &Source,
    request: &ExpandRequest,
    vars: &SkillVariables,
    run: (Execution, TurnOverrides),
) -> Expansion {
    let source = request
        .session
        .as_ref()
        .and_then(|session| session.plugin_source);
    let granted = grant_applies(&skill.policy, request.policy, source.unwrap_or_default());
    let allowed = &skill.allowed.allowed_tools;
    let (execution, turn) = run;
    Expansion::Expanded(Box::new(Expanded {
        ok: true,
        text: with_folder_line(
            &append_arguments(&skill.own.body, &request.arguments),
            skill_dir(vars),
        ),
        references: vec![],
        grants: if granted { allowed.clone() } else { vec![] },
        ignored_grants: if granted { vec![] } else { allowed.clone() },
        disallowed: skill.own.disallowed_tools.clone(),
        execution,
        turn,
        hooks: None,
        issues: vec![],
    }))
}

/// `vars.scope.extra?.['CLAUDE_SKILL_DIR']`.
fn skill_dir(vars: &SkillVariables) -> Option<&str> {
    let extra = vars.scope.extra.as_ref()?;
    extra.get("CLAUDE_SKILL_DIR").map(String::as_str)
}

fn is_core_skill(skill: &Source) -> bool {
    skill.own.origin.core == Some(JsValue::Bool(true))
}

/// Expands `skill` for one invocation.
pub async fn expand_skill(
    skill: &Source,
    request: &ExpandRequest,
    ports: ExpandPorts,
) -> Expansion {
    let environment = read_execution_environment(&ports.environment).await;
    let limits = request.limits.clone().unwrap_or_default();
    let session = TurnSession {
        effort_environment: environment.effort,
        ..limits
    };
    let turn = resolve_turn_overrides(&skill.runnable, &session).await;
    let context = ExecutionContext {
        interactive: request.context.interactive,
        trigger: request.context.trigger,
        running_forks: request.context.running_forks.clone(),
        background_tasks_disabled: environment.background_tasks_disabled,
    };
    let execution = resolve_execution(&skill.runnable, &context);
    let vars = variables_for(skill, request, ports, &turn).await;
    if is_core_skill(skill) {
        return core_expansion(skill, request, &vars, (execution, turn));
    }
    let injections = find_injections(&skill.own.body);
    let body = substitute_pieces(&cut(&skill.own.body, &injections), skill, request, &vars).await;
    let plugin_source = request
        .session
        .as_ref()
        .and_then(|session| session.plugin_source);
    let plugin_source = plugin_source.unwrap_or_default();
    let granted = grant_applies(&skill.policy, request.policy, plugin_source);
    let allowed = if granted {
        substitute_allowed_tools(&skill.allowed, &vars, "allowed-tools").await
    } else {
        AllowedToolsSubstituted {
            rules: vec![],
            issues: vec![],
        }
    };
    let piece = |pieces: &[String], index: usize| pieces.get(2 * index + 1).cloned();
    let commands = (0..injections.len()).map(|i| piece(&body.pieces, i).unwrap_or_default());
    let templates = (0..injections.len()).map(|i| piece(&body.templates, i).unwrap_or_default());
    let run = Run {
        skill,
        request,
        ports,
        injections: &injections,
        commands: commands.collect(),
        templates: templates.collect(),
        vars: &vars,
        grants: allowed.rules.clone(),
    };
    let replacements = match render(&run, mode_of(skill, request)).await {
        Rendered::Failed(issue) => {
            return Expansion::Failed {
                ok: false,
                issues: vec![issue],
            };
        }
        Rendered::Replaced(replacements) => replacements,
    };
    let text = assemble(&body.pieces, &replacements);
    let hooks = skill
        .policy
        .hooks
        .clone()
        .filter(|hooks| !hooks.is_undefined());
    let registers = hooks_register(&skill.policy, request.policy, plugin_source);
    Expansion::Expanded(Box::new(Expanded {
        ok: true,
        text: with_folder_line(&text, skill_dir(&vars)),
        references: references_of(&text, skill, request, ports).await,
        grants: allowed.rules,
        ignored_grants: if granted {
            vec![]
        } else {
            skill.allowed.allowed_tools.clone()
        },
        disallowed: skill.own.disallowed_tools.clone(),
        execution,
        turn,
        hooks: hooks.filter(|_| registers),
        issues: body.issues.into_iter().chain(allowed.issues).collect(),
    }))
}

crate::registry::exports! {
    "skills/expand" {
        "expandSkill" => async ok |skill: Source, request: RequestGiven, ports: Port| {
            let request = request.read().await;
            expand_skill(&skill, &request, ExpandPorts::of(ports).await).await
        };
    }
}
