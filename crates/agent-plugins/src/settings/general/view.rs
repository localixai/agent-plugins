//! Port of `src/settings/general/view.ts`, switched.
//!
//! The policy view, plus every setting of the files as one object. There is one view: the general
//! keys are rows beside the policy keys, so `get`, `winner` and `eachFromHighest` answer for them
//! too, and `merged` is the whole of settings.md, "Settings precedence" and "Lists merge instead of
//! overriding" for a host that wants an object rather than a key.

use std::sync::Arc;

use indexmap::IndexSet;
use serde::Serialize;

use crate::js::{JsObject, JsValue, in_property_order};
use crate::protocol::settings_keys::POLICY_KEYS;
use crate::registry::Function;
use crate::settings::general::keys::GENERAL_KEYS;
use crate::settings::key_spec::{SPECS, scope_carries};
use crate::settings::layers::PolicyLayer;
use crate::settings::merge::MERGES;
use crate::settings::paths::{SettingsPath, with_value_at};
use crate::settings::view::{HostView, LayersView, create_policy_view, host_view};

fn is_settings_key(name: &str) -> bool {
    POLICY_KEYS.contains(&name) || GENERAL_KEYS.contains(&name)
}

/// The top-level names any layer carries, first the highest layer's. A key a layer recorded as
/// invalid counts: no value is kept for it, but its floor still applies (`crossSessionInbound`
/// holds messages).
fn names_of(layers: &[PolicyLayer]) -> IndexSet<String> {
    layers
        .iter()
        .flat_map(|layer| {
            let own = layer
                .settings
                .as_object()
                .into_iter()
                .flat_map(|settings| settings.keys());
            own.chain(layer.invalid.iter().flatten())
                .filter(|name| scope_carries(layer.scope, name))
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A key with no row: the layers that carry it, highest first, merged by the general rule.
fn general_value(layers: &[PolicyLayer], name: &str) -> JsValue {
    let values: Vec<JsValue> = layers
        .iter()
        .filter(|layer| scope_carries(layer.scope, name))
        .filter_map(|layer| layer.settings.get(name))
        .filter(|value| !matches!(value, JsValue::Undefined | JsValue::Null))
        .cloned()
        .collect();
    if values.is_empty() {
        JsValue::Undefined
    } else {
        (MERGES.deep_merge.combine)(&values)
    }
}

fn merged_of(view: &LayersView) -> JsValue {
    let layers = view.ordered();
    let entries: JsObject = names_of(layers)
        .into_iter()
        .map(|name| {
            let value = if is_settings_key(&name) {
                view.value(&name)
            } else {
                general_value(layers, &name)
            };
            (name, value)
        })
        .filter(|(_, value)| !value.is_undefined())
        .collect();
    // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` name stays a name.
    let combined = JsValue::Object(in_property_order(entries));
    // A key inside a block has a rule of its own: the block's merge must not carry a value that rule refused.
    GENERAL_KEYS.iter().fold(combined, |settings, key| {
        match SPECS.get(key).and_then(|spec| spec.path) {
            None => settings,
            Some(path) => {
                let names = SettingsPath::Names(path.iter().map(|&name| name.to_owned()).collect());
                with_value_at(&settings, &names, &view.value(key))
            }
        }
    })
}

#[derive(Debug)]
struct Merged(Arc<LayersView>);

crate::registry::methods! {
    Merged {
        "call" => ok |this| merged_of(&this.0);
    }
}

/// The general view as the host holds it: the policy view's members and `merged`.
#[derive(Debug, Serialize)]
struct GeneralView {
    #[serde(flatten)]
    view: HostView,
    merged: Function,
}

fn create_general_view(layers: &[JsValue]) -> GeneralView {
    let view = Arc::new(create_policy_view(layers));
    GeneralView {
        view: host_view(&view),
        merged: Function::new(Merged(view)),
    }
}

crate::registry::exports! {
    "settings/general/view" {
        "createGeneralView" => ok |layers: Vec<JsValue>| create_general_view(&layers);
    }
}
