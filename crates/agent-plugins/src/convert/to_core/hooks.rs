//! Port of `src/convert/to-core/hooks.ts`, switched.
//!
//! A core `hooks.json` from a Claude Code plugin's hooks (Agent Plugins Extended §18.4.3 item 4,
//! J18.17): only `command` hooks on the five shared events convert, each through the hook adapter,
//! limited to the profile its matcher selects. Hooks on the other events, the other handler types,
//! `if`, `async`, `asyncRewake`, `once` and `onFailure` are not converted; `statusMessage` is dropped
//! and reported. A hook naming `${user_config.*}`, or the core's own `${PLUGIN_ROOT}` and
//! `${PLUGIN_DATA}` (which a core client would expand in the adapter's arguments), is not converted.
//! A Claude Code hook this converter wrote around a core handler is unwrapped to that handler, and
//! its `PostToolUseFailure` twin is dropped quietly (judgment, spec/judgments.md). Positions are
//! those of the plugin's merged hooks.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::hook_matchers::read_matcher;
use crate::convert::adapters::registration::{
    ClaudeCodeRun, claude_code_adapter_handler, is_shell, unwrap_core_adapter,
};
use crate::convert::notes::{ConversionNote, not_converted, reported};
use crate::convert::placeholders::{holds_core_placeholder, to_core_placeholders};
use crate::convert::to_claude_code::hooks::adapter_matcher;
use crate::format::core::hooks::{CORE_HOOK_EVENTS, CoreHookEvent, CoreHookHandler, CoreHooks};
use crate::js::JsValue;

/// Where the converted hooks are written.
pub const CORE_HOOKS: &str = "hooks.json";
const SOURCE: &str = "hooks/hooks.json";

const REFUSED_FIELDS: [&str; 5] = ["if", "async", "asyncRewake", "once", "onFailure"];

fn core_event_of(value: &str) -> Option<CoreHookEvent> {
    CORE_HOOK_EVENTS
        .into_iter()
        .find(|event| event.as_str() == value)
}

/// A matcher group of Claude Code's hooks settings; each hook is read as written.
#[derive(Clone, Debug, Deserialize)]
pub struct HookGroup {
    /// The matcher.
    #[serde(default)]
    pub matcher: Option<String>,
    /// The hooks.
    pub hooks: Vec<JsValue>,
}

/// Claude Code's hooks settings, by event (`HooksSettings`).
pub type HooksSettings = IndexMap<String, Vec<HookGroup>>;

#[derive(Default)]
struct Converted {
    handler: Option<CoreHookHandler>,
    /// The handler runs through the adapter, which must then be written.
    adapted: bool,
    notes: Vec<ConversionNote>,
}

fn refused(field: &str, message: String) -> Converted {
    Converted {
        notes: vec![not_converted(
            SOURCE.to_owned(),
            Some(field.to_owned()),
            message,
        )],
        ..Converted::default()
    }
}

fn set(hook: &JsValue, key: &str) -> bool {
    hook.get(key).is_some_and(|value| !value.is_undefined())
}

