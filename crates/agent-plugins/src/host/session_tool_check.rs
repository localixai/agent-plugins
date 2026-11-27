//! Port of `src/host/session-tool-check.ts`, switched.
//!
//! Whether a tool call may run: the PreToolUse hooks, then the permission decision over the input
//! they return. The session's hook part is the host's (`hooks.dispatch`, a port's method), and the
//! tool names are the core's own handle; the permission context's three functions cross beside its
//! data, as for `decide`.

use serde::{Deserialize, Serialize};

use crate::hooks::output::pre_tool_use_outcome::{
    PreToolUseOutcomeInput, pre_tool_use_hook_outcome,
};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::Signal;
use crate::ports::failure::PortFailure;
use crate::registry::{Callback, Own, Port};
use crate::rules::decide::context::{DecideContext, PreToolUseHookOutcome};
use crate::rules::decide::decide::decide;
use crate::rules::path::symlink::{Prepared, prepare_call, resolve_rule_directories};
use crate::rules::tools::host_tool_names::ToolNames;
use crate::rules::types::{Decision, DecisionReason, PreparedCall};

/// What `checkTool` found for one tool call.
#[derive(Clone, Debug, Serialize)]
pub struct ToolCheck {
    /// What the PreToolUse hooks answered: their messages, context and effects.
    pub hook: JsValue,
    /// The input to run: the hook's `updatedInput`, unless the hook deferred or returned none.
    pub input: JsValue,
    /// The permission decision over that input, with the hook's answer applied.
    pub decision: Decision,
    /// Why the path of that input could not be resolved (symlinks that form a loop): "the Read,
    /// Edit, and Write tools refuse the operation" (permissions.md, "Paths that can't be resolved
    /// or that change"), so `decision` is a deny for `unresolvable-path`. Only with a `resolver`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<PortFailure>,
}

/// The options of one check besides the permission context.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckOptions {
    /// The call's id, for the hooks.
    pub tool_use_id: String,
    /// The turn's signal.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// The host's path resolver, for the symlink check.
    #[serde(default)]
    pub resolver: Option<Port>,
}

/// The host's resolver and the signal it is asked with.
struct Resolving<'a> {
    resolver: &'a Port,
    signal: Option<&'a Signal>,
}

/// The context with the real locations of the rules' and the working directories, unless the host
/// gave them.
async fn with_real_directories(context: &mut DecideContext, resolving: &Resolving<'_>) {
    let paths = &context.match_context.paths;
    if paths.real_directories.is_some() {
        return;
    }
    let real =
        resolve_rule_directories(&context.rules, paths, resolving.resolver, resolving.signal).await;
    context.match_context.paths.real_directories = Some(real);
}

/// The call that runs and the context it is decided in, with the symlink check done when a
/// resolver is given.
async fn prepared_run(
    mut call: JsValue,
    input: JsValue,
    context: &mut DecideContext,
    resolving: Option<Resolving<'_>>,
) -> Prepared {
    if let JsValue::Object(members) = &mut call {
        members.insert("input".to_owned(), input);
        if resolving.is_some() {
            members.shift_remove("resolvedPaths");
        }
    }
    let Some(resolving) = resolving else {
        return Prepared::Call(call);
    };
    let paths = context.match_context.paths.clone();
    let run = prepare_call(call, &paths, resolving.resolver, resolving.signal).await;
    with_real_directories(context, &resolving).await;
    run
}

/// The decide context's reading of the hook's answer (`preToolUseHookOutcome`).
fn hook_outcome(hook: &JsValue) -> Option<PreToolUseHookOutcome> {
    let read = PreToolUseOutcomeInput::deserialize(hook.clone()).ok()?;
    let outcome = to_js_value(&pre_tool_use_hook_outcome(&read)).ok()?;
    PreToolUseHookOutcome::deserialize(outcome).ok()
}

/// One check (`ToolCheckPart.checkTool`). `names` reads the call's tool name in Claude Code's
/// terms (`SessionOptions.toolNames`).
pub async fn check_tool(
    hooks: &Port,
    names: &ToolNames,
    given: JsValue,
    options: CheckOptions,
    mut context: DecideContext,
) -> Result<ToolCheck, String> {
    let tool_name = given.get("toolName").cloned().unwrap_or_default();
    let given_input = given.get("input").cloned().unwrap_or_default();
    // The dispatch reads the host's name itself; the rules decide the Claude Code name.
    let facts = JsValue::Object(
        [
            ("tool_name".to_owned(), tool_name.clone()),
            ("tool_input".to_owned(), given_input.clone()),
            ("tool_use_id".to_owned(), options.tool_use_id.clone().into()),
        ]
        .into_iter()
        .collect(),
    );
    let mut asked = crate::js::JsObject::new();
    if let Some(signal) = options.signal.as_ref().and_then(|s| to_js_value(s).ok()) {
        asked.insert("signal".to_owned(), signal);
    }
    let arguments = vec!["PreToolUse".into(), facts, JsValue::Object(asked)];
    let hook = hooks.method::<JsValue>("dispatch", arguments).await;
    // The merge already drops `updatedInput` after a deny or a defer (hooks/output).
    let input = match hook
        .get("effects")
        .and_then(|effects| effects.get("updatedInput"))
    {
        None | Some(JsValue::Undefined | JsValue::Null) => given_input,
        Some(updated) => updated.clone(),
    };
    // The rules decide the Claude Code name of the call that runs, read from the input it runs with.
    let host_name = tool_name.as_str().unwrap_or_default().to_owned();
    let claude_name = names.claude_name(&host_name, input.clone()).await;
    let mut call = given;
    if let JsValue::Object(members) = &mut call {
        members.insert("toolName".to_owned(), claude_name.into());
    }
    let resolving = options.resolver.as_ref().map(|resolver| Resolving {
        resolver,
        signal: options.signal.as_ref(),
    });
    let run = prepared_run(call, input.clone(), &mut context, resolving).await;
    let run = match run {
        Prepared::Failure(refused) => {
            let decision = Decision::Deny {
                reason: DecisionReason::UnresolvablePath,
            };
            return Ok(ToolCheck {
                hook,
                input,
                decision,
                refused: Some(refused),
            });
        }
        Prepared::Call(run) => run,
    };
    context.hook = hook_outcome(&hook);
    let run = PreparedCall::deserialize(run).map_err(|error| error.to_string())?;
    let decision = decide(&run, &context).await?;
    Ok(ToolCheck {
        hook,
        input,
        decision,
        refused: None,
    })
}

crate::registry::exports! {
    "host/session-tool-check" {
        "checkTool" => async fallible |hooks: Port, names: Own<ToolNames>, given: JsValue, options: CheckOptions, context: DecideContext, requires_permission: Option<Callback>, preapproved_fetch_domain: Option<Callback>, foreign_artifact: Option<Callback>| {
            let mut context = context;
            context.requires_permission = requires_permission;
            context.preapproved_fetch_domain = preapproved_fetch_domain;
            context.foreign_artifact = foreign_artifact;
            check_tool(&hooks, &names, given, options, context).await
        };
    }
}
