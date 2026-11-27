//! Ported from `src/hooks/verdict/blocked-prompt.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a blocked prompt leaves behind (hooks.md, "What a blocked prompt leaves behind"): "By
//! default the block message shown to the user ends with `Original prompt:` followed by the
//! submitted text". Printing `suppressOriginalPrompt: true` in `hookSpecificOutput` leaves the text
//! out, "whether the hook blocks with `decision: "block"` or by exiting 2". It changes only the
//! block message: the host still writes the submitted text wherever it keeps the transcript.
//!
//! Judgment (spec/judgments.md): the docs give no layout for the ending, so the label starts a line
//! of its own. A block no one is shown (`to` is not the user) has no message to end.
//!
//! The outcome is the host's whole value, so every member it does not change goes back as given.

use crate::hooks::input::fact_read::text_fact_of;
use crate::js::{JsObject, JsValue};

/// The events whose block message quotes what was submitted.
const QUOTES_PROMPT: [&str; 1] = ["UserPromptSubmit"];

const ORIGINAL_PROMPT: &str = "Original prompt:";

/// The outcome with the submitted prompt at the end of its block message.
#[must_use]
pub fn with_original_prompt(mut outcome: JsObject, facts: &JsValue) -> JsObject {
    let prompt = text_fact_of(facts, "prompt");
    let event = outcome.get("event").and_then(JsValue::as_str);
    let quotes = event.is_some_and(|event| QUOTES_PROMPT.contains(&event));
    let suppressed = outcome
        .get("effects")
        .and_then(|effects| effects.get("suppressOriginalPrompt"))
        .and_then(JsValue::as_bool)
        == Some(true);
    let (Some(prompt), true) = (prompt, quotes) else {
        return outcome;
    };
    let Some(JsValue::Object(block)) = outcome.get_mut("block") else {
        return outcome;
    };
    if block.get("to").and_then(JsValue::as_str) != Some("user") || suppressed {
        return outcome;
    }
    let message = block
        .get("message")
        .map(crate::js::string_of)
        .unwrap_or_default();
    block.insert(
        "message".to_owned(),
        JsValue::String(format!("{message}\n{ORIGINAL_PROMPT} {prompt}")),
    );
    outcome
}