fn command_of(hook: &JsValue) -> &str {
    hook.get("command")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

fn args_of(hook: &JsValue) -> Option<Vec<String>> {
    let list = hook.get("args").filter(|value| !value.is_undefined())?;
    Some(
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(JsValue::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

/// Why a command hook cannot run under the core, or `None`.
fn command_problem(hook: &JsValue) -> Option<String> {
    let fields: Vec<String> = REFUSED_FIELDS
        .into_iter()
        .filter(|key| set(hook, key))
        .map(|key| format!("\"{key}\""))
        .collect();
    if !fields.is_empty() {
        return Some(format!("{} has no core counterpart", fields.join(", ")));
    }
    let mut texts = vec![command_of(hook).to_owned()];
    texts.extend(args_of(hook).unwrap_or_default());
    if texts.iter().any(|text| text.contains("${user_config.")) {
        return Some("it names ${user_config.*}, which the core has no counterpart for".to_owned());
    }
    texts
        .iter()
        .any(|text| holds_core_placeholder(text))
        .then(|| {
            "it holds ${PLUGIN_ROOT} or ${PLUGIN_DATA}, which a core client would expand".to_owned()
        })
}

/// A core handler this converter wrapped, back as it was: registered exactly as it registers one.
fn unwrapped(
    event: CoreHookEvent,
    hook: &JsValue,
    matcher: Option<&str>,
) -> Option<CoreHookHandler> {
    let found = unwrap_core_adapter(hook)?;
    if found.event != event || matcher != adapter_matcher(event) {
        return None;
    }
    let args: Vec<String> = found
        .program
        .args
        .iter()
        .map(|arg| to_core_placeholders(arg))
        .collect();
    Some(CoreHookHandler {
        command: found.program.command,
        args: (!args.is_empty()).then_some(args),
        extensions: None,
    })
}

struct HookSite<'a> {
    event: CoreHookEvent,
    matcher: Option<&'a str>,
    field: String,
    adapter: bool,
}

fn notes(site: &HookSite<'_>, hook: &JsValue, narrowed: Option<&str>) -> Vec<ConversionNote> {
    let field = &site.field;
    let name = format!("hook \"{}\"", command_of(hook));
    let mut notes = Vec::new();
    if set(hook, "statusMessage") {
        notes.push(reported(
            SOURCE.to_owned(),
            Some(format!("{field}.statusMessage")),
            "\"statusMessage\" has no core counterpart: dropped".to_owned(),
        ));
    }
    if let Some(narrowed) = narrowed {
        notes.push(reported(
            SOURCE.to_owned(),
            Some(field.clone()),
            format!("{name}: {narrowed}"),
        ));
    }
    if site.event == CoreHookEvent::PostToolUse {
        notes.push(reported(
            SOURCE.to_owned(),
            Some(field.clone()),
            format!("{name} also runs on failed calls, which the core's PostToolUse includes"),
        ));
    }
    notes
}

/// The adapter handler of a command hook that converts, with what it reports.
fn adapted(site: &HookSite<'_>, hook: &JsValue) -> Converted {
    let reading = read_matcher(site.event, site.matcher);
    let problem = command_problem(hook)
        .or_else(|| (!site.adapter).then(|| "it needs a hook adapter (§18.4.4)".to_owned()))
        .or_else(|| reading.problem.clone());
    if let Some(problem) = problem {
        let message = format!("hook \"{}\" not converted: {problem}", command_of(hook));
        return refused(&site.field, message);
    }
    let args = args_of(hook);
    let handler = claude_code_adapter_handler(&ClaudeCodeRun {
        event: site.event,
        profile: reading.profile,
        timeout: hook.get("timeout").and_then(JsValue::as_f64),
        shell: hook
            .get("shell")
            .and_then(JsValue::as_str)
            .and_then(is_shell),
        exec: args.is_some(),
        command: command_of(hook).to_owned(),
        args: args.unwrap_or_default(),
    });
    Converted {
        handler: Some(handler),
        adapted: true,
        notes: notes(site, hook, reading.narrowed.as_deref()),
    }
}

fn convert_hook(site: &HookSite<'_>, hook: &JsValue) -> Converted {
    if let Some(original) = unwrapped(site.event, hook, site.matcher) {
        return Converted {
            handler: Some(original),
            ..Converted::default()
        };
    }
    if !hook
        .as_object()
        .is_some_and(|members| members.contains_key("command"))
    {
        let kind = hook
            .get("type")
            .and_then(JsValue::as_str)
            .unwrap_or("undefined");
        return refused(
            &site.field,
            format!("a \"{kind}\" hook has no core counterpart"),
        );
    }
    adapted(site, hook)
}

/// Whether a `PostToolUseFailure` hook is the twin of a `PostToolUse` adapter this converter wrote.
fn is_adapter_twin(hook: &JsValue) -> bool {
    unwrap_core_adapter(hook).is_some_and(|found| found.event == CoreHookEvent::PostToolUse)
}

fn event_parts(event: &str, groups: &[HookGroup], adapter: bool) -> Vec<(String, Converted)> {
    let mut parts = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        for (index, hook) in group.hooks.iter().enumerate() {
            let field = format!("hooks.{event}[{at}].hooks[{index}]");
            let converted = if let Some(core) = core_event_of(event) {
                let site = HookSite {
                    event: core,
                    matcher: group.matcher.as_deref(),
                    field,
                    adapter,
                };
                convert_hook(&site, hook)
            } else if event == "PostToolUseFailure"
                && group.matcher.is_none()
                && is_adapter_twin(hook)
            {
                Converted::default()
            } else {
                refused(&field, format!("hooks on {event} have no core counterpart"))
            };
            parts.push((event.to_owned(), converted));
        }
    }
    parts
}

/// The core hooks, whether the adapter is needed, and the notes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoreHooksPart {
    /// The handlers, by event.
    pub hooks: CoreHooks,
    /// At least one handler runs through the adapter.
    pub adapted: bool,
    /// What the conversion says.
    pub notes: Vec<ConversionNote>,
}

/// A core `hooks.json` from a Claude Code plugin's hooks.
#[must_use]
pub fn core_hooks_of(hooks: &HooksSettings, adapter: bool) -> CoreHooksPart {
    let parts: Vec<(String, Converted)> = hooks
        .iter()
        .flat_map(|(event, groups)| event_parts(event, groups, adapter))
        .collect();
    let mut kept: CoreHooks = IndexMap::new();
    for event in CORE_HOOK_EVENTS {
        let handlers: Vec<CoreHookHandler> = parts
            .iter()
            .filter(|(at, _)| at == event.as_str())
            .filter_map(|(_, part)| part.handler.clone())
            .collect();
        if !handlers.is_empty() {
            kept.insert(event.as_str().to_owned(), handlers);
        }
    }
    CoreHooksPart {
        hooks: kept,
        adapted: parts.iter().any(|(_, part)| part.adapted),
        notes: parts.into_iter().flat_map(|(_, part)| part.notes).collect(),
    }
}

crate::registry::exports! {
    "convert/to-core/hooks" {
        "CORE_HOOKS" => ok || CORE_HOOKS;
        "coreHooksOf" => ok |hooks: HooksSettings, adapter: bool| core_hooks_of(&hooks, adapter);
    }
}
