//! The core's counterpart of `src/hooks/select/matcher-targets.ts` at sha256:0be1b4d5b420, which stays
//! TypeScript: its table holds a reader function per event, which no value crossing the boundary
//! can carry (docs/RUST.md, "What stays TypeScript"). Here each event's target is a row of data
//! read by the same rules: what the matcher is tested against, the tool call a tool event carries
//! and whether the event's hooks have the session's MCP servers.
//!
//! Which input field an event's matcher filters on (hooks.md, "Matcher patterns", the table "Each
//! event type matches on a different field"), or none: "If you add a `matcher` field to an event
//! without matcher support, it is silently ignored".

use crate::hooks::contracts::ToolCallView;
use crate::js::JsValue;
use crate::registry::Port;
use crate::rules::hook_matcher::MatcherMode;

use super::model_switch::canonical_matcher_model;

/// What a matcher is tested against: a value (possibly absent), or nothing to filter on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatcherValue {
    /// `{ filters: true, value }`.
    Filters(Option<String>),
    /// `{ filters: false }`.
    NoFilter,
}

/// How an event's target reads its facts.
#[derive(Clone, Copy, Debug)]
enum Target {
    /// The five tool events filter on the tool name, and carry the call a handler's `if` reads.
    Tool,
    /// The model events filter on the canonical name of `to_model`; with none, every hook runs.
    Model,
    /// An event that filters on one field of its input, in a matcher mode (`byField`). `narrow` is
    /// for `FileChanged` and `StopFailure`, which accept only letters, digits, `_` and `|`.
    Field(&'static str, MatcherMode),
    /// `FileChanged`: the last path segment of its field, whichever separator the host's paths use.
    Basename(&'static str),
    /// An event with no matcher support: it fires on every occurrence.
    None,
}

const STANDARD: MatcherMode = MatcherMode::Standard;

const MATCHER_TARGETS: [(&str, Target); 33] = [
    ("PreToolUse", Target::Tool),
    ("PostToolUse", Target::Tool),
    ("PostToolUseFailure", Target::Tool),
    ("PermissionRequest", Target::Tool),
    ("PermissionDenied", Target::Tool),
    ("SessionStart", Target::Field("source", STANDARD)),
    ("Setup", Target::Field("trigger", STANDARD)),
    ("SessionEnd", Target::Field("reason", STANDARD)),
    ("Notification", Target::Field("notification_type", STANDARD)),
    ("SubagentStart", Target::Field("agent_type", STANDARD)),
    ("SubagentStop", Target::Field("agent_type", STANDARD)),
    ("PreCompact", Target::Field("trigger", STANDARD)),
    ("PostCompact", Target::Field("trigger", STANDARD)),
    ("PreModelSwitch", Target::Model),
    ("PostModelSwitch", Target::Model),
    ("ConfigChange", Target::Field("source", STANDARD)),
    ("CwdChanged", Target::None),
    ("DirectoryAdded", Target::Field("source", STANDARD)),
    ("FileChanged", Target::Basename("file_path")),
    ("StopFailure", Target::Field("error", MatcherMode::Narrow)),
    ("InstructionsLoaded", Target::Field("load_reason", STANDARD)),
    (
        "UserPromptExpansion",
        Target::Field("command_name", STANDARD),
    ),
    ("Elicitation", Target::Field("mcp_server_name", STANDARD)),
    (
        "ElicitationResult",
        Target::Field("mcp_server_name", STANDARD),
    ),
    ("UserPromptSubmit", Target::None),
    ("PostToolBatch", Target::None),
    ("Stop", Target::None),
    ("TeammateIdle", Target::None),
    ("TaskCreated", Target::None),
    ("TaskCompleted", Target::None),
    ("WorktreeCreate", Target::None),
    ("WorktreeRemove", Target::None),
    ("MessageDisplay", Target::None),
];

fn target_of(event: &str) -> Target {
    MATCHER_TARGETS
        .iter()
        .find(|(name, _)| *name == event)
        .map_or(Target::None, |(_, target)| *target)
}

fn text_of(facts: &JsValue, key: &str) -> Option<String> {
    facts.get(key).and_then(JsValue::as_str).map(str::to_owned)
}

/// The last path segment, whichever separator the host's paths use.
fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The matcher mode of `event`'s target.
#[must_use]
pub fn mode_of(event: &str) -> MatcherMode {
    match target_of(event) {
        Target::Field(_, mode) => mode,
        Target::Basename(_) => MatcherMode::Narrow,
        Target::Tool | Target::Model | Target::None => STANDARD,
    }
}

/// What `event`'s matcher is tested against in these facts (`target.valueOf`).
pub async fn value_of(event: &str, facts: &JsValue, model_catalog: Option<&Port>) -> MatcherValue {
    match target_of(event) {
        Target::Tool => MatcherValue::Filters(text_of(facts, "tool_name")),
        Target::Model => {
            let to_model = text_of(facts, "to_model").unwrap_or_default();
            canonical_matcher_model(&to_model, model_catalog)
                .await
                .map_or(MatcherValue::NoFilter, |canonical| {
                    MatcherValue::Filters(Some(canonical))
                })
        }
        Target::Field(key, _) => MatcherValue::Filters(text_of(facts, key)),
        Target::Basename(key) => {
            MatcherValue::Filters(text_of(facts, key).map(|path| basename(&path).to_owned()))
        }
        Target::None => MatcherValue::NoFilter,
    }
}

/// The tool call a tool event carries, which a handler's `if` is evaluated against
/// (`target.call`).
#[must_use]
pub fn call_of(event: &str, facts: &JsValue) -> Option<ToolCallView> {
    let member = |key: &str| facts.get(key).cloned().unwrap_or_default();
    matches!(target_of(event), Target::Tool).then(|| ToolCallView {
        tool_name: member("tool_name"),
        tool_input: member("tool_input"),
    })
}

/// `SessionStart` "at launch, including with `--continue` or `--resume`" fires before the MCP servers
/// are available; "after `/clear` or a compaction" they are. A fork (`--fork-session`) is a launch.
fn mcp_at_source(source: &str) -> Option<bool> {
    match source {
        "startup" | "resume" | "fork" => Some(false),
        "clear" | "compact" => Some(true),
        _ => None,
    }
}

/// Whether the session's MCP servers are available to the event's hooks (`target.mcpAvailable`),
/// `None` for an event whose target does not say (they are). "every `Setup` event" fires before
/// they are available.
#[must_use]
pub fn mcp_available(event: &str, facts: &JsValue) -> Option<bool> {
    match event {
        "SessionStart" => facts
            .get("source")
            .and_then(JsValue::as_str)
            .and_then(mcp_at_source),
        "Setup" => Some(false),
        _ => None,
    }
}
