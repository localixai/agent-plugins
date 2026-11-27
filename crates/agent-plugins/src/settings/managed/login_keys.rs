//! Port of `src/settings/managed/login-keys.ts`, switched.
//!
//! Which managed source supplies the login keys (managed-settings.md, "Which managed source supplies
//! login keys"; settings-reference.md, `managedSourcesBehavior`). Under either setting the keys come
//! from the machine's own policy: "the MDM policy or managed settings file, whichever is the
//! higher-ranked one carrying a policy key", ahead of remote settings. When that policy sets any
//! policy key, the file below it contributes no login key, even under `"merge"`.

use crate::js::{JsObject, JsValue};
use crate::settings::layers::{PolicyLayer, PolicyScope};

/// The login keys.
pub const LOGIN_KEYS: [&str; 4] = [
    "forceLoginOrgUUID",
    "forceLoginMethod",
    "forceLoginGatewayUrl",
    "gatewayInternalNetworks",
];

/// The `forceLoginMethod` values the first-party sign-in takes; `"gateway"` is a gateway sign-in key.
const FIRST_PARTY_METHODS: [&str; 2] = ["claudeai", "console"];

/// What remote settings may add where the machine's policy omits the key: the organization and the
/// first-party methods. "Claude Code doesn't use a value in remote settings for the gateway sign-in."
fn remote_fills(key: &str, value: &JsValue) -> bool {
    match key {
        "forceLoginOrgUUID" => true,
        "forceLoginMethod" => value
            .as_str()
            .is_some_and(|method| FIRST_PARTY_METHODS.contains(&method)),
        _ => false,
    }
}

/// A member of a settings object that is set (not `undefined`).
fn member<'a>(settings: &'a JsValue, key: &str) -> Option<&'a JsValue> {
    settings.get(key).filter(|value| !value.is_undefined())
}

/// The login keys in force: the machine's policy wins, remote settings fill the first-party keys it omits.
#[must_use]
pub fn login_keys_of(machine: &JsValue, remote: &JsValue) -> JsValue {
    let mut entries = JsObject::new();
    for key in LOGIN_KEYS {
        if let Some(from_machine) = member(machine, key) {
            entries.insert(key.to_owned(), from_machine.clone());
        } else if let Some(from_remote) = member(remote, key)
            && remote_fills(key, from_remote)
        {
            entries.insert(key.to_owned(), from_remote.clone());
        }
    }
    JsValue::Object(entries)
}

/// The settings without any login key.
#[must_use]
pub fn without_login_keys(settings: &JsValue) -> JsValue {
    JsValue::Object(
        settings
            .as_object()
            .map(|members| {
                members
                    .iter()
                    .filter(|(name, _)| !LOGIN_KEYS.contains(&name.as_str()))
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// `/login` refuses every new cloud gateway sign-in on the machine while the highest managed source on
/// it carries `gatewayInternalNetworks` with a value that cannot be read (managed-settings.md, "Keys
/// that fail closed"): no value says that, so the managed layer records the key as invalid.
#[must_use]
pub fn gateway_sign_in_refused(layer: &PolicyLayer) -> bool {
    layer
        .invalid
        .as_ref()
        .is_some_and(|invalid| invalid.iter().any(|name| name == "gatewayInternalNetworks"))
}

/// The managed tier as a layer: the login keys the highest machine source carried unreadable are
/// recorded as invalid (`PolicyLayer.invalid`), since only that source supplies them.
#[must_use]
pub fn managed_layer(settings: JsValue, machine: Option<&[String]>) -> PolicyLayer {
    let invalid: Vec<String> = machine
        .unwrap_or_default()
        .iter()
        .filter(|name| LOGIN_KEYS.contains(&name.as_str()))
        .cloned()
        .collect();
    PolicyLayer {
        scope: PolicyScope::Managed,
        settings,
        invalid: if invalid.is_empty() {
            None
        } else {
            Some(invalid)
        },
    }
}

/// The machine's source as `managedLayer` reads it: the keys it carried unreadable.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct MachineInvalid {
    /// The unreadable keys.
    pub invalid: Vec<String>,
}

crate::registry::exports! {
    "settings/managed/login-keys" {
        "loginKeysOf" => ok |machine: JsValue, remote: JsValue| login_keys_of(&machine, &remote);
        "withoutLoginKeys" => ok |settings: JsValue| without_login_keys(&settings);
        "gatewaySignInRefused" => ok |layer: PolicyLayer| gateway_sign_in_refused(&layer);
        "managedLayer" => ok |settings: JsValue, machine: Option<MachineInvalid>| managed_layer(settings, machine.as_ref().map(|found| found.invalid.as_slice()));
    }
}
