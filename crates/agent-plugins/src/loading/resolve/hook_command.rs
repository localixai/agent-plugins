//! Port of `src/loading/resolve/hook-command.ts`, switched.
//!
//! A command hook ready for the host's `CommandRunner` (plugins/manifest-reference.md, "Where each
//! variable resolves"; hooks.md, "Exec form and shell form").

use std::sync::LazyLock;

use indexmap::IndexSet;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::exported_environment::exported_environment;
use super::field_map::VariableComponent;
use super::hook_space_path::{hook_needs_quotes, unquoted_space_path};
use super::powershell_no_program::no_program_issue;
use super::shell_guard::reject_user_config_in_shell;
use super::shell_guard_hook::{HOOK_GUARD, HookGuardContext};
use super::substitute::{FieldPolicy, SubstitutionScope, UserConfigPolicy};
use super::substituter::{Substituter, VariableSource, substituter};
use super::surfaces::{ShellPathsScope, SubstitutionSurface, surface_policy};
use super::variables::PluginVariable;
use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsObject, JsValue, Pattern, is_whitespace, promise_all, promise_all_pair};
use crate::ports::BoxFuture;
use crate::ports::command_runner::{CommandSpec, ShellKind};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// A `command` hook handler, as the official type has it (the members read here).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookCommandHandler {
    /// The command.
    pub command: String,
    /// Exec form's arguments.
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// The shell of shell form.
    #[serde(default)]
    pub shell: Option<ShellKind>,
    /// Whether the hook runs in the background and wakes the model.
    #[serde(default)]
    pub async_rewake: Option<bool>,
}

/// How a hook is resolved.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookCommandOptions {
    /// Where the hook sits, for the issues (`hooks.PostToolUse[0].hooks[1]`). Default `hook`.
    #[serde(default)]
    pub path: Option<String>,
    /// The shell a hook with no `shell` field runs in on this host. hooks.md: "Defaults to `bash`,
    /// or to `powershell` on Windows when Git Bash isn't installed"; the kit cannot tell (L3), so a
    /// Windows host without Git Bash says `powershell`. Default `bash`.
    #[serde(default)]
    pub default_shell: Option<ShellKind>,
    /// False for a hook of no plugin (settings, a skill's frontmatter): `${user_config.*}` stays as
    /// written in both forms, shell form is not refused for it (plugins/manifest-reference.md:
    /// "Plugin hooks additionally substitute `${user_config.*}` values"), and `environment` holds
    /// only `CLAUDE_PROJECT_DIR`. Default true.
    #[serde(default)]
    pub of_plugin: Option<bool>,
    /// The hook's event, which the path-with-a-space notification names (`Stop hook needs quotes ...`).
    #[serde(default)]
    pub event: Option<String>,
}

/// A command hook ready for the host's `CommandRunner`. The kit never spawns it: the host runs
/// `spec` with `environment` added to the process, in the working directory and with the timeout
/// the hooks engine decides.
#[derive(Clone, Debug, Serialize)]
pub struct ResolvedHookCommand {
    /// What to run.
    pub spec: CommandSpec,
    /// The three path variables and `CLAUDE_PLUGIN_OPTION_<KEY>` for every option with a value,
    /// sensitive ones included, in both forms (only `CLAUDE_PROJECT_DIR` for a hook of no plugin)
    /// (hooks.md: "both export them ... on the spawned process").
    pub environment: JsObject,
}

/// One hook resolved.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookCommandResolution {
    /// Undefined when the hook does not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedHookCommand>,
    /// Set when the hook runs but would start no program (`NO_PROGRAM`): the run is a failure to
    /// start, with this message. Undefined for a hook that resolved or was refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_started: Option<String>,
    /// Set when an `asyncRewake` shell-form hook does not run because an unquoted path placeholder
    /// resolves to a path with a space: the notification for the conversation view (errors.md,
    /// "Hook needs quotes around a path with a space").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

#[derive(Default)]
struct Plan {
    spec: Option<CommandSpec>,
    not_started: Option<String>,
    notice: Option<String>,
    issues: Vec<PluginIssue>,
}

/// What each shell does to the exported path variables on a Windows host: a Git Bash hook gets
/// them with forward slashes, as its command text has them; PowerShell keeps the native paths (spec
/// §3.2.4, J3.6; plugins/troubleshooting.md: a Git Bash hook "receives `${CLAUDE_PLUGIN_ROOT}`" with
/// forward slashes). Option values are never touched.
const PATH_VARIABLES: [&str; 3] = [
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PLUGIN_DATA",
    "CLAUDE_PROJECT_DIR",
];

fn forward_slashes(mut environment: JsObject) -> JsObject {
    for name in PATH_VARIABLES {
        if let Some(JsValue::String(value)) = environment.get_mut(name) {
            *value = value.replace('\\', "/");
        }
    }
    environment
}

