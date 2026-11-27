//! Port of `src/hooks/core/input.ts`, switched.
//!
//! Which Claude Code occurrences are also core occurrences, and the core input each gives (Agent
//! Plugins Extended §18.1.2, the table and "Inputs"; §18.2): exactly the core fields (core
//! §7.3.3–§7.3.8), the common ones with the values Claude Code's handlers get, never a Claude Code
//! field. An event without a core counterpart, and an occurrence the table excludes, gives none.

use serde::Deserialize;

use super::profile_results::{Completed, completed_profile};
use super::profiles::{ProfileContext, pending_profile};
use super::types::{CoreCallFacts, CoreOccurrence, CoreToolProfile, TurnOrigin};
use crate::format::core::hooks::CoreHookEvent;
use crate::hooks::input::fact_read::{fact_of, text_fact_of};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

/// Where an occurrence happens.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceContext {
    /// The session.
    pub session_id: String,
    /// The working directory.
    pub cwd: String,
    /// The hooks fire in a subagent: core `SessionStart` does not run for one (core §7.3.4).
    pub subagent: bool,
    /// What the host knows of the call.
    pub facts: CoreCallFacts,
}

type Facts = JsValue;
type Builder = fn(&Facts, &OccurrenceContext) -> Option<CoreOccurrence>;

/// J18.6: a new root session with a new `session_id`; `resume` and `compact` keep the id.
const NEW_SESSION: [&str; 3] = ["startup", "clear", "fork"];

/// J18.7: the `source` values of the turns Claude Code starts itself (a scheduled task, a `/loop`
/// wakeup). `system` holds both kinds ("peer/channel messages, task notifications,
/// auto-continuation"), so only the host's `CoreCallFacts.turnOrigin` decides it, never a default
/// (`needsTurnOrigin`); an absent `source` is a submitted prompt unless the host says otherwise.
const STARTED_BY_CLIENT: [&str; 2] = ["loop_wakeup", "schedule_wakeup"];
const BY_ORIGIN_ONLY: [&str; 1] = ["system"];
const ORIGIN_EVENTS: [&str; 1] = ["UserPromptSubmit"];

const fn fires(origin: TurnOrigin) -> bool {
    match origin {
        TurnOrigin::External => true,
        TurnOrigin::Client => false,
    }
}

fn source_of(facts: &Facts) -> String {
    text_fact_of(facts, "source").unwrap_or_default()
}

/// Whether a `UserPromptSubmit` turn is one only the host can classify and the host did not.
#[must_use]
pub fn needs_turn_origin(event: &str, facts: &Facts, core: &CoreCallFacts) -> bool {
    ORIGIN_EVENTS.contains(&event)
        && core.turn_origin.is_none()
        && BY_ORIGIN_ONLY.contains(&source_of(facts).as_str())
}

/// Whether a `UserPromptSubmit` turn is an external submission (J18.7).
fn is_external(facts: &Facts, core: &CoreCallFacts) -> bool {
    if let Some(origin) = core.turn_origin {
        return fires(origin);
    }
    let source = source_of(facts);
    !BY_ORIGIN_ONLY.contains(&source.as_str()) && !STARTED_BY_CLIENT.contains(&source.as_str())
}

fn common(event: CoreHookEvent, context: &OccurrenceContext) -> JsObject {
    JsObject::from_iter([
        ("hook_event_name".to_owned(), JsValue::from(event.as_str())),
        (
            "session_id".to_owned(),
            JsValue::from(context.session_id.as_str()),
        ),
        ("cwd".to_owned(), JsValue::from(context.cwd.as_str())),
    ])
}

fn with_profile(mut input: JsObject, profile: Option<CoreToolProfile>) -> JsObject {
    if let Some(profile) = profile {
        input.insert(
            "tool_profile".to_owned(),
            to_js_value(&profile).unwrap_or_default(),
        );
    }
    input
}

struct Call {
    tool_name: String,
    tool_use_id: String,
    tool_input: JsValue,
}

fn call(facts: &Facts) -> Call {
    let input = fact_of(facts, "tool_input");
    Call {
        tool_name: text_fact_of(facts, "tool_name").unwrap_or_default(),
        tool_use_id: text_fact_of(facts, "tool_use_id").unwrap_or_default(),
        tool_input: if input.is_nullish() {
            JsValue::Object(JsObject::new())
        } else {
            input
        },
    }
}

/// What a completed call came to, and the result the core input carries.
struct After {
    completed: Completed,
    result: JsValue,
}

fn tool_occurrence(
    event: CoreHookEvent,
    facts: &Facts,
    context: &OccurrenceContext,
    after: Option<After>,
) -> CoreOccurrence {
    let Call {
        tool_name,
        tool_use_id,
        tool_input,
    } = call(facts);
    let mut base = common(event, context);
    base.insert("tool_use_id".to_owned(), JsValue::from(tool_use_id));
    base.insert("tool_name".to_owned(), JsValue::from(tool_name.as_str()));
    base.insert("tool_input".to_owned(), tool_input.clone());
    let profile_context = ProfileContext {
        cwd: context.cwd.clone(),
        facts: context.facts.clone(),
    };
    let input = match after {
        None => with_profile(
            base,
            pending_profile(&tool_name, &tool_input, &profile_context),
        ),
        Some(after) => {
            base.insert("tool_result".to_owned(), after.result);
            with_profile(
                base,
                completed_profile(&tool_name, &tool_input, &after.completed, &profile_context),
            )
        }
    };
    CoreOccurrence {
        event,
        input,
        tool_name: Some(tool_name),
    }
}

