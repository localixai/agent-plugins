//! Port of `src/hooks/output/limit.ts`, switched.
//!
//! The 10,000-character cap on each string of each hook (hooks.md, "JSON output"): the JSON fields
//! `additionalContext`, `systemMessage` and `initialUserMessage`, each on its own, and the plain
//! stdout whole. Every hook is measured alone, "even when several hooks run for the same event".
//!
//! A `TaggedOutput` is taken and given back as the object the caller built (`{ ...tagged }` keeps
//! every member, the handler's whole record among them); only its capped strings change.

use serde::{Deserialize, Serialize};

use super::caps::output_caps;
use super::fields::string_in;
use super::reminder_tags::escape_reminder_tags;
use super::text_cap::{Limited, cut_text, limit_text};
use crate::format::issues::PluginIssue;
use crate::hooks::verdict::types::BackgroundDelivery;
use crate::js::{JsObject, JsValue};
use crate::ports::Signal;
use crate::registry::Port;

/// Where text over the cap is saved, and the abort.
#[derive(Debug, Deserialize)]
pub struct LimitRequest {
    /// The host's `ScratchFiles`; without it such text is cut at the cap.
    #[serde(default)]
    pub scratch: Option<Port>,
    /// The session's abort.
    pub signal: Signal,
}

/// `LimitedOutputs`: each hook's output with its text capped, and why any was cut.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LimitedOutputs {
    /// The outputs (`TaggedOutput`s), in order.
    pub outputs: Vec<JsObject>,
    /// What capping found.
    pub issues: Vec<PluginIssue>,
}

/// One capped string of a JSON output: how to read it and how to put a replacement back.
struct Slot {
    field: &'static str,
    /// The event's cap that must accept the field: a field the event discards is never written out.
    cap: &'static str,
    read: fn(&JsObject, &str) -> Option<String>,
    write: fn(&mut JsObject, &str, String),
}

fn read_specific(output: &JsObject, key: &str) -> Option<String> {
    string_in(output.get("hookSpecificOutput"), key)
}

fn write_specific(output: &mut JsObject, key: &str, text: String) {
    if let Some(JsValue::Object(specific)) = output.get_mut("hookSpecificOutput") {
        specific.insert(key.to_owned(), JsValue::String(text));
    }
}

const fn specific_slot(key: &'static str) -> Slot {
    Slot {
        field: key,
        cap: "specific",
        read: read_specific,
        write: write_specific,
    }
}

const SLOTS: [Slot; 3] = [
    Slot {
        field: "systemMessage",
        cap: "systemMessage",
        read: |output, _| {
            output
                .get("systemMessage")
                .and_then(JsValue::as_str)
                .map(str::to_owned)
        },
        write: |output, _, text| {
            output.insert("systemMessage".to_owned(), JsValue::String(text));
        },
    },
    specific_slot("additionalContext"),
    specific_slot("initialUserMessage"),
];

fn name_of(event: &str, handler_id: &str, field: &str) -> String {
    format!("hook-{event}-{handler_id}-{field}")
}