fn shell_environment(shell: ShellKind, environment: JsObject) -> JsObject {
    match shell {
        ShellKind::Bash => forward_slashes(environment),
        ShellKind::Powershell => environment,
    }
}

/// The surface of each shell: PowerShell rewrites the placeholders, bash gets forward slashes.
const fn shell_surface(shell: ShellKind) -> SubstitutionSurface {
    match shell {
        ShellKind::Bash => SubstitutionSurface::HookShell,
        ShellKind::Powershell => SubstitutionSurface::HookPowershell,
    }
}

/// The policy of one surface for this hook: the scope's `shellPaths` sets a bash command's path
/// rendering (`surfacePolicy`; PowerShell's is always a reference), and a hook of no plugin keeps
/// `${user_config.*}` as written.
fn policy_of<L, E>(surface: SubstitutionSurface, run: &Run<'_, L, E>) -> FieldPolicy {
    let shell = ShellPathsScope {
        shell_paths: run.scope.shell_paths,
    };
    let policy = surface_policy(surface, shell);
    if run.options.of_plugin == Some(false) {
        FieldPolicy {
            user_config: UserConfigPolicy::Refuse,
            ..policy
        }
    } else {
        policy
    }
}

/// hooks.md, "Reference scripts by path in PowerShell hooks": the bare `$CLAUDE_PROJECT_DIR` form
/// "parses as an undefined local variable" and "Claude Code doesn't rewrite that form; it logs a
/// warning". `$env:NAME` and `${env:NAME}` do not match: the `$` is followed by `env`, not the name.
/// Judgment: the same holds for the other two exported path variables; each name is reported once.
static BARE_VARIABLE: Pattern = LazyLock::new(|| {
    Regex::new(r"\$(CLAUDE_(?:PROJECT_DIR|PLUGIN_ROOT|PLUGIN_DATA))(?-u:\b)").ok()
});

fn bare_variable_warnings(command: &str, path: &str) -> Vec<PluginIssue> {
    let Some(pattern) = BARE_VARIABLE.as_ref() else {
        return Vec::new();
    };
    let names: IndexSet<&str> = pattern
        .captures_iter(command)
        .filter_map(|found| Some(found.get(1)?.as_str()))
        .collect();
    names
        .into_iter()
        .map(|name| {
            let message = format!(
                "${name} is an undefined variable in PowerShell and resolves to $null. Write ${{{name}}} or $env:{name}."
            );
            warning(path, &message)
        })
        .collect()
}

/// Per shell, what else a shell-form command is checked for.
fn shell_checks(shell: ShellKind, command: &str, path: &str) -> Vec<PluginIssue> {
    match shell {
        ShellKind::Bash => Vec::new(),
        ShellKind::Powershell => bare_variable_warnings(command, path),
    }
}

/// hooks.md, "Exec form": "If `command` is a bare name with no path separator and contains
/// whitespace alongside `args`, Claude Code logs a warning because the spawn will fail"; an
/// absolute path with spaces is "a single valid executable" and does not warn.
fn bare_name_warning(command: &str, path: &str) -> Vec<PluginIssue> {
    if !command.contains(['\\', '/']) && command.chars().any(is_whitespace) {
        vec![warning(
            path,
            &format!(
                "\"{command}\" is a bare name with whitespace and \"args\" is set, so no executable of that name will start. Move the extra tokens into \"args\"."
            ),
        )]
    } else {
        Vec::new()
    }
}

/// One hook being resolved: its scope, the one substituter, the options and the issue path.
struct Run<'a, L, E> {
    scope: &'a SubstitutionScope<L, E>,
    substitution: Substituter<'a, L, E>,
    options: &'a HookCommandOptions,
    base: String,
}

