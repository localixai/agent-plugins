//! Ported from `src/convert/to-claude-code/hooks.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hooks/hooks.json` from a core `hooks.json` (Agent Plugins Extended §18.4.2 item 4, J18.17): every
//! core handler runs through the hook adapter, one exec-form `command` hook each, registered as
//! §18.3.4 item 3 says: a `SessionStart` handler with the matcher `startup|clear|fork`, a
//! `PostToolUse` handler on `PostToolUse` and on `PostToolUseFailure`, the others on the event of the
//! same name, with no matcher. `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` in `args` become Claude Code's
//! names; a handler with any other `${` in its `args`, which Claude Code would substitute (§18.4.5), is
//! not converted. A handler this converter wrote around a Claude Code hook is unwrapped to that hook.
//! A `UserPromptSubmit` handler is listed as also running on the turns the adapter cannot tell from an
//! external submission (§18.4.4 item 5, J18.23).

use indexmap::IndexMap;
use serde::Serialize;

use crate::convert::adapters::registration::{
    ClaudeCodeRun, CommandHook, HandlerProgram, Shell, core_adapter_hook,
    unwrap_claude_code_adapter,
};
use crate::convert::notes::{ConversionNote, not_converted};
use crate::convert::placeholders::{holds_other_reference, to_claude_code_placeholders};
use crate::convert::profiles::matcher_of;
use crate::format::core::hooks::{CORE_HOOK_EVENTS, CoreHookEvent, CoreHookHandler, CoreHooks};

/// Where the converted hooks are written.
pub const CLAUDE_HOOKS: &str = "hooks/hooks.json";
const SOURCE: &str = "hooks.json";
const NEW_SESSIONS: &str = "startup|clear|fork";

/// Where each core event's handler is registered in Claude Code, with the matcher each needs.
const fn registrations(event: CoreHookEvent) -> &'static [(&'static str, Option<&'static str>)] {
    match event {
        CoreHookEvent::SessionStart => &[("SessionStart", Some(NEW_SESSIONS))],
        CoreHookEvent::UserPromptSubmit => &[("UserPromptSubmit", None)],
        CoreHookEvent::PreToolUse => &[("PreToolUse", None)],
        CoreHookEvent::PostToolUse => &[("PostToolUse", None), ("PostToolUseFailure", None)],
        CoreHookEvent::Stop => &[("Stop", None)],
    }
}

/// The matcher a hook adapter of this converter is registered with on a core event's own event.
#[must_use]
pub fn adapter_matcher(event: CoreHookEvent) -> Option<&'static str> {
    registrations(event)
        .first()
        .and_then(|(_, matcher)| *matcher)
}

/// A Claude Code hook the adapter ran under the core, back in its own form.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnwrappedHook {
    /// Always `command`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// The command.
    pub command: String,
    /// Its arguments, in exec form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Its timeout.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_timeout"
    )]
    pub timeout: Option<f64>,
    /// Its shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Shell>,
}

fn serialize_timeout<S: serde::Serializer>(
    timeout: &Option<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match timeout {
        Some(number) => crate::js::value::serialize_number(number, serializer),
        None => serializer.serialize_none(),
    }
}

/// A hook of a group.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Hook {
    /// The adapter around a core handler.
    Adapter(CommandHook),
    /// A Claude Code hook unwrapped.
    Unwrapped(UnwrappedHook),
}

/// A matcher group of one hook.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HookMatcherGroup {
    /// The matcher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The hook.
    pub hooks: Vec<Hook>,
}

struct Placed {
    event: String,
    group: HookMatcherGroup,
}

fn group(hook: Hook, matcher: Option<String>) -> HookMatcherGroup {
    HookMatcherGroup {
        matcher,
        hooks: vec![hook],
    }
}

/// A Claude Code hook the adapter ran under the core, back in its own place.
fn unwrapped(run: ClaudeCodeRun) -> Placed {
    let hook = UnwrappedHook {
        kind: "command",
        command: run.command,
        args: run.exec.then_some(run.args),
        timeout: run.timeout,
        shell: run.shell,
    };
    let matcher = match run.profile {
        None => adapter_matcher(run.event).map(str::to_owned),
        Some(profile) => Some(matcher_of(profile)),
    };
    Placed {
        event: run.event.as_str().to_owned(),
        group: group(Hook::Unwrapped(hook), matcher),
    }
}