fn mcp_result_or(context: &OccurrenceContext, fallback: JsValue) -> JsValue {
    context
        .facts
        .mcp_result
        .clone()
        .map_or(fallback, JsValue::Object)
}

fn session_start(facts: &Facts, context: &OccurrenceContext) -> Option<CoreOccurrence> {
    let source = text_fact_of(facts, "source").unwrap_or_default();
    if context.subagent || !NEW_SESSION.contains(&source.as_str()) {
        return None;
    }
    Some(CoreOccurrence {
        event: CoreHookEvent::SessionStart,
        input: common(CoreHookEvent::SessionStart, context),
        tool_name: None,
    })
}

fn user_prompt_submit(facts: &Facts, context: &OccurrenceContext) -> Option<CoreOccurrence> {
    if !is_external(facts, &context.facts) {
        return None;
    }
    let prompt = text_fact_of(facts, "prompt").unwrap_or_default();
    let mut input = common(CoreHookEvent::UserPromptSubmit, context);
    input.insert("prompt".to_owned(), JsValue::from(prompt));
    Some(CoreOccurrence {
        event: CoreHookEvent::UserPromptSubmit,
        input,
        tool_name: None,
    })
}

fn pre_tool_use(facts: &Facts, context: &OccurrenceContext) -> CoreOccurrence {
    tool_occurrence(CoreHookEvent::PreToolUse, facts, context, None)
}

fn post_tool_use(facts: &Facts, context: &OccurrenceContext) -> CoreOccurrence {
    let response = fact_of(facts, "tool_response");
    // J18.2: `tool_result` is the tool's own result, as Claude Code's `tool_response` carries it;
    // an MCP call's is the server's `CallToolResult` when the host gives it.
    let result = mcp_result_or(context, response.clone());
    let completed = Completed {
        ok: true,
        response,
        error: String::new(),
    };
    tool_occurrence(
        CoreHookEvent::PostToolUse,
        facts,
        context,
        Some(After { completed, result }),
    )
}

// J18.8: a failure the tool reports is a completed result; an interrupt is not.
fn post_tool_use_failure(facts: &Facts, context: &OccurrenceContext) -> Option<CoreOccurrence> {
    if fact_of(facts, "is_interrupt").as_bool() == Some(true) {
        return None;
    }
    let error = text_fact_of(facts, "error").unwrap_or_default();
    let completed = Completed {
        ok: false,
        response: JsValue::Undefined,
        error: error.clone(),
    };
    let result = mcp_result_or(context, JsValue::from(error));
    Some(tool_occurrence(
        CoreHookEvent::PostToolUse,
        facts,
        context,
        Some(After { completed, result }),
    ))
}

// J18.9: one continuation mechanism, so `continuation_active` is `stop_hook_active`.
fn stop(facts: &Facts, context: &OccurrenceContext) -> CoreOccurrence {
    let mut input = common(CoreHookEvent::Stop, context);
    input.insert(
        "continuation_active".to_owned(),
        JsValue::Bool(fact_of(facts, "stop_hook_active").as_bool() == Some(true)),
    );
    CoreOccurrence {
        event: CoreHookEvent::Stop,
        input,
        tool_name: None,
    }
}

// `EndConversation` fires no tool event in Claude Code either; the host does not dispatch one.
const BUILDERS: [(&str, Builder); 6] = [
    ("SessionStart", session_start),
    ("UserPromptSubmit", user_prompt_submit),
    ("PreToolUse", |facts, context| {
        Some(pre_tool_use(facts, context))
    }),
    ("PostToolUse", |facts, context| {
        Some(post_tool_use(facts, context))
    }),
    ("PostToolUseFailure", post_tool_use_failure),
    ("Stop", |facts, context| Some(stop(facts, context))),
];

/// The core occurrence of a Claude Code dispatch, or `None` when the core event does not fire.
#[must_use]
pub fn core_occurrence(
    event: &str,
    facts: &Facts,
    context: &OccurrenceContext,
) -> Option<CoreOccurrence> {
    BUILDERS
        .iter()
        .find(|(name, _)| *name == event)
        .and_then(|(_, build)| build(facts, context))
}

crate::registry::exports! {
    "hooks/core/input" {
        "needsTurnOrigin" => ok |event: String, facts: JsValue, core: CoreCallFacts|
            needs_turn_origin(&event, &facts, &core);
        "coreOccurrence" => ok |event: String, facts: JsValue, context: OccurrenceContext|
            core_occurrence(&event, &facts, &context);
    }
}
