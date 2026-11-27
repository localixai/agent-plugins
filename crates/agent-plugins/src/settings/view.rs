//! Port of `src/settings/view.ts`, switched.
//!
//! Typed questions over the ordered layers, the one thing every area resolver asks (docs/API.md,
//! "Settings"). A resolver never walks the layers itself, so none of them repeats the precedence
//! order, the scope lines or the merge rules.
//!
//! `PolicyView` is the trait a resolver asks: the core's own view (`create_policy_view`) answers it
//! at once, and the host's object crosses as a `Port` whose methods are needs (`crate::registry`).
//! The view the package builds is the core's: its methods cross as functions of the core
//! (`registry::Function`), its layers as data.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::{Function, Port};
use crate::settings::general::exceptions::honor_exception;
use crate::settings::key_spec::SPECS;
use crate::settings::layers::{KeyHolder, PolicyLayer, PolicyScope, holders_of, ordered_layers};

/// Typed questions over the ordered layers (docs/API.md, "Settings"). A key is the name the
/// package uses (`allowedChannelPlugins`, `permissions.defaultMode`).
pub trait PolicyView: Send + Sync {
    /// The layers, highest precedence first.
    fn layers(&self) -> BoxFuture<'_, Vec<PolicyLayer>>;
    /// The key as the files combine it (`None` when no file sets it).
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<JsValue>>;
    /// The highest layer that sets the key.
    fn winner<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<KeyHolder>>;
    /// Every layer that sets the key, highest first: what a key with its own rule walks.
    fn each_from_highest<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Vec<KeyHolder>>;
}

/// The host's view: `layers` read as its member, each question as its method.
impl PolicyView for Port {
    fn layers(&self) -> BoxFuture<'_, Vec<PolicyLayer>> {
        Box::pin(self.member::<Vec<PolicyLayer>>("layers"))
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<JsValue>> {
        let asked = self.method::<JsValue>("get", vec![key.into()]);
        Box::pin(async move { Some(asked.await).filter(|value| !value.is_undefined()) })
    }

    fn winner<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<KeyHolder>> {
        Box::pin(self.method::<Option<KeyHolder>>("winner", vec![key.into()]))
    }

    fn each_from_highest<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Vec<KeyHolder>> {
        Box::pin(self.method::<Vec<KeyHolder>>("eachFromHighest", vec![key.into()]))
    }
}

fn is_managed(holder: &KeyHolder) -> bool {
    holder.scope == PolicyScope::Managed
}
fn is_parent(holder: &KeyHolder) -> bool {
    holder.scope == PolicyScope::Parent
}

/// The admin tier's holders of a key: the highest managed source that sets it, else the parent's:
/// "the list from the highest-ranked admin source that sets one applies and blocks the parent's. The
/// parent's list applies only when no admin source sets one" (managed-settings.md, "Let an embedding
/// host add policy").
fn admin_or_parent(holders: &[KeyHolder]) -> Vec<KeyHolder> {
    match holders.iter().find(|holder| is_managed(holder)) {
        None => holders
            .iter()
            .filter(|holder| is_parent(holder))
            .cloned()
            .collect(),
        Some(admin) => vec![admin.clone()],
    }
}

/// The holders the key's `AdminTier` lets count (`ValueSpec.admin`).
fn counted_holders(key: &str, holders: &[KeyHolder], managed: &[JsValue]) -> Vec<KeyHolder> {
    let Some(rule) = SPECS.get(key).and_then(|spec| spec.admin.as_ref()) else {
        return holders.to_vec();
    };
    if rule
        .pinned_when
        .as_ref()
        .is_some_and(|pinned| pinned(managed))
    {
        return admin_or_parent(holders);
    }
    let admin = admin_or_parent(holders);
    if rule.owns == Some(true) && !admin.is_empty() {
        return admin;
    }
    if rule.blocks_parent == Some(true) && holders.iter().any(is_managed) {
        holders
            .iter()
            .filter(|holder| !is_parent(holder))
            .cloned()
            .collect()
    } else {
        holders.to_vec()
    }
}

/// What the layers that set `key` come to: the row's merge over the holders that count, highest
/// first. A key whose managed value is not an override then lets a restrictive value of a lower scope
/// stand (settings.md, "Exceptions to managed settings precedence").
fn combine(key: &str, counted: &[KeyHolder], floors: Vec<JsValue>) -> JsValue {
    let Some(spec) = SPECS.get(key) else {
        return JsValue::Undefined;
    };
    let exception = spec.exception.map(|rule| &**rule);
    let by_precedence: Vec<KeyHolder> = match exception {
        Some(rule) => counted
            .iter()
            .filter(|holder| !rule.only_when_stricter.contains(&holder.scope))
            .cloned()
            .collect(),
        None => counted.to_vec(),
    };
    let winning = if by_precedence.is_empty() {
        JsValue::Undefined
    } else {
        (spec.merge.combine)(&values_of(&by_precedence))
    };
    let Some(rule) = exception else {
        return winning;
    };
    let honored: Vec<KeyHolder> = counted
        .iter()
        .filter(|holder| rule.honored_from.contains(&holder.scope))
        .cloned()
        .collect();
    let mut values = values_of(&honored);
    values.extend(floors);
    honor_exception(rule, winning, &values)
}