/// `tagged.handler.id`.
fn handler_id(tagged: &JsObject) -> &str {
    tagged
        .get("handler")
        .and_then(|handler| handler.get("id"))
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

/// Whether `OUTPUT_CAPS[event].accepts` has `cap`.
fn accepts(event: &str, cap: &str) -> bool {
    output_caps()
        .get(event)
        .is_some_and(|caps| caps.accepts.members.iter().any(|field| field == cap))
}

async fn limit_json(
    event: &str,
    tagged: &JsObject,
    request: &LimitRequest,
) -> (Option<JsObject>, Vec<PluginIssue>) {
    let Some(JsValue::Object(output)) = tagged.get("output") else {
        return (None, Vec::new());
    };
    let mut current = output.clone();
    let mut issues = Vec::new();
    for slot in &SLOTS {
        // "Some events discard" the field ("JSON output"): spilling it would only write a file nobody reads.
        if !accepts(event, slot.cap) {
            continue;
        }
        let Some(value) = (slot.read)(&current, slot.field) else {
            continue;
        };
        let limited = limit_text(
            &name_of(event, handler_id(tagged), slot.field),
            &value,
            request.scratch.as_ref(),
            &request.signal,
        )
        .await;
        (slot.write)(&mut current, slot.field, limited.text);
        issues.extend(limited.issues);
    }
    (Some(current), issues)
}

/// Plain stdout: spilled like the JSON strings, except `PreCompact`'s, which is cut at the cap.
async fn limit_plain(
    event: &str,
    tagged: &JsObject,
    request: &LimitRequest,
) -> (Option<JsObject>, Vec<PluginIssue>) {
    let Some(JsValue::Object(plain)) = tagged.get("plainText") else {
        return (None, Vec::new());
    };
    let text = plain
        .get("text")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let mut limited_plain = plain.clone();
    if plain.get("as").and_then(JsValue::as_str) == Some("compaction-instructions") {
        limited_plain.insert("text".to_owned(), JsValue::String(cut_text(text)));
        return (Some(limited_plain), Vec::new());
    }
    let limited = limit_text(
        &name_of(event, handler_id(tagged), "stdout"),
        text,
        request.scratch.as_ref(),
        &request.signal,
    )
    .await;
    limited_plain.insert("text".to_owned(), JsValue::String(limited.text));
    (Some(limited_plain), limited.issues)
}

/// Caps each hook's text on `event`.
pub async fn limit_outputs(
    event: &str,
    outputs: Vec<JsObject>,
    request: &LimitRequest,
) -> LimitedOutputs {
    let mut limited = Vec::new();
    let mut issues = Vec::new();
    for mut tagged in outputs {
        let (json, json_issues) = limit_json(event, &tagged, request).await;
        let (plain, plain_issues) = limit_plain(event, &tagged, request).await;
        issues.extend(json_issues);
        issues.extend(plain_issues);
        if let Some(output) = json {
            tagged.insert("output".to_owned(), JsValue::Object(output));
        }
        if let Some(plain_text) = plain {
            tagged.insert("plainText".to_owned(), JsValue::Object(plain_text));
        }
        limited.push(tagged);
    }
    LimitedOutputs {
        outputs: limited,
        issues,
    }
}

/// What a background hook delivers, under the same cap and escape as a foreground hook's text: "A
/// hook's `additionalContext`, `systemMessage` ... strings ... are capped at 10,000 characters"
/// whether it ran in the foreground or not, and the context reaches Claude with its reminder tags
/// escaped (`escapeReminderTags`). A delivery carries no issues: a text cut for want of a
/// `ScratchFiles` port says so in its own marker.
pub async fn limit_delivery(
    event: &str,
    delivery: BackgroundDelivery,
    request: &LimitRequest,
) -> BackgroundDelivery {
    let limit = Limit {
        event,
        handler_id: &delivery.handler_id,
        request,
    };
    let context = limit
        .of("additionalContext", delivery.additional_context.as_deref())
        .await;
    let message = limit
        .of("systemMessage", delivery.system_message.as_deref())
        .await;
    BackgroundDelivery {
        additional_context: context.map(|context| escape_reminder_tags(&context.text)),
        system_message: message.map(|message| message.text),
        ..delivery
    }
}

/// `limit` of `limitDelivery`: what names a field's file.
struct Limit<'a> {
    event: &'a str,
    handler_id: &'a str,
    request: &'a LimitRequest,
}

impl Limit<'_> {
    /// The field's text capped, `None` when it has none.
    async fn of(&self, field: &str, text: Option<&str>) -> Option<Limited> {
        let text = text?;
        let name = name_of(self.event, self.handler_id, field);
        let request = self.request;
        Some(limit_text(&name, text, request.scratch.as_ref(), &request.signal).await)
    }
}

crate::registry::exports! {
    "hooks/output/limit" {
        "limitDelivery" => async ok |event: String, delivery: BackgroundDelivery, request: LimitRequest|
            limit_delivery(&event, delivery, &request).await;
    }
}
