//! Port of `src/settings/sources.ts`, switched.
//!
//! Where a host's settings come from. The kit knows no file path, MDM profile, registry value or
//! server-managed fetch: the host reads each source it has and hands over its raw JSON, either
//! parsed or as text. Choosing among managed sources (`managedSourcesBehavior`, `policyHelper`) is
//! the host's, not the kit's. Optional: a host may build layers with `readSettingsLayer` itself.
//!
//! The host's `SettingsSources` is a port: `read(scope, signal)`, and the optional `label(scope)`
//! and `managed(signal)`, each asked as its method.

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::json_parse::json_parse;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, promise_all, trim};
use crate::ports::signal::Signal;
use crate::registry::{Callback, Port};
use crate::settings::broken_file::{broken_source, stops_startup};
use crate::settings::layers::{POLICY_SCOPES, PolicyLayer, PolicyScope};
use crate::settings::managed::compose::{ComposeOptions, ManagedBehavior, ManagedComposition};
use crate::settings::managed::load::{ManagedReads, compose_managed_reads};
use crate::settings::managed::parent::{ParentComposition, parent_settings_of};
use crate::settings::read_layer::read_settings_layer;

/// A source's JSON as a value, or the issue that skips it.
type Parsed = Result<JsValue, PluginIssue>;

fn parse_source(scope: PolicyScope, raw: JsValue, source: Option<&str>) -> Parsed {
    let JsValue::String(text) = &raw else {
        return Ok(raw);
    };
    // An empty `managed-settings.json` counts as `{}` (errors.md).
    if trim(text).is_empty() {
        return Ok(JsValue::Object(JsObject::new()));
    }
    json_parse(text).map_err(|why| broken_source(scope, Some(&why), source))
}

/// What loading every source came to.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LoadedSettings {
    /// The sources that exist and could be read, highest first; none when `fatal`.
    pub layers: Vec<PolicyLayer>,
    /// What reading them reported.
    pub issues: Vec<PluginIssue>,
    /// A managed document that is present and does not parse as a JSON object: Claude Code "exits with
    /// code 1 at startup instead of running without the policy the document carries" (errors.md). The
    /// host stops and reports `issues`; no layer is returned, so none runs without the policy.
    pub fatal: bool,
}

fn signal_arg(signal: &Signal) -> JsValue {
    to_js_value(signal).unwrap_or_default()
}

/// Reads each of `scopes` through `sources.read`, a source at a time into a layer.
async fn load_scopes(sources: Port, signal: &Signal, scopes: &[PolicyScope]) -> LoadedSettings {
    let reads = scopes
        .iter()
        .map(|scope| {
            let args = vec![JsValue::from(scope.name()), signal_arg(signal)];
            Box::pin(sources.method::<JsValue>("read", args))
                as crate::ports::BoxFuture<'_, JsValue>
        })
        .collect();
    let raws = promise_all(reads).await;
    let labelled = sources.member::<Option<Callback>>("label").await.is_some();
    let mut layers = Vec::new();
    let mut issues = Vec::new();
    let mut fatal = false;
    for (scope, raw) in scopes.iter().copied().zip(raws) {
        if raw.is_undefined() {
            continue;
        }
        let source: Option<String> = if labelled {
            sources
                .method("label", vec![JsValue::from(scope.name())])
                .await
        } else {
            None
        };
        let issue = match parse_source(scope, raw, source.as_deref()) {
            Ok(value @ JsValue::Object(_)) => {
                let read = read_settings_layer(scope, &value, source.as_deref());
                layers.push(read.layer);
                issues.extend(read.issues);
                continue;
            }
            Ok(_) => broken_source(scope, None, source.as_deref()),
            Err(issue) => issue,
        };
        issues.push(issue);
        fatal = fatal || stops_startup(scope);
    }
    LoadedSettings {
        layers: if fatal { Vec::new() } else { layers },
        issues,
        fatal,
    }
}

