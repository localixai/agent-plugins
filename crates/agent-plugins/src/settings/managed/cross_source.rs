//! Port of `src/settings/managed/cross-source.ts`, switched.
//!
//! The keys Claude Code reads from every admin source, under both settings of
//! `managedSourcesBehavior` (managed-settings.md, "Keys read from every admin source"): "a
//! lower-ranked MDM policy or managed settings file can still set them when the selected source
//! doesn't". The user-writable HKCU registry is left out of the scan; the caller passes the admin
//! sources only, highest first.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue, in_property_order, json_stringify, strict_equals};
use crate::protocol::is_record::is_record;
use crate::settings::canonical::union_of;
use crate::settings::general::exceptions::{
    FALSE_FROM_ANY_SCOPE, LOWEST_CAP, TRUE_FROM_ANY_SCOPE, strictest_of,
};
use crate::settings::managed::combine::combine_key;
use crate::settings::merge::MERGES;
use crate::settings::paths::{SettingsPath, value_at, with_value_at};

/// What the cross-source keys are read from.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossSourceInput {
    /// The repaired settings of every admin source that delivers a policy key, highest first.
    pub admins: Vec<JsValue>,
    /// Whether each of those is a machine source (an MDM policy or a managed file), not server-managed settings.
    pub on_machine: Vec<bool>,
    /// `"merge"`: the machine's lists combine by the table rather than the highest alone.
    pub merging: bool,
}

type Rule = Box<dyn Fn(&CrossSourceInput, JsValue) -> JsValue + Send + Sync>;

fn at(doc: &JsValue, path: &str) -> JsValue {
    value_at(doc, &SettingsPath::Dotted(path.to_owned()))
}

fn set_in(result: JsValue, path: &str, value: &JsValue) -> JsValue {
    if value.is_undefined() {
        result
    } else {
        with_value_at(&result, &SettingsPath::Dotted(path.to_owned()), value)
    }
}

fn strings(value: &JsValue) -> Vec<JsValue> {
    value
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry.is_string())
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn first_set_in(admins: &[JsValue], path: &str) -> JsValue {
    admins
        .iter()
        .map(|doc| at(doc, path))
        .find(|value| !value.is_undefined())
        .unwrap_or_default()
}

/// The most restrictive value any admin source sets on the key's ladder: "A `true` in any admin
/// source turns the lock on", "A `false` from any admin source turns the behavior off", "The lowest
/// cap in any admin source applies". The ladders are the key rows' own (general/exceptions.ts).
fn strictest_of_admins(ladder: Vec<JsValue>, paths: &'static [&'static str]) -> Rule {
    Box::new(move |input, result| {
        paths.iter().fold(result, |current, path| {
            let on_ladder: Vec<JsValue> = input
                .admins
                .iter()
                .map(|doc| at(doc, path))
                .filter(|value| ladder.iter().any(|rung| strict_equals(rung, value)))
                .collect();
            set_in(current, path, &strictest_of(ladder.clone(), &on_ladder))
        })
    })
}

/// A key read from the highest admin source that sets it, whether or not the selected source does.
fn from_highest_admin(paths: &'static [&'static str]) -> Rule {
    Box::new(move |input, result| {
        paths.iter().fold(result, |current, path| {
            set_in(current, path, &first_set_in(&input.admins, path))
        })
    })
}

/// "An entry in any admin source applies": the lists of every source, together (entries may be objects).
fn union_of_admins(path: &'static str) -> Rule {
    Box::new(move |input, result| {
        let lists: Vec<JsValue> = input
            .admins
            .iter()
            .map(|doc| at(doc, path))
            .filter(JsValue::is_array)
            .collect();
        let entries = (MERGES.union_arrays.combine)(&lists);
        if entries.as_array().is_none_or(Vec::is_empty) {
            result
        } else {
            set_in(result, path, &entries)
        }
    })
}

fn is_on(settings: &JsValue, path: &str) -> bool {
    at(settings, path) == JsValue::Bool(true)
}

