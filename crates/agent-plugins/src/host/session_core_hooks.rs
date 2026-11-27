//! Ported from `src/host/session-core-hooks.ts`; only the core uses it now, so it has no wrapper.
//!
//! The core side of one hook occurrence in a session (Agent Plugins Extended §18.1.2, core §7.3.3):
//! every handler of the event across the enabled core packages, in the order they loaded (J18.3),
//! each a fresh process with the core input on stdin. Core handlers are plugin hooks to the hook
//! policy (§18.1.2 item 5, J18.15): `disableAllHooks` turns them off, `allowManagedHooksOnly` keeps
//! only those of a package force-enabled in managed `enabledPlugins`, and a plugin-only lock leaves
//! them alone. Their environment is a Claude Code plugin hook's base (§3.3) with `CLAUDE_PROJECT_DIR`
//! and without `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` or `CLAUDE_PLUGIN_OPTION_<KEY>` (J18.12),
//! then `PLUGIN_ROOT` and `PLUGIN_DATA` (core §9.1).
//!
//! The packages are the state's (`Own<HostState>`): each `ActivePlugin` carries its core package,
//! so nothing finds a package by the host's object.

use indexmap::IndexMap;
use serde::Deserialize;

use super::core_servers::{CorePlace, core_paths_of};
use super::state::{ActivePlugin, HostState};
use crate::format::core::hooks::CoreHookEvent;
use crate::format::core::subprocess::{CoreHookHandler, CoreProcessInput, core_handler_process};
use crate::format::path_text::PathPlatform;
use crate::hooks::contracts::HookSession;
use crate::hooks::core::combine::CoreSide;
use crate::hooks::core::run::{CoreInvocation, run_core_handler};
use crate::hooks::core::types::{CoreOccurrence, CoreResult, CoreRun};
use crate::hooks::output::reminder_tags::escape_reminder_tags;
use crate::hooks::output::text_cap::{Limited, limit_text};
use crate::js::JsValue;
use crate::js::promise_all;
use crate::js::value::to_js_value;
use crate::ports::locations::Locations;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Callback, Own, Port};

/// The ports a core handler's run reads (`HookPorts`).
#[derive(Debug, Deserialize)]
pub struct CorePorts {
    /// Runs the processes.
    pub commands: Port,
    /// The session's locations.
    pub locations: Port,
    /// The session's environment.
    pub environment: Port,
    /// Where a long context is saved.
    #[serde(default)]
    pub scratch: Option<Port>,
}

/// What a session's core side reads.
#[derive(Debug, Deserialize)]
pub struct CoreHandlerUse {
    /// The state the session reads now.
    pub state: Own<HostState>,
    /// The hook session of the turn.
    pub session: HookSession,
    /// The session's ports.
    pub ports: CorePorts,
    /// The base environment of a plugin hook's process in this session
    /// (`subprocessEnvironment('hook')`), a function of the host's.
    pub base: Callback,
}

/// One handler that runs: its package and its place in the event's list.
pub struct Selected<'a> {
    plugin: &'a ActivePlugin,
    handler: CoreHookHandler,
    index: usize,
}

/// Whether the hook policy lets this package's handlers run (the gates of a plugin's hooks).
fn allowed(plugin: &ActivePlugin, session: &HookSession) -> bool {
    let gates = &session.disable_all_hooks;
    if gates.managed_locked || gates.effective {
        return false;
    }
    session.allow_managed_hooks_only != Some(true)
        || session
            .force_enabled_plugins
            .as_ref()
            .is_some_and(|ids| ids.members.iter().any(|id| id == plugin.id()))
}

/// The handlers that run on this event, in package load order.
#[must_use]
pub fn selected_core_handlers(use_: &CoreHandlerUse, event: CoreHookEvent) -> Vec<Selected<'_>> {
    let mut selected = Vec::new();
    for plugin in &use_.state.active {
        let Some(core) = plugin.core() else { continue };
        if !allowed(plugin, &use_.session) {
            continue;
        }
        let handlers = core
            .get("hooks")
            .and_then(|hooks| hooks.get(event.as_str()));
        let handlers = handlers
            .and_then(|list| Vec::<CoreHookHandler>::deserialize(list.clone()).ok())
            .unwrap_or_default();
        selected.extend(
            handlers
                .into_iter()
                .enumerate()
                .map(|(index, handler)| Selected {
                    plugin,
                    handler,
                    index,
                }),
        );
    }
    selected
}

