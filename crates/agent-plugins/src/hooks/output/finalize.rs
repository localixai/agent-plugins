//! Port of `src/hooks/output/finalize.ts`, switched.
//!
//! A block that is not a JSON decision (an exit-2 hook, a failure) holds the action back, so the
//! effects the merge computed from the other hooks must agree with it: "precedence is `deny` >
//! `defer` > `ask` > `allow`", "a decline from one of them overrides an `accept` or `cancel` from
//! another" (hooks.md, "PreToolUse decision control", "Other ways to decline an elicitation").

use super::fields::has_text;
use crate::hooks::verdict::types::BlockInfo;
use crate::js::{JsObject, JsValue};

type OnBlock = fn(JsObject, &BlockInfo) -> JsObject;

fn deny_tool(mut effects: JsObject, block: &BlockInfo) -> JsObject {
    effects.shift_remove("updatedInput");
    effects.insert(
        "permissionDecision".to_owned(),
        JsValue::String("deny".to_owned()),
    );
    if has_text(Some(&block.reason)) {
        effects.insert(
            "permissionDecisionReason".to_owned(),
            JsValue::String(block.reason.clone()),
        );
    }
    effects
}

/// Only a failure that "denies" blocks a `PermissionRequest`; its text is the denial's `message`. A
/// deny a hook decided keeps its own `interrupt` ("if `true`, stops Claude").
fn deny_request(mut effects: JsObject, block: &BlockInfo) -> JsObject {
    let decided = effects.get("permissionRequest");
    let interrupt = decided
        .and_then(|decided| decided.get("behavior"))
        .and_then(JsValue::as_str)
        == Some("deny")
        && decided
            .and_then(|decided| decided.get("interrupt"))
            .and_then(JsValue::as_bool)
            == Some(true);
    let mut request = JsObject::new();
    request.insert("behavior".to_owned(), JsValue::String("deny".to_owned()));
    if has_text(Some(&block.reason)) {
        request.insert("message".to_owned(), JsValue::String(block.reason.clone()));
    }
    if interrupt {
        request.insert("interrupt".to_owned(), JsValue::Bool(true));
    }
    effects.insert("permissionRequest".to_owned(), JsValue::Object(request));
    effects
}

fn decline(mut effects: JsObject, _: &BlockInfo) -> JsObject {
    effects.shift_remove("elicitation");
    let mut answer = JsObject::new();
    answer.insert("action".to_owned(), JsValue::String("decline".to_owned()));
    effects.insert("elicitation".to_owned(), JsValue::Object(answer));
    effects
}

fn on_block(event: &str) -> Option<OnBlock> {
    match event {
        "PreToolUse" | "PreModelSwitch" => Some(deny_tool),
        "PermissionRequest" => Some(deny_request),
        "Elicitation" | "ElicitationResult" => Some(decline),
        _ => None,
    }
}

/// The effects as a block that is not a JSON decision leaves them.
#[must_use]
pub fn finalize_effects(event: &str, effects: JsObject, block: Option<&BlockInfo>) -> JsObject {
    match (block, on_block(event)) {
        (Some(block), Some(rule)) => rule(effects, block),
        _ => effects,
    }
}

crate::registry::exports! {
    "hooks/output/finalize" {
        "finalizeEffects" => ok |event: String, effects: JsObject, block: Option<BlockInfo>|
            finalize_effects(&event, effects, block.as_ref());
    }
}