/// While `lock` is on, `path` is the union over every admin source; `extra` adds entries of another key.
fn locked_allowlist(lock: &'static str, path: &'static str) -> Rule {
    Box::new(move |input, result| {
        if !is_on(&result, lock) {
            return result;
        }
        let lists: Vec<Vec<JsValue>> = input
            .admins
            .iter()
            .map(|doc| strings(&at(doc, path)))
            .collect();
        set_in(result, path, &JsValue::Array(union_of(&lists)))
    })
}

const WEBFETCH_DOMAIN: &str = "WebFetch(domain:";

/// `WebFetch(domain:...)` allow rules count in the allowlist that `allowManagedDomainsOnly` locks.
fn locked_web_fetch_rules(input: &CrossSourceInput, result: JsValue) -> JsValue {
    if !is_on(&result, "sandbox.network.allowManagedDomainsOnly") {
        return result;
    }
    let fetch_rules: Vec<JsValue> = input
        .admins
        .iter()
        .flat_map(|doc| strings(&at(doc, "permissions.allow")))
        .filter(|rule| {
            rule.as_str()
                .is_some_and(|text| text.starts_with(WEBFETCH_DOMAIN))
        })
        .collect();
    let kept = strings(&at(&result, "permissions.allow"));
    let allow = JsValue::Array(union_of(&[kept, fetch_rules]));
    set_in(result, "permissions.allow", &allow)
}

/// While `allowManagedMcpServersOnly` is on, "the managed `allowedMcpServers` list comes from the
/// highest-ranked admin source that sets one. A server-managed list replaces a lower source's list
/// rather than combining with it."
fn locked_mcp_allowlist(input: &CrossSourceInput, result: JsValue) -> JsValue {
    if is_on(&result, "allowManagedMcpServersOnly") {
        let list = first_set_in(&input.admins, "allowedMcpServers");
        set_in(result, "allowedMcpServers", &list)
    } else {
        result
    }
}

/// A commit-trailer opt-out: `attribution: false`, an empty `attribution.commit`, or `includeCoAuthoredBy: false`.
fn opts_out(doc: &JsValue) -> bool {
    let attribution = doc.get("attribution").cloned().unwrap_or_default();
    attribution == JsValue::Bool(false)
        || (is_record(&attribution) && attribution.get("commit") == Some(&JsValue::from("")))
        || (doc.get("includeCoAuthoredBy") == Some(&JsValue::Bool(false))
            && attribution.is_undefined())
}

fn commit_trailer_opt_out(input: &CrossSourceInput, result: JsValue) -> JsValue {
    if !input.admins.iter().any(opts_out) {
        return result;
    }
    let value = match result.get("attribution").and_then(JsValue::as_object) {
        Some(attribution) => {
            let mut spread: JsObject = attribution.clone();
            spread.insert("commit".to_owned(), JsValue::from(""));
            JsValue::Object(in_property_order(spread))
        }
        None => JsValue::Bool(false),
    };
    set_in(result, "attribution", &value)
}

/// `env`: "merged per variable across the admin sources".
fn merged_env(input: &CrossSourceInput, result: JsValue) -> JsValue {
    set_in(result, "env", &combine_key("env", &input.admins))
}

/// `allowedProviders`: "a list that the machine's own admin sources set keeps applying when
/// server-managed settings also deliver one: a session may then use only the providers on both
/// lists". The machine's list is the highest machine source's under `"first-wins"`, and the highest
/// machine source that sets one under `"merge"`.
fn allowed_providers(input: &CrossSourceInput, result: JsValue) -> JsValue {
    let on_machine = |index: usize| input.on_machine.get(index).copied().unwrap_or(false);
    let machine: Vec<JsValue> = input
        .admins
        .iter()
        .enumerate()
        .filter(|(index, _)| on_machine(*index))
        .map(|(_, doc)| doc.clone())
        .collect();
    let providers_of = |doc: Option<&JsValue>| {
        doc.and_then(|doc| doc.get("allowedProviders"))
            .cloned()
            .unwrap_or_default()
    };
    let machine_list = if input.merging {
        first_set_in(&machine, "allowedProviders")
    } else {
        providers_of(machine.first())
    };
    let remote = providers_of(
        input
            .admins
            .iter()
            .enumerate()
            .find(|(index, _)| !on_machine(*index))
            .map(|(_, doc)| doc),
    );
    let Some(machine_entries) = machine_list.as_array() else {
        return result;
    };
    let Some(remote_entries) = remote.as_array() else {
        return set_in(result, "allowedProviders", &machine_list);
    };
    // `includes` is SameValueZero: `NaN` is found too.
    let on_both: Vec<JsValue> = machine_entries
        .iter()
        .filter(|provider| {
            remote_entries
                .iter()
                .any(|one| same_value_zero(one, provider))
        })
        .cloned()
        .collect();
    set_in(result, "allowedProviders", &JsValue::Array(on_both))
}

