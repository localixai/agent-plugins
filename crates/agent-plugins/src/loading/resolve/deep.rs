//! Port of `src/loading/resolve/deep.ts`, switched.
//!
//! Every string of a JSON value substituted, each with the policy of its path.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use super::substitute::{FieldPolicy, PolicyArg, SubstitutionScope, TokenReference};
use super::substituter::{Substituter, substituter};
use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue, number_to_string, promise_all};
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::Callback;

/// One step of a `ValuePath`: a member's key or an element's index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathPart {
    /// An element's index.
    Index(f64),
    /// A member's key.
    Key(String),
}

/// Where a string sits in the value.
pub type ValuePath = Vec<PathPart>;

/// `(path) => FieldPolicy`: the policy of the string at a path.
pub trait PolicyFor: Send + Sync {
    /// The policy at `path`.
    fn policy_for<'a>(&'a self, path: &'a [PathPart]) -> BoxFuture<'a, FieldPolicy>;
}

impl<F: Fn(&[PathPart]) -> FieldPolicy + Send + Sync> PolicyFor for F {
    fn policy_for<'a>(&'a self, path: &'a [PathPart]) -> BoxFuture<'a, FieldPolicy> {
        Box::pin(std::future::ready(self(path)))
    }
}

impl PolicyFor for Callback {
    fn policy_for<'a>(&'a self, path: &'a [PathPart]) -> BoxFuture<'a, FieldPolicy> {
        let argument = crate::js::value::to_js_value(&path).unwrap_or_default();
        let asked = self.call::<PolicyArg>(vec![argument]);
        Box::pin(async move { asked.await.read().await })
    }
}

/// The value substituted.
#[derive(Clone, Debug, Serialize)]
pub struct DeepSubstituted {
    /// The input with every string substituted; callers narrow it (a schema parse), the walk keeps
    /// no type.
    pub value: JsValue,
    /// The issues.
    pub issues: Vec<PluginIssue>,
    /// What the strings referenced.
    pub referenced: Vec<TokenReference>,
}

struct Walk<'a, L, E> {
    substitute: Substituter<'a, L, E>,
    policy_for: &'a dyn PolicyFor,
    base: &'a str,
    issues: Mutex<Vec<PluginIssue>>,
    referenced: Mutex<Vec<TokenReference>>,
}

fn name_of(base: &str, path: &[PathPart]) -> String {
    let parts = std::iter::once(base.to_owned()).chain(path.iter().map(|part| match part {
        PathPart::Index(index) => number_to_string(*index),
        PathPart::Key(key) => key.clone(),
    }));
    parts
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

fn walk<'w, 's: 'w, L: Locations + Clone + 's, E: Environment + 's>(
    value: &'w JsValue,
    path: ValuePath,
    state: &'w Walk<'s, L, E>,
) -> BoxFuture<'w, JsValue> {
    Box::pin(async move {
        match value {
            JsValue::String(text) => {
                let policy = state.policy_for.policy_for(&path).await;
                let name = name_of(state.base, &path);
                let done = state.substitute.run(text, &policy, &name).await;
                lock(&state.issues).extend(done.issues);
                lock(&state.referenced).extend(done.referenced);
                JsValue::String(done.text)
            }
            JsValue::Array(items) => {
                let walks = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        #[expect(clippy::cast_precision_loss, reason = "an index of a JSON array")]
                        let at = PathPart::Index(index as f64);
                        walk(item, [path.clone(), vec![at]].concat(), state)
                    })
                    .collect();
                JsValue::Array(promise_all(walks).await)
            }
            JsValue::Object(members) => {
                let walks = members
                    .iter()
                    .map(|(key, item)| {
                        walk(
                            item,
                            [path.clone(), vec![PathPart::Key(key.clone())]].concat(),
                            state,
                        )
                    })
                    .collect();
                let values = promise_all(walks).await;
                JsValue::Object(members.keys().cloned().zip(values).collect::<JsObject>())
            }
            JsValue::Undefined | JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) => {
                value.clone()
            }
        }
    })
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Substitutes every string of a JSON value (an MCP server's `args` and `env`, an LSP server), each
/// with the policy of its path; numbers, booleans and `null` stay. The host is asked once per
/// variable for the whole value. Issues name the field (`env.TOKEN`, `args.1`), under `base` when
/// given.
pub async fn substitute_deep<L: Locations + Clone, E: Environment>(
    value: &JsValue,
    scope: &SubstitutionScope<L, E>,
    policy_for: &dyn PolicyFor,
    base: &str,
) -> DeepSubstituted {
    let state = Walk {
        substitute: substituter(scope),
        policy_for,
        base,
        issues: Mutex::new(Vec::new()),
        referenced: Mutex::new(Vec::new()),
    };
    let done = walk(value, Vec::new(), &state).await;
    DeepSubstituted {
        value: done,
        issues: state
            .issues
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner),
        referenced: state
            .referenced
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner),
    }
}

crate::registry::exports! {
    "loading/resolve/deep" {
        "substituteDeep" => async ok |value: JsValue, scope: SubstitutionScope, policy_for: Callback, base: Option<String>|
            substitute_deep(&value, &scope, &policy_for, &base.unwrap_or_default()).await;
    }
}