async fn exec_form<L: Locations + Clone, E: Environment>(
    handler: &HookCommandHandler,
    args: &[String],
    run: &Run<'_, L, E>,
) -> Plan {
    let Run {
        substitution, base, ..
    } = run;
    let policy = policy_of(SubstitutionSurface::HookExec, run);
    let paths: Vec<String> = (0..args.len())
        .map(|index| format!("{base}.args[{index}]"))
        .collect();
    let each = args
        .iter()
        .zip(&paths)
        .map(|(arg, path)| -> BoxFuture<'_, _> { Box::pin(substitution.run(arg, &policy, path)) })
        .collect();
    let path = format!("{base}.command");
    let (command, each) = promise_all_pair(
        Box::pin(substitution.run(&handler.command, &policy, &path)),
        promise_all(each),
    )
    .await;
    let mut issues = command.issues;
    issues.extend(each.iter().flat_map(|one| one.issues.iter().cloned()));
    issues.extend(bare_name_warning(&command.text, &path));
    Plan {
        spec: Some(CommandSpec::Exec {
            command: command.text,
            args: each.into_iter().map(|one| one.text).collect(),
        }),
        issues,
        ..Plan::default()
    }
}

/// The errors.md refusal of a plugin's shell-form `${user_config.*}`, read from the template.
async fn refusal_of<L, E>(
    handler: &HookCommandHandler,
    scope: &SubstitutionScope<L, E>,
    options: &HookCommandOptions,
    path: &str,
) -> Option<PluginIssue> {
    if options.of_plugin == Some(false) {
        return None;
    }
    let context = HookGuardContext {
        plugin: scope.plugin_id.clone(),
        command: handler.command.clone(),
    };
    reject_user_config_in_shell(&HOOK_GUARD, &handler.command, &context, path).await
}

async fn shell_form<L: Locations + Clone, E: Environment>(
    handler: &HookCommandHandler,
    run: &Run<'_, L, E>,
) -> Plan {
    let Run {
        scope,
        substitution,
        options,
        base,
    } = run;
    let path = format!("{base}.command");
    if let Some(refused) = refusal_of(handler, scope, options, &path).await {
        return Plan {
            issues: vec![refused],
            ..Plan::default()
        };
    }
    let shell = handler
        .shell
        .or(options.default_shell)
        .unwrap_or(ShellKind::Bash);
    if let Some(none) = no_program_issue(shell, handler.shell, &path, &handler.command) {
        return Plan {
            not_started: Some(none.message.clone()),
            issues: vec![none],
            ..Plan::default()
        };
    }
    let script = if handler.async_rewake == Some(true) {
        unquoted_space_path(&handler.command, substitution).await
    } else {
        None
    };
    if let Some(script) = script {
        let notice = hook_needs_quotes(options.event.as_deref(), &script);
        let issues = vec![warning(&path, &notice)];
        return Plan {
            notice: Some(notice),
            issues,
            ..Plan::default()
        };
    }
    let policy = policy_of(shell_surface(shell), run);
    let command = substitution.run(&handler.command, &policy, &path).await;
    let mut issues = command.issues;
    issues.extend(shell_checks(shell, &handler.command, &path));
    Plan {
        spec: Some(CommandSpec::Shell {
            command: command.text,
            shell,
        }),
        issues,
        ..Plan::default()
    }
}

/// What the process gets: a plugin's hook the three path variables and its options; a hook of no
/// plugin only `CLAUDE_PROJECT_DIR`, since it has no plugin folder, data folder or options.
async fn environment_of<L: Locations + Clone, E: Environment>(run: &Run<'_, L, E>) -> JsObject {
    if run.options.of_plugin == Some(false) {
        let project = run
            .substitution
            .variable(PluginVariable::ClaudeProjectDir)
            .await;
        return JsObject::from([("CLAUDE_PROJECT_DIR".to_owned(), project.into())]);
    }
    let source: &dyn VariableSource = &run.substitution;
    exported_environment(VariableComponent::HookCommand, run.scope, Some(source)).await
}

/// Resolves one command hook. With `args` it is exec form: `command` and each element are
/// substituted as plain strings, `${user_config.*}` included, and `shell` is ignored. Without
/// `args` it is shell form: in a plugin's hook `${user_config.*}` anywhere in `command` is an error
/// and the hook does not run, whatever is configured (the check reads the template); the way out is
/// the `CLAUDE_PLUGIN_OPTION_<KEY>` variable in `environment`, or exec form. A command PowerShell
/// would only print does not start (`notStarted`).
pub async fn resolve_hook_command<L: Locations + Clone, E: Environment>(
    handler: &HookCommandHandler,
    scope: &SubstitutionScope<L, E>,
    options: &HookCommandOptions,
) -> HookCommandResolution {
    // One substituter for the command and the environment: the data folder is asked for once.
    let run = Run {
        scope,
        substitution: substituter(scope),
        options,
        base: options.path.clone().unwrap_or_else(|| "hook".to_owned()),
    };
    let plan = match &handler.args {
        None => shell_form(handler, &run).await,
        Some(args) => exec_form(handler, args, &run).await,
    };
    let Some(spec) = plan.spec else {
        return HookCommandResolution {
            resolved: None,
            not_started: plan.not_started,
            notice: plan.notice,
            issues: plan.issues,
        };
    };
    let exported = environment_of(&run).await;
    let shell = match &spec {
        CommandSpec::Shell { shell, .. } if scope.windows => Some(*shell),
        CommandSpec::Shell { .. } | CommandSpec::Exec { .. } => None,
    };
    let environment = match shell {
        None => exported,
        Some(shell) => shell_environment(shell, exported),
    };
    HookCommandResolution {
        resolved: Some(ResolvedHookCommand { spec, environment }),
        not_started: None,
        notice: None,
        issues: plan.issues,
    }
}

crate::registry::exports! {
    "loading/resolve/hook-command" {
        "resolveHookCommand" => async ok |handler: HookCommandHandler, scope: SubstitutionScope, options: Option<HookCommandOptions>|
            resolve_hook_command(&handler, &scope, &options.unwrap_or_default()).await;
    }
}