/// `^CLAUDE_PLUGIN_(ROOT|DATA|OPTION_.*)$`, case-insensitive on Windows: `.` takes no line
/// terminator, as JavaScript's does.
fn is_claude_plugin(name: &str, any_case: bool) -> bool {
    const PREFIX: &str = "CLAUDE_PLUGIN_";
    let Some(head) = name.get(..PREFIX.len()) else {
        return false;
    };
    let same = |left: &str, right: &str| {
        if any_case {
            left.eq_ignore_ascii_case(right)
        } else {
            left == right
        }
    };
    if !same(head, PREFIX) {
        return false;
    }
    let rest = name.get(PREFIX.len()..).unwrap_or_default();
    if same(rest, "ROOT") || same(rest, "DATA") {
        return true;
    }
    rest.get(..7).is_some_and(|option| same(option, "OPTION_"))
        && !rest
            .chars()
            .any(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn windows(use_: &CoreHandlerUse) -> bool {
    use_.state.options.windows.unwrap_or(false)
}

/// The base of a core handler's process: no `CLAUDE_PLUGIN_*`, with `CLAUDE_PROJECT_DIR` (J18.12).
async fn core_base(use_: &CoreHandlerUse) -> IndexMap<String, String> {
    let any_case = windows(use_);
    let given: IndexMap<String, String> = use_.base.call(Vec::new()).await;
    let mut kept: IndexMap<String, String> = given
        .into_iter()
        .filter(|(name, _)| !is_claude_plugin(name, any_case))
        .collect();
    let project = use_.ports.locations.project_dir().await;
    kept.insert("CLAUDE_PROJECT_DIR".to_owned(), project);
    kept
}

async fn invoke(
    use_: &CoreHandlerUse,
    selected: &Selected<'_>,
    occurrence: &CoreOccurrence,
    signal: &Signal,
) -> CoreRun {
    let Selected {
        plugin,
        handler,
        index,
    } = selected;
    let place = CorePlace {
        locations: use_.ports.locations,
        environment: use_.ports.environment,
        signal: Some(signal.clone()),
    };
    let paths = core_paths_of(plugin.id(), &place).await;
    let platform = if windows(use_) {
        PathPlatform::Windows
    } else {
        PathPlatform::Posix
    };
    let input = CoreProcessInput {
        paths,
        base: Some(core_base(use_).await),
        platform: Some(platform),
    };
    let launch = core_handler_process(handler, &input);
    let mut label = vec![handler.command.clone()];
    label.extend(handler.args.clone().unwrap_or_default());
    let invocation = CoreInvocation {
        handler_id: format!("core:{}:{}:{index}", plugin.id(), occurrence.event.as_str()),
        plugin_id: plugin.id().to_owned(),
        event: occurrence.event,
        process: launch,
        label: label.join(" "),
        input: occurrence.input.clone(),
    };
    run_core_handler(&use_.ports.commands, &invocation, signal.clone()).await
}

/// J18.5: a context string is capped, saved and escaped as a Claude Code hook's is (§7.5.3).
async fn contexts_of(
    runs: &[CoreRun],
    use_: &CoreHandlerUse,
    signal: &Signal,
) -> (Vec<String>, Vec<JsValue>) {
    let mut limits: Vec<BoxFuture<'_, Limited>> = Vec::new();
    for run in runs {
        let Some(CoreResult::Context { context }) = &run.result else {
            continue;
        };
        let name = format!("{}.additional_context", run.handler_id);
        let scratch = use_.ports.scratch.as_ref();
        limits.push(Box::pin(async move {
            limit_text(&name, context, scratch, signal).await
        }));
    }
    let limited = promise_all(limits).await;
    let contexts = limited
        .iter()
        .map(|one| escape_reminder_tags(&one.text))
        .collect();
    let issues = limited
        .iter()
        .flat_map(|one| {
            one.issues
                .iter()
                .filter_map(|issue| to_js_value(issue).ok())
        })
        .collect();
    (contexts, issues)
}

/// Runs the selected handlers concurrently (core §7.3.3 lets the client) and waits for all of them,
/// a failure included, before the occurrence is acted on (core §7.3.5, §7.3.7).
pub async fn run_core_side(
    use_: &CoreHandlerUse,
    occurrence: CoreOccurrence,
    signal: &Signal,
) -> CoreSide {
    let selected = selected_core_handlers(use_, occurrence.event);
    let started: Vec<BoxFuture<'_, CoreRun>> = selected
        .iter()
        .map(|one| Box::pin(invoke(use_, one, &occurrence, signal)) as BoxFuture<'_, CoreRun>)
        .collect();
    let runs = promise_all(started).await;
    let (contexts, issues) = contexts_of(&runs, use_, signal).await;
    CoreSide {
        occurrence,
        runs,
        contexts,
        issues,
    }
}
