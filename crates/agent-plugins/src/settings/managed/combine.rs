//! Port of `src/settings/managed/combine.ts`, switched.
//!
//! How `managedSourcesBehavior: "merge"` combines the sources, by kind of key (managed-settings.md,
//! "Compose every managed source"; settings-reference.md, `managedSourcesBehavior`). One combiner
//! per kind, one table from key to kind; a key with no row is an "other key" (the value of the
//! highest source that sets it), and a block with no row merges key by key so its lists and locks
//! follow their own rows (spec/judgments.md).

use std::sync::LazyLock;

use indexmap::{IndexMap, IndexSet};

use crate::js::{JsObject, JsValue, in_property_order, property_order, strict_equals};
use crate::protocol::is_record::is_record;
use crate::settings::hooks::merge_groups::is_hooks_settings;
use crate::settings::merge::MERGES;
use crate::settings::paths::{SettingsPath, value_at};

/// What a combiner reads besides the values: the sources' repaired settings, highest first.
pub struct CombineContext<'a> {
    /// The sources' repaired settings, highest first.
    pub docs: &'a [JsValue],
}

/// How one kind of key combines: the values one key has in the sources, highest first
/// (`undefined` where a source leaves it unset), to its value (`undefined` for none).
pub type Combiner = Box<dyn Fn(&[JsValue], &CombineContext<'_>) -> JsValue + Send + Sync>;

fn defined(values: &[JsValue]) -> Vec<JsValue> {
    values
        .iter()
        .filter(|value| !value.is_undefined())
        .cloned()
        .collect()
}

/// "Takes the value from the highest-ranked source that sets it."
fn highest_set(values: &[JsValue], _: &CombineContext<'_>) -> JsValue {
    defined(values).into_iter().next().unwrap_or_default()
}

/// "Reads the key only from the highest-priority source": a lower source's value is ignored even when this one sets none.
fn highest_source_only(values: &[JsValue], _: &CombineContext<'_>) -> JsValue {
    values.first().cloned().unwrap_or_default()
}

/// "Combines the entries from every source", duplicates once.
fn union_lists(values: &[JsValue], _: &CombineContext<'_>) -> JsValue {
    let lists: Vec<JsValue> = defined(values)
        .into_iter()
        .filter(JsValue::is_array)
        .collect();
    (MERGES.union_arrays.combine)(&lists)
}

/// The `hooks` key: every source's groups, the highest source's first.
fn hook_groups(values: &[JsValue], _: &CombineContext<'_>) -> JsValue {
    let files: Vec<JsValue> = defined(values)
        .into_iter()
        .filter(is_hooks_settings)
        .collect();
    (MERGES.hook_groups.combine)(&files)
}

/// "Combines the server names from every source; when two sources set the same name, applies the higher-ranked source's whole entry."
fn servers_by_name(values: &[JsValue], _: &CombineContext<'_>) -> JsValue {
    let maps: Vec<JsValue> = defined(values).into_iter().filter(is_record).collect();
    (MERGES.per_key_highest_wins.combine)(&maps)
}

/// "Applies the strictest value any source sets; a looser value applies only from the
/// highest-ranked source." The ladder lists the strict values from the least to the most strict;
/// any other value (`false`, `"accept"`, `"prefix"`) is a looser one, which counts only where the
/// highest-ranked source sets it (a lower source's looser value is ignored, even when the highest
/// sets none).
fn lock(ladder: Vec<JsValue>) -> Combiner {
    Box::new(move |values, _| {
        let strictness = |value: &JsValue| -> i64 {
            ladder
                .iter()
                .position(|rung| strict_equals(rung, value))
                .and_then(|place| i64::try_from(place).ok())
                .unwrap_or(-1)
        };
        let strictest = defined(values)
            .into_iter()
            .fold(JsValue::Undefined, |best, value| {
                if strictness(&value) > strictness(&best) {
                    value
                } else {
                    best
                }
            });
        if strictness(&strictest) >= 0 {
            strictest
        } else {
            values.first().cloned().unwrap_or_default()
        }
    })
}

/// `env`: "each variable comes from the highest-priority source that defines it", and the
/// telemetry unit (`OTEL_EXPORTER_OTLP_*`, `OTEL_LOG_*`, `OTEL_LOGS_EXPORTER`, the two beta tracing
/// variables) follows the highest source that claims it as one (server-managed-settings.md,
/// "Per-key exceptions across managed sources"). A source claims the unit by setting any of its
/// variables, or by delivering `otelHeadersHelper`; one that only delivers the helper lands none of
/// the variables and still blocks lower sources from filling them in.
///
/// `TELEMETRY_UNIT`: `/^(OTEL_EXPORTER_OTLP_|OTEL_LOG_|OTEL_LOGS_EXPORTER$|ENABLE_BETA_TRACING_DETAILED$|BETA_TRACING_ENDPOINT$)/`.
fn in_unit(name: &str) -> bool {
    name.starts_with("OTEL_EXPORTER_OTLP_")
        || name.starts_with("OTEL_LOG_")
        || matches!(
            name,
            "OTEL_LOGS_EXPORTER" | "ENABLE_BETA_TRACING_DETAILED" | "BETA_TRACING_ENDPOINT"
        )
}

fn env_block(value: &JsValue) -> Vec<(&String, &JsValue)> {
    value.as_object().map(property_order).unwrap_or_default()
}

fn delivers_helper(source: Option<&JsValue>) -> bool {
    source
        .and_then(|doc| doc.get("otelHeadersHelper"))
        .is_some_and(|helper| !helper.is_undefined())
}

fn claims_unit(env: &JsValue, source: Option<&JsValue>) -> bool {
    env_block(env).into_iter().any(|(name, _)| in_unit(name)) || delivers_helper(source)
}

fn combine_env(values: &[JsValue], context: &CombineContext<'_>) -> JsValue {
    let docs = context.docs;
    let claimant = values
        .iter()
        .enumerate()
        .position(|(index, env)| claims_unit(env, docs.get(index)));
    // A claimant below the selected source that delivers `otelHeadersHelper` lands none of the unit.
    let lands = claimant == Some(0) || !delivers_helper(claimant.and_then(|index| docs.get(index)));
    let mut merged: JsObject = IndexMap::new();
    for (index, value) in values.iter().enumerate().rev() {
        for (name, entry) in env_block(value) {
            let unit_variable = in_unit(name);
            if !unit_variable || (Some(index) == claimant && lands) {
                merged.insert(name.clone(), entry.clone());
            }
        }
    }
    // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` name stays a name.
    if merged.is_empty() {
        JsValue::Undefined
    } else {
        JsValue::Object(in_property_order(merged))
    }
}

/// Restriction allowlists and values taken whole: "takes the list whole from the highest source
/// that sets it". `modelOverrides` is taken "from the highest source that sets it" as well
/// (settings-reference.md, `managedSourcesBehavior`; its pairing with `availableModels` is `compose.ts`).
const WHOLE: [&str; 9] = [
    "modelOverrides",
    "availableModels",
    "allowedMcpServers",
    "allowedProviders",
    "strictKnownMarketplaces",
    "allowedChannelPlugins",
    "fallbackModel",
    "sandbox.credentials.awsPairs",
    "sandbox.ripgrep",
];

/// "Read from the highest-priority source only" (the login keys have a rule of their own, `login-keys.ts`).
const HIGHEST_ONLY: [&str; 10] = [
    "apiKeyHelper",
    "awsAuthRefresh",
    "awsCredentialExport",
    "gcpAuthRefresh",
    "otelHeadersHelper",
    "proxyAuthHelper",
    "parentSettingsBehavior",
    "modelPicker",
    "policyHelper",
    "permissions.defaultMode",
];

/// The locks, each with its strict values from the least to the most strict. The docs name four
/// (`allowManagedHooksOnly`, `permissions.disableBypassPermissionsMode`, `crossSessionInbound`,
/// `availableModelsMatch`) and "other boolean or enum locks"; the rest are the keys the docs
/// describe as restricting (spec/judgments.md). `"exact"` is the strict `availableModelsMatch` (an
/// invalid value reads as it), and `"hold"` and `"refuse"` the strict `crossSessionInbound`
/// (`"refuse"` is "the most restrictive value").
fn lock_ladders() -> Vec<(&'static str, Vec<JsValue>)> {
    let on = || vec![JsValue::Bool(true)];
    let texts = |rungs: &[&str]| rungs.iter().map(|rung| JsValue::from(*rung)).collect();
    vec![
        ("allowManagedHooksOnly", on()),
        ("allowManagedMcpServersOnly", on()),
        ("allowManagedPermissionRulesOnly", on()),
        ("sandbox.filesystem.allowManagedReadPathsOnly", on()),
        ("sandbox.network.allowManagedDomainsOnly", on()),
        ("sandbox.network.strictAllowlist", on()),
        ("disableCommandPluginSources", on()),
        ("disableSideloadFlags", on()),
        ("disableAutoMode", texts(&["disable"])),
        (
            "permissions.disableBypassPermissionsMode",
            texts(&["disable"]),
        ),
        ("crossSessionInbound", texts(&["hold", "refuse"])),
        ("availableModelsMatch", texts(&["exact"])),
    ]
}

static RULES: LazyLock<IndexMap<&'static str, Combiner>> = LazyLock::new(|| {
    let mut rules: IndexMap<&'static str, Combiner> = IndexMap::new();
    for path in WHOLE {
        rules.insert(path, Box::new(highest_set));
    }
    for path in HIGHEST_ONLY {
        rules.insert(path, Box::new(highest_source_only));
    }
    for (path, ladder) in lock_ladders() {
        rules.insert(path, lock(ladder));
    }
    rules.insert("managedMcpServers", Box::new(servers_by_name));
    rules.insert("hooks", Box::new(hook_groups));
    rules.insert("env", Box::new(combine_env));
    rules
});