struct HandlerPart {
    placed: Vec<Placed>,
    adapted: bool,
    notes: Vec<ConversionNote>,
}

fn not_placed(field: &str, message: String) -> HandlerPart {
    HandlerPart {
        placed: Vec::new(),
        adapted: false,
        notes: vec![not_converted(
            SOURCE.to_owned(),
            Some(field.to_owned()),
            message,
        )],
    }
}

fn handler_part(
    event: CoreHookEvent,
    handler: &CoreHookHandler,
    field: &str,
    adapter: bool,
) -> HandlerPart {
    if let Some(run) = unwrap_claude_code_adapter(handler).filter(|run| run.event == event) {
        return HandlerPart {
            placed: vec![unwrapped(run)],
            adapted: false,
            notes: Vec::new(),
        };
    }
    let args = handler.args.clone().unwrap_or_default();
    let mut words = vec![handler.command.clone()];
    words.extend(args.iter().cloned());
    let label = words.join(" ");
    if !adapter {
        return not_placed(
            field,
            format!(
                "handler \"{label}\" not converted: a core handler needs a hook adapter (§18.4.4)"
            ),
        );
    }
    if args.iter().any(|arg| holds_other_reference(arg, "core")) {
        return not_placed(
            field,
            format!(
                "handler \"{label}\" not converted: its args hold \"${{\" text, which Claude Code would substitute"
            ),
        );
    }
    let hook = core_adapter_hook(
        event,
        &HandlerProgram {
            command: handler.command.clone(),
            args: args
                .iter()
                .map(|arg| to_claude_code_placeholders(arg))
                .collect(),
        },
    );
    let placed = registrations(event)
        .iter()
        .map(|(at, matcher)| Placed {
            event: (*at).to_owned(),
            group: group(Hook::Adapter(hook.clone()), matcher.map(str::to_owned)),
        })
        .collect();
    HandlerPart {
        placed,
        adapted: true,
        notes: over_firing(event, field, &label),
    }
}

/// What an adapted handler does under Claude Code that its core event does not (J18.23).
fn over_firing(event: CoreHookEvent, field: &str, label: &str) -> Vec<ConversionNote> {
    if event != CoreHookEvent::UserPromptSubmit {
        return Vec::new();
    }
    vec![not_converted(
        SOURCE.to_owned(),
        Some(field.to_owned()),
        format!(
            "handler \"{label}\" also runs on task notifications and automatic continuations: Claude Code gives them the source \"system\" of external turns (§18.4.4 item 5)"
        ),
    )]
}

fn extension_notes(handler: &CoreHookHandler, field: &str) -> Vec<ConversionNote> {
    handler
        .extensions
        .iter()
        .flat_map(|extensions| extensions.keys())
        .map(|namespace| {
            not_converted(
                SOURCE.to_owned(),
                Some(format!("{field}.extensions.{namespace}")),
                "client extension data has no Claude Code counterpart".to_owned(),
            )
        })
        .collect()
}

/// `hooks/hooks.json`'s hooks, whether the adapter is needed, and the notes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HooksPart {
    /// The hooks, by event.
    pub hooks: IndexMap<String, Vec<HookMatcherGroup>>,
    /// At least one handler runs through the adapter, which must then be written.
    pub adapted: bool,
    /// What the conversion says.
    pub notes: Vec<ConversionNote>,
}

/// `hooks/hooks.json` from a core `hooks.json`.
#[must_use]
pub fn claude_code_hooks(hooks: &CoreHooks, adapter: bool) -> HooksPart {
    let mut by_event: IndexMap<String, Vec<HookMatcherGroup>> = IndexMap::new();
    let mut adapted = false;
    let mut notes = Vec::new();
    for event in CORE_HOOK_EVENTS {
        for (index, handler) in hooks.get(event.as_str()).into_iter().flatten().enumerate() {
            let field = format!("hooks.{}[{index}]", event.as_str());
            let part = handler_part(event, handler, &field, adapter);
            for placed in part.placed {
                by_event.entry(placed.event).or_default().push(placed.group);
            }
            adapted |= part.adapted;
            notes.extend(part.notes);
            notes.extend(extension_notes(handler, &field));
        }
    }
    HooksPart {
        hooks: by_event,
        adapted,
        notes,
    }
}
