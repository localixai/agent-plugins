//! Port of `src/convert/adapters/registration.ts`, switched.
//!
//! How each target's hooks file names a hook adapter (Agent Plugins Extended §18.4.4), and how a
//! converter tells its own adapter apart when it converts back: an adapter registration of exactly
//! this shape is unwrapped to the handler it runs (judgment, spec/judgments.md), so a converted
//! package converts back to the hooks it had instead of an adapter around an adapter.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::sources::ADAPTER_RUNTIME;
use crate::convert::profiles::{PROFILE_NAMES, ProfileName, name_of};
use crate::format::core::hooks::{CORE_HOOK_EVENTS, CoreHookEvent, CoreHookHandler};
use crate::js::{JsValue, number, number_to_string};

/// The adapter that runs a core handler under Claude Code.
pub const RUN_CORE_HANDLER: &str = "hooks/run-core-handler.mjs";
/// The adapter that runs a Claude Code hook under the core.
pub const RUN_CLAUDE_CODE_HANDLER: &str = "hooks/run-claude-code-handler.mjs";

const CORE_ADAPTER_ARG: &str = "${CLAUDE_PLUGIN_ROOT}/hooks/run-core-handler.mjs";
const CLAUDE_CODE_ADAPTER_ARG: &str = "${PLUGIN_ROOT}/hooks/run-claude-code-handler.mjs";
const END: &str = "--";

fn is_core_event(value: Option<&str>) -> Option<CoreHookEvent> {
    CORE_HOOK_EVENTS
        .into_iter()
        .find(|event| Some(event.as_str()) == value)
}

/// A core handler's program: its command and arguments, as the form that runs it writes them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandlerProgram {
    /// The command.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
}

/// A Claude Code `command` hook in exec form (`HookHandlerOf<'command'>`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandHook {
    /// Always `command`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// The program.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
}

/// The Claude Code hook that runs a core handler; `args` already use Claude Code's placeholders.
#[must_use]
pub fn core_adapter_hook(event: CoreHookEvent, program: &HandlerProgram) -> CommandHook {
    let mut args = vec![
        CORE_ADAPTER_ARG.to_owned(),
        event.as_str().to_owned(),
        END.to_owned(),
        program.command.clone(),
    ];
    args.extend(program.args.iter().cloned());
    CommandHook {
        kind: "command",
        command: ADAPTER_RUNTIME.to_owned(),
        args,
    }
}

const ONLY_ADAPTER_FIELDS: [&str; 3] = ["type", "command", "args"];

/// The core handler a hook runs, with its event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnwrappedCore {
    /// The event it runs on.
    pub event: CoreHookEvent,
    /// What it runs.
    pub program: HandlerProgram,
}

/// The core handler a Claude Code hook of this converter runs, or `None` for any other hook.
#[must_use]
pub fn unwrap_core_adapter(hook: &JsValue) -> Option<UnwrappedCore> {
    let members = hook.as_object()?;
    if !members
        .keys()
        .all(|key| ONLY_ADAPTER_FIELDS.contains(&key.as_str()))
    {
        return None;
    }
    if !members.contains_key("args") || hook.get("command")?.as_str() != Some(ADAPTER_RUNTIME) {
        return None;
    }
    let list: Vec<&str> = hook
        .get("args")?
        .as_array()?
        .iter()
        .map(JsValue::as_str)
        .collect::<Option<_>>()?;
    let mut items = list.into_iter();
    let (adapter, event, end, command) = (items.next(), items.next(), items.next(), items.next());
    let event = is_core_event(event);
    if adapter != Some(CORE_ADAPTER_ARG) || end != Some(END) {
        return None;
    }
    let event = event?;
    let command = command?;
    Some(UnwrappedCore {
        event,
        program: HandlerProgram {
            command: command.to_owned(),
            args: items.map(str::to_owned).collect(),
        },
    })
}

/// A hook's shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    /// `bash`.
    Bash,
    /// `powershell`.
    Powershell,
}

const SHELLS: [Shell; 2] = [Shell::Bash, Shell::Powershell];

impl Shell {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Powershell => "powershell",
        }
    }
}

/// What a Claude Code `command` hook is made of, as the adapter that runs it under the core takes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeRun {
    /// The event.
    pub event: CoreHookEvent,
    /// Run only on calls of this profile: the matcher selected its tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// The hook's timeout.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_timeout"
    )]
    pub timeout: Option<f64>,
    /// The hook's shell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<Shell>,
    /// Exec form: the hook had `args`.
    pub exec: bool,
    /// The command.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
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