fn combine_values(path: &str, values: &[JsValue], context: &CombineContext<'_>) -> JsValue {
    if let Some(rule) = RULES.get(path) {
        return rule(values, context);
    }
    let set = defined(values);
    let Some(first) = set.first() else {
        return JsValue::Undefined;
    };
    if set.iter().all(JsValue::is_array) {
        return union_lists(values, context);
    }
    if set.iter().all(is_record) {
        let blocks: Vec<Option<&JsObject>> = values.iter().map(JsValue::as_object).collect();
        return JsValue::Object(combine_blocks(path, &blocks, context));
    }
    first.clone()
}

/// The blocks key by key, each key by its own row.
fn combine_blocks(
    path: &str,
    blocks: &[Option<&JsObject>],
    context: &CombineContext<'_>,
) -> JsObject {
    let names: IndexSet<&String> = blocks
        .iter()
        .flatten()
        .flat_map(|block| property_order(block).into_iter().map(|(name, _)| name))
        .collect();
    let entries: JsObject = names
        .into_iter()
        .filter_map(|name| {
            let joined = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            let values: Vec<JsValue> = blocks
                .iter()
                .map(|block| {
                    block
                        .and_then(|block| block.get(name))
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();
            let value = combine_values(&joined, &values, context);
            (!value.is_undefined()).then(|| (name.clone(), value))
        })
        .collect();
    // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` name stays a name.
    in_property_order(entries)
}

/// The settings of the sources (highest first) as `"merge"` combines them.
#[must_use]
pub fn combine_sources(docs: &[JsValue]) -> JsValue {
    let blocks: Vec<Option<&JsObject>> = docs.iter().map(JsValue::as_object).collect();
    JsValue::Object(combine_blocks("", &blocks, &CombineContext { docs }))
}

/// One key across the sources (highest first) by its row: what a cross-source key needs without the rest of the settings.
#[must_use]
pub fn combine_key(path: &str, docs: &[JsValue]) -> JsValue {
    let at = SettingsPath::Dotted(path.to_owned());
    let values: Vec<JsValue> = docs.iter().map(|doc| value_at(doc, &at)).collect();
    combine_values(path, &values, &CombineContext { docs })
}

crate::registry::exports! {
    "settings/managed/combine" {
        "combineSources" => ok |docs: Vec<JsValue>| combine_sources(&docs);
    }
}
