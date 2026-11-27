//! Ported from `src/settings/repair/closed.ts`; only the core uses it now, so it has no wrapper.
//!
//! The repairs of the keys that have their own fallback (managed-settings.md, "Keys that fail
//! closed", the table): in the policy tier an unreadable value reads as the fallback the table
//! gives; every other source loses the value, and an individual invalid entry is stripped
//! everywhere.

use super::network::gateway_networks_fault;
use super::support::{
    RepairAt, Repaired, dropped, failed_closed, failed_closed_no_value, kept, strip_invalid_now,
};
use crate::js::JsValue;
use crate::protocol::settings_keys::{API_PROVIDERS, MODEL_MATCH_MODES};

fn read_text(entry: &JsValue) -> Option<JsValue> {
    entry.is_string().then(|| entry.clone())
}

fn read_provider(entry: &JsValue) -> Option<JsValue> {
    entry
        .as_str()
        .filter(|name| API_PROVIDERS.contains(name))
        .map(|_| entry.clone())
}

/// `allowedProviders`: unreadable in managed settings it is an empty allowlist, so every provider
/// is refused; an entry that is no known provider name is dropped and reported, the rest enforced.
#[must_use]
pub fn provider_allowlist(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return if at.closed {
            failed_closed(
                at,
                JsValue::Array(Vec::new()),
                "an empty allowlist: every API provider is refused",
            )
        } else {
            dropped(
                at,
                &format!("\"{}\" must be a list of provider names", at.key),
            )
        };
    };
    let stripped = strip_invalid_now(items, at, read_provider, "Not a known provider name");
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// `availableModelsMatch`: `prefix` or `exact`; unreadable in managed settings it reads as
/// `exact`.
#[must_use]
pub fn match_mode(value: &JsValue, at: &RepairAt) -> Repaired {
    if value
        .as_str()
        .is_some_and(|mode| MODEL_MATCH_MODES.contains(&mode))
    {
        return kept(value.clone(), Vec::new());
    }
    if at.closed {
        failed_closed(at, JsValue::String("exact".to_owned()), "exact")
    } else {
        dropped(
            at,
            &format!(
                "\"{}\" must be one of {}",
                at.key,
                MODEL_MATCH_MODES.join(", ")
            ),
        )
    }
}

/// `deniedModels`: a non-text entry is stripped and the rest enforced; a wholly invalid value is
/// dropped with a warning and blocks no model until fixed.
#[must_use]
pub fn model_denylist(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return dropped(at, &format!("\"{}\" must be a list of models", at.key));
    };
    let stripped = strip_invalid_now(items, at, read_text, "Expected a text");
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// `forceLoginOrgUUID`: a UUID or a list of them. In managed settings "a value that isn't a
/// string or an array of strings" admits no organization, which the empty list says: any of no
/// organizations (settings-reference.md, `forceLoginOrgUUID`; spec/judgments.md). A list with one
/// non-text entry is not an array of strings, so it reads that way whole: stripping the entry
/// would admit the others. Outside the policy tier an invalid entry is stripped like any list
/// entry.
#[must_use]
pub fn org_uuid(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_string() {
        return kept(value.clone(), Vec::new());
    }
    if let Some(items) = value
        .as_array()
        .filter(|items| !at.closed || items.iter().all(JsValue::is_string))
    {
        let stripped = strip_invalid_now(items, at, read_text, "Expected a text");
        return kept(JsValue::Array(stripped.entries), stripped.issues);
    }
    if at.closed {
        failed_closed(
            at,
            JsValue::Array(Vec::new()),
            "no organization is permitted to log in",
        )
    } else {
        dropped(
            at,
            &format!("\"{}\" must be a UUID or a list of UUIDs", at.key),
        )
    }
}

/// `gatewayInternalNetworks`: unreadable in managed settings, `/login` "refuses every new cloud
/// gateway sign-in on that machine until the value is fixed". No value says that, so the layer
/// records the key as invalid (`PolicyLayer.invalid`).
#[must_use]
pub fn gateway_networks(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(fault) = gateway_networks_fault(value) else {
        return kept(value.clone(), Vec::new());
    };
    if at.closed {
        failed_closed_no_value(
            at,
            &format!("{fault}; /login refuses every new cloud gateway sign-in"),
        )
    } else {
        dropped(at, &fault)
    }
}

/// A key another module judges from the document as written: kept whatever the value.
#[must_use]
pub const fn as_written(value: JsValue) -> Repaired {
    kept(value, Vec::new())
}