fn flag(name: &str, value: Option<String>) -> Vec<String> {
    value.map_or_else(Vec::new, |text| vec![name.to_owned(), text])
}

/// The core handler that runs a Claude Code hook through the adapter.
#[must_use]
pub fn claude_code_adapter_handler(run: &ClaudeCodeRun) -> CoreHookHandler {
    let mut args = vec![CLAUDE_CODE_ADAPTER_ARG.to_owned()];
    args.extend(flag("--event", Some(run.event.as_str().to_owned())));
    args.extend(flag(
        "--profile",
        run.profile.map(|profile| name_of(profile).to_owned()),
    ));
    args.extend(flag("--timeout", run.timeout.map(number_to_string)));
    args.extend(flag(
        "--shell",
        run.shell.map(|shell| shell.as_str().to_owned()),
    ));
    if run.exec {
        args.push("--exec".to_owned());
    }
    args.push(END.to_owned());
    args.push(run.command.clone());
    args.extend(run.args.iter().cloned());
    CoreHookHandler {
        command: ADAPTER_RUNTIME.to_owned(),
        args: Some(args),
        extensions: None,
    }
}

pub(crate) fn is_shell(value: &str) -> Option<Shell> {
    SHELLS.into_iter().find(|shell| shell.as_str() == value)
}

const VALUED: [&str; 4] = ["--event", "--profile", "--timeout", "--shell"];

/// The flags of an adapter handler, by name; `None` for a flag list this converter never writes.
fn flags_of(flags: &[String]) -> Option<IndexMap<String, String>> {
    let mut found = IndexMap::new();
    let mut at = 0;
    while at < flags.len() {
        let name = flags.get(at).cloned().unwrap_or_default();
        if name == "--exec" {
            found.insert(name, String::new());
        } else if VALUED.contains(&name.as_str()) && at + 1 < flags.len() {
            found.insert(name, flags.get(at + 1).cloned().unwrap_or_default());
            at += 1;
        } else {
            return None;
        }
        at += 1;
    }
    Some(found)
}

/// The optional flags read back: `profile`, `timeout`, `shell`.
#[derive(Default)]
struct Options {
    profile: Option<ProfileName>,
    timeout: Option<f64>,
    shell: Option<Shell>,
}

/// Each optional flag read back, or `None` when its value is not one this converter writes.
fn options_of(flags: &IndexMap<String, String>) -> Option<Options> {
    let mut options = Options::default();
    if let Some(value) = flags.get("--profile") {
        options.profile = Some(
            PROFILE_NAMES
                .into_iter()
                .find(|profile| name_of(*profile) == value)?,
        );
    }
    if let Some(value) = flags.get("--timeout") {
        let timeout = number(value);
        if timeout.is_nan() || timeout <= 0.0 {
            return None;
        }
        options.timeout = Some(timeout);
    }
    if let Some(value) = flags.get("--shell") {
        options.shell = Some(is_shell(value)?);
    }
    Some(options)
}

/// The flags and the hook's command and arguments of an adapter handler's `args`.
struct Parts {
    flags: IndexMap<String, String>,
    command: String,
    args: Vec<String>,
}

fn parts_of(args: &[String]) -> Option<Parts> {
    let end = args.iter().position(|arg| arg == END)?;
    if args.first().map(String::as_str) != Some(CLAUDE_CODE_ADAPTER_ARG) {
        return None;
    }
    let flags = flags_of(args.get(1..end).unwrap_or_default());
    let command = args.get(end + 1).cloned();
    Some(Parts {
        flags: flags?,
        command: command?,
        args: args.get(end + 2..).unwrap_or_default().to_vec(),
    })
}

/// The Claude Code hook a core handler of this converter runs, or `None` for any other handler.
#[must_use]
pub fn unwrap_claude_code_adapter(handler: &CoreHookHandler) -> Option<ClaudeCodeRun> {
    if handler.command != ADAPTER_RUNTIME {
        return None;
    }
    let parts = parts_of(handler.args.as_deref().unwrap_or_default())?;
    let event = is_core_event(parts.flags.get("--event").map(String::as_str));
    let options = options_of(&parts.flags)?;
    Some(ClaudeCodeRun {
        event: event?,
        profile: options.profile,
        timeout: options.timeout,
        shell: options.shell,
        exec: parts.flags.contains_key("--exec"),
        command: parts.command,
        args: parts.args,
    })
}

crate::registry::exports! {
    "convert/adapters/registration" {
        "RUN_CLAUDE_CODE_HANDLER" => ok || RUN_CLAUDE_CODE_HANDLER;
    }
}
