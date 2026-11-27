//! Port of `src/settings/context.ts`, switched.
//!
//! What a host tells the policy besides the layers: plain data it copies in, so the kit reads no
//! `process.env`, no file and no account. Every field is a fact the docs make a decision depend on
//! and no settings file holds.
//!
//! The facts cross as data, or, when they hold the host's `isFolderTrusted`, as a port whose members
//! are read by name, the function among them as the host's own (`{"$fn": n}`), given back as it came.
//! A member a port answers `undefined` is left out: the spread cannot tell it from an absent one
//! there. The default `isFolderTrusted` is a function of the core's.

use serde::Deserialize;

use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, promise_all};
use crate::registry::{Function, Port};
use crate::settings::marketplaces::registry::OFFICIAL_MARKETPLACE;

/// The members of a `PolicyContext`, in its order.
const CONTEXT_KEYS: [&str; 14] = [
    "account",
    "plan",
    "hasManagedSettings",
    "parentMerge",
    "isFolderTrusted",
    "localSettingsTracked",
    "localSettingsInRepository",
    "env",
    "defaultChannelAllowlist",
    "officialMarketplaces",
    "orgWorkflowsOff",
    "safeMode",
    "workflowSizeChoice",
    "ultracode",
];

/// The facts a host knows: data, or a record holding its `isFolderTrusted`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PolicyFacts {
    /// A record holding a function.
    Host(Port),
    /// Plain data.
    Data(JsObject),
}

/// `() => false`: no folder is trusted.
struct Untrusted;

crate::registry::methods! {
    Untrusted {
        "call" => ok |this, _scope: JsValue| false;
    }
}

/// The facts' own members, in their order.
async fn members_of(facts: PolicyFacts) -> JsObject {
    match facts {
        PolicyFacts::Data(members) => members,
        PolicyFacts::Host(port) => {
            let read = CONTEXT_KEYS.map(|key| Box::pin(port.member::<JsValue>(key)) as _);
            let values = promise_all(read.into_iter().collect()).await;
            CONTEXT_KEYS
                .into_iter()
                .zip(values)
                .filter(|(_, value)| !value.is_undefined())
                .map(|(key, value)| (key.to_owned(), value))
                .collect()
        }
    }
}

/// A `PolicyContext` from the facts a host knows; every fact it leaves out takes the value that grants
/// the least (spec/judgments.md): an API-key (`console`) account without Pro, no managed settings and no
/// parent merge, no folder trusted, an empty environment, no default channel allowlist, and the one
/// official marketplace the docs name (plugins/loading.md).
pub async fn policy_context(facts: Option<PolicyFacts>) -> JsObject {
    let mut plan = JsObject::new();
    plan.insert("pro".to_owned(), JsValue::Bool(false));
    let untrusted = to_js_value(&Function::new(Untrusted)).unwrap_or_default();
    let defaults: [(&str, JsValue); 8] = [
        ("account", JsValue::from("console")),
        ("plan", JsValue::Object(plan)),
        ("hasManagedSettings", JsValue::Bool(false)),
        ("parentMerge", JsValue::Bool(false)),
        ("isFolderTrusted", untrusted),
        ("env", JsValue::Object(JsObject::new())),
        ("defaultChannelAllowlist", JsValue::Array(Vec::new())),
        (
            "officialMarketplaces",
            JsValue::Array(vec![JsValue::from(OFFICIAL_MARKETPLACE)]),
        ),
    ];
    let mut context: JsObject = defaults
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    if let Some(facts) = facts {
        context.extend(members_of(facts).await);
    }
    context
}

crate::registry::exports! {
    "settings/context" {
        "policyContext" => async ok |facts: Option<PolicyFacts>| policy_context(facts).await;
    }
}