fn same_value_zero(a: &JsValue, b: &JsValue) -> bool {
    strict_equals(a, b)
        || matches!((a, b), (JsValue::Number(x), JsValue::Number(y)) if x.is_nan() && y.is_nan())
}

static CROSS_SOURCE_RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        strictest_of_admins(
            TRUE_FROM_ANY_SCOPE.ladder.clone(),
            &[
                "sandbox.network.allowManagedDomainsOnly",
                "sandbox.filesystem.allowManagedReadPathsOnly",
                "sandbox.network.strictAllowlist",
                "allowManagedMcpServersOnly",
                "disableClaudeAiConnectors",
                "forceRemoteSettingsRefresh",
            ],
        ),
        strictest_of_admins(
            FALSE_FROM_ANY_SCOPE.ladder.clone(),
            &[
                "useAutoModeDuringPlan",
                "syncClaudeAiSkills",
                "syncClaudeAiPlugins",
                "enableArtifact",
            ],
        ),
        from_highest_admin(&[
            "allowAllClaudeAiMcps",
            "sandbox.bwrapPath",
            "sandbox.socatPath",
            "sandbox.ripgrep",
            "sandbox.filesystem.disabled",
        ]),
        union_of_admins("deniedMcpServers"),
        locked_allowlist(
            "sandbox.network.allowManagedDomainsOnly",
            "sandbox.network.allowedDomains",
        ),
        Box::new(locked_web_fetch_rules),
        locked_allowlist(
            "sandbox.filesystem.allowManagedReadPathsOnly",
            "sandbox.filesystem.allowRead",
        ),
        Box::new(locked_mcp_allowlist),
        strictest_of_admins(LOWEST_CAP.ladder.clone(), &["maxEffortLevel"]),
        Box::new(commit_trailer_opt_out),
        Box::new(merged_env),
        Box::new(allowed_providers),
    ]
});

/// The managed settings with every cross-source key read from all the admin sources.
#[must_use]
pub fn with_cross_source_keys(input: &CrossSourceInput, selected: JsValue) -> JsValue {
    CROSS_SOURCE_RULES
        .iter()
        .fold(selected, |result, rule| rule(input, result))
}

/// `/status` names a source whose lock or allowlist is not the one in force (managed-settings.md,
/// the end of "Keys read from every admin source").
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InForceNote {
    /// The source's place among the admin sources.
    pub index: usize,
    /// `allowManagedMcpServersOnly` or `allowedMcpServers`.
    pub key: &'static str,
}

const NOTED_KEYS: [&str; 2] = ["allowManagedMcpServersOnly", "allowedMcpServers"];

/// The sources whose noted key is set to another value than the one in force.
#[must_use]
pub fn not_in_force(admins: &[JsValue], result: &JsValue) -> Vec<InForceNote> {
    admins
        .iter()
        .enumerate()
        .flat_map(|(index, doc)| {
            NOTED_KEYS.into_iter().filter_map(move |key| {
                let set = doc.get(key).cloned().unwrap_or_default();
                let in_force = result.get(key).cloned().unwrap_or_default();
                (!set.is_undefined() && json_stringify(&set) != json_stringify(&in_force))
                    .then_some(InForceNote { index, key })
            })
        })
        .collect()
}

crate::registry::exports! {
    "settings/managed/cross-source" {
        "withCrossSourceKeys" => ok |input: CrossSourceInput, selected: JsValue|
            with_cross_source_keys(&input, selected);
        "notInForce" => ok |admins: Vec<JsValue>, result: JsValue| not_in_force(&admins, &result);
    }
}
