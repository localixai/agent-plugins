//! Port of `src/host/session-permission-updates.ts`, switched.
//!
//! The outcome is the host's whole value, so every member it does not change goes back as given;
//! the session's options hold its ports (functions), so they cross as the host's object and the two
//! members read here are asked for.

use serde::Deserialize;

use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::Port;
use crate::rules::decide::hook_permission_updates::{
    PermissionUpdate, Session, permission_updates_in_effect,
};

/// The part of `permissionContext` read here.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassContext {
    /// Whether bypass mode is available, when the host gave it.
    #[serde(default)]
    pub bypass_available: Option<bool>,
}

/// The members of `SessionOptions` read here.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassOptions {
    /// What `decide` reads.
    #[serde(default)]
    pub permission_context: Option<BypassContext>,
    /// The session's mode.
    #[serde(default)]
    pub permission_mode: Option<String>,
}

/// `SessionOptions` as it crosses: the host's object when it holds a function, else data.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SessionOptions {
    /// The options hold a function.
    Host(Port),
    /// The options are data.
    Data(BypassOptions),
}

impl SessionOptions {
    async fn read(&self) -> BypassOptions {
        match self {
            Self::Data(data) => data.clone(),
            Self::Host(port) => BypassOptions {
                permission_context: port
                    .member::<Option<BypassContext>>("permissionContext")
                    .await,
                permission_mode: port.member::<Option<String>>("permissionMode").await,
            },
        }
    }
}

/// Whether bypass mode is available to this session: `permissionContext.bypassAvailable` when the
/// host gave it, otherwise only a session that is already in `bypassPermissions` (judgment,
/// spec/judgments.md: the session knows no more of its launch).
fn bypass_available(options: &BypassOptions) -> bool {
    options
        .permission_context
        .and_then(|context| context.bypass_available)
        .unwrap_or_else(|| options.permission_mode.as_deref() == Some("bypassPermissions"))
}

/// The outcome with the `updatedPermissions` of a `PermissionRequest` allow as they take effect
/// (hooks.md, "Permission update entries"): a `setMode` to `bypassPermissions` the session cannot
/// take is dropped, and one it can take stays in memory (`permissionUpdatesInEffect`).
pub async fn with_updates_in_effect(mut outcome: JsObject, options: &SessionOptions) -> JsObject {
    let Some(JsValue::Object(effects)) = outcome.get("effects") else {
        return outcome;
    };
    let Some(JsValue::Object(answer)) = effects.get("permissionRequest") else {
        return outcome;
    };
    let Some(given) = answer.get("updatedPermissions") else {
        return outcome;
    };
    if given.is_undefined() {
        return outcome;
    }
    let Ok(updates) = Vec::<PermissionUpdate>::deserialize(given.clone()) else {
        return outcome;
    };
    let session = Session {
        bypass_available: bypass_available(&options.read().await),
    };
    let kept = permission_updates_in_effect(updates, session);
    let mut request = answer.clone();
    request.shift_remove("updatedPermissions");
    if !kept.is_empty() {
        request.insert(
            "updatedPermissions".to_owned(),
            to_js_value(&kept).unwrap_or_default(),
        );
    }
    if let Some(JsValue::Object(effects)) = outcome.get_mut("effects") {
        effects.insert("permissionRequest".to_owned(), JsValue::Object(request));
    }
    outcome
}

crate::registry::exports! {
    "host/session-permission-updates" {
        "withUpdatesInEffect" => async ok |outcome: JsObject, options: SessionOptions|
            with_updates_in_effect(outcome, &options).await;
    }
}