/// The floors the layers' unreadable values of `key` set (`ManagedException.unreadable`).
fn floors_of(layers: &[PolicyLayer], key: &str) -> Vec<JsValue> {
    let rule = SPECS
        .get(key)
        .and_then(|spec| spec.exception)
        .and_then(|exception| exception.unreadable.as_ref());
    let Some(rule) = rule else {
        return Vec::new();
    };
    layers
        .iter()
        .filter(|layer| {
            layer
                .invalid
                .as_ref()
                .is_some_and(|keys| keys.iter().any(|one| one == key))
                && rule.from.contains(&layer.scope)
        })
        .map(|_| rule.floor.clone())
        .collect()
}

fn values_of(holders: &[KeyHolder]) -> Vec<JsValue> {
    holders.iter().map(|holder| holder.value.clone()).collect()
}

/// The core's view over its layers: what `createPolicyView` returns.
#[derive(Debug)]
pub struct LayersView {
    /// The layers as the host gave them (whatever other members they carry), highest first.
    given: Vec<JsValue>,
    layers: Vec<PolicyLayer>,
    managed: Vec<JsValue>,
    // The layers never change, so each key's holders are found once.
    found: Mutex<HashMap<String, Arc<Vec<KeyHolder>>>>,
}

impl LayersView {
    fn holders(&self, key: &str) -> Arc<Vec<KeyHolder>> {
        let mut found = self.found.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(known) = found.get(key) {
            return Arc::clone(known);
        }
        let fresh = Arc::new(holders_of(&self.layers, key));
        found.insert(key.to_owned(), Arc::clone(&fresh));
        fresh
    }

    /// The layers, highest precedence first.
    #[must_use]
    pub fn ordered(&self) -> &[PolicyLayer] {
        &self.layers
    }

    /// The key as the files combine it (`undefined` when no file sets it).
    #[must_use]
    pub fn value(&self, key: &str) -> JsValue {
        let floors = floors_of(&self.layers, key);
        let counted = counted_holders(key, &self.holders(key), &self.managed);
        if counted.is_empty() && floors.is_empty() {
            JsValue::Undefined
        } else {
            combine(key, &counted, floors)
        }
    }

    /// The highest layer that sets the key.
    #[must_use]
    pub fn top(&self, key: &str) -> Option<KeyHolder> {
        self.holders(key).first().cloned()
    }

    /// Every layer that sets the key, highest first.
    #[must_use]
    pub fn all(&self, key: &str) -> Vec<KeyHolder> {
        self.holders(key).to_vec()
    }
}

impl PolicyView for LayersView {
    fn layers(&self) -> BoxFuture<'_, Vec<PolicyLayer>> {
        Box::pin(std::future::ready(self.layers.clone()))
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<JsValue>> {
        Box::pin(std::future::ready(
            Some(self.value(key)).filter(|value| !value.is_undefined()),
        ))
    }

    fn winner<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<KeyHolder>> {
        Box::pin(std::future::ready(self.top(key)))
    }

    fn each_from_highest<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Vec<KeyHolder>> {
        Box::pin(std::future::ready(self.all(key)))
    }
}

/// The view over `given`, ordered; a value that is no layer counts as none (the package's layers
/// are typed, so every caller passes layers).
#[must_use]
pub fn create_policy_view(given: &[JsValue]) -> LayersView {
    let ordered = ordered_layers(given);
    let layers: Vec<PolicyLayer> = ordered
        .iter()
        .filter_map(|layer| PolicyLayer::deserialize(layer.clone()).ok())
        .collect();
    let managed = layers
        .iter()
        .filter(|layer| layer.scope == PolicyScope::Managed)
        .map(|layer| layer.settings.clone())
        .collect();
    LayersView {
        given: ordered,
        layers,
        managed,
        found: Mutex::new(HashMap::new()),
    }
}

/// A question of the view as the host calls it.
#[derive(Debug)]
enum Question {
    Get,
    Winner,
    EachFromHighest,
}

#[derive(Debug)]
struct Asked(Arc<LayersView>, Question);

impl Asked {
    fn answer(&self, key: &str) -> JsValue {
        let answer = match self.1 {
            Question::Get => return self.0.value(key),
            Question::Winner => crate::js::value::to_js_value(&self.0.top(key)),
            Question::EachFromHighest => crate::js::value::to_js_value(&self.0.all(key)),
        };
        answer.unwrap_or_default()
    }
}

crate::registry::methods! {
    Asked {
        "call" => ok |this, key: String| this.answer(&key);
    }
}

/// The view as the host holds it: the layers as data, each question a function of the core's.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostView {
    layers: Vec<JsValue>,
    get: Function,
    winner: Function,
    each_from_highest: Function,
}

/// `view` as the host holds it; the functions share it.
pub(crate) fn host_view(view: &Arc<LayersView>) -> HostView {
    let asked = |question| Function::new(Asked(Arc::clone(view), question));
    HostView {
        layers: view.given.clone(),
        get: asked(Question::Get),
        winner: asked(Question::Winner),
        each_from_highest: asked(Question::EachFromHighest),
    }
}

crate::registry::exports! {
    "settings/view" {
        "createPolicyView" => ok |given: Vec<JsValue>| host_view(&Arc::new(create_policy_view(&given)));
    }
}