fn parent_composition(composition: &ManagedComposition) -> ParentComposition {
    let behavior = match composition.behavior {
        ManagedBehavior::Merge => "merge",
        ManagedBehavior::FirstWins => "first-wins",
    };
    ParentComposition {
        layer: composition.layer.clone(),
        behavior: behavior.to_owned(),
        has_managed_settings: composition.has_managed_settings,
        parent_merge: composition.parent_merge,
        admins: composition.admins.clone(),
    }
}

fn has_keys(settings: &JsValue) -> bool {
    settings
        .as_object()
        .is_some_and(|members| !members.is_empty())
}

/// The managed tier from every managed source the machine holds, with the host's parent settings
/// beside it: the parent layer is dropped, or cut down to what the managed policy lets stand
/// (managed-settings.md, "Let an embedding host add policy"), and the HKCU registry applies only when no
/// restrictive parent settings were passed.
async fn load_with_managed(
    sources: Port,
    signal: &Signal,
    scopes: &[PolicyScope],
) -> LoadedSettings {
    let others_scopes: Vec<PolicyScope> = scopes
        .iter()
        .copied()
        .filter(|scope| *scope != PolicyScope::Managed)
        .collect();
    let others = load_scopes(sources, signal, &others_scopes).await;
    let parent = others
        .layers
        .iter()
        .find(|layer| layer.scope == PolicyScope::Parent);
    let parent_restrictive = parent.is_some_and(|layer| has_keys(&layer.settings));
    let reads: Option<ManagedReads> = sources.method("managed", vec![signal_arg(signal)]).await;
    let options = ComposeOptions {
        parent_restrictive: Some(parent_restrictive),
    };
    let composition = compose_managed_reads(&reads.unwrap_or_default(), options).await;
    let mut issues = composition.issues.clone();
    issues.extend(others.issues);
    if composition.fatal || others.fatal {
        return LoadedSettings {
            layers: Vec::new(),
            issues,
            fatal: true,
        };
    }
    let ruled = parent_composition(&composition);
    let kept: Vec<PolicyLayer> = others
        .layers
        .into_iter()
        .filter_map(|layer| {
            if layer.scope != PolicyScope::Parent {
                return Some(layer);
            }
            let settings = parent_settings_of(&ruled, &layer.settings).settings;
            // The parent's unreadable keys stay recorded: their fail-closed floors still apply.
            (!settings.is_undefined()).then_some(PolicyLayer { settings, ..layer })
        })
        .collect();
    let has_managed = !composition.used.is_empty() || has_keys(&composition.layer.settings);
    let layers = if has_managed {
        std::iter::once(composition.layer).chain(kept).collect()
    } else {
        kept
    };
    LoadedSettings {
        layers,
        issues,
        fatal: false,
    }
}

/// How the sources are read.
#[derive(Clone, Debug, Deserialize)]
pub struct LoadOptions {
    /// Ends the reads.
    pub signal: Signal,
    /// The scopes to read, every one by default.
    #[serde(default)]
    pub scopes: Option<Vec<PolicyScope>>,
}

/// Reads every scope the host has, in precedence order, into layers. A broken user, project, local or
/// `--settings` source is skipped; a broken managed one is `fatal`. A host that says what each managed
/// source holds (`SettingsSources.managed`) gets the managed tier composed by the kit.
pub async fn load_settings_layers(sources: Port, options: LoadOptions) -> LoadedSettings {
    let scopes = options.scopes.unwrap_or_else(|| POLICY_SCOPES.to_vec());
    let managed = sources.member::<Option<Callback>>("managed").await;
    if managed.is_none() || !scopes.contains(&PolicyScope::Managed) {
        return load_scopes(sources, &options.signal, &scopes).await;
    }
    load_with_managed(sources, &options.signal, &scopes).await
}

crate::registry::exports! {
    "settings/sources" {
        "loadSettingsLayers" => async ok |sources: Port, options: LoadOptions|
            load_settings_layers(sources, options).await;
    }
}
