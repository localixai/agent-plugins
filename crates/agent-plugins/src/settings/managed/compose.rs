//! Port of `src/settings/managed/compose.ts`, switched.
//!
//! Which managed sources apply and the managed tier they make (managed-settings.md, "How Claude Code
//! combines managed sources", "Keys read from every admin source", "Compose every managed source";
//! settings-reference.md, `managedSourcesBehavior`). Pure over the documents the host read.

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue, in_property_order};
use crate::settings::layers::{PolicyLayer, PolicyScope};
use crate::settings::managed::admin_source::{
    AdminDocument, CONTROL_KEYS, ManagedSource, source_traits, unreadable_verdict, verdict_issue,
};
use crate::settings::managed::combine::combine_sources;
use crate::settings::managed::cross_source::{
    CrossSourceInput, InForceNote, not_in_force, with_cross_source_keys,
};
use crate::settings::managed::login_keys::{login_keys_of, managed_layer, without_login_keys};
use crate::settings::managed::policy_helper::{HelperKey, PolicyHelper, read_policy_helper};

/// `managedSourcesBehavior`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagedBehavior {
    /// The highest source that delivers a policy is the policy.
    FirstWins,
    /// Every source that delivers one, combined.
    Merge,
}

/// The behaviors.
pub const MANAGED_BEHAVIORS: [ManagedBehavior; 2] =
    [ManagedBehavior::FirstWins, ManagedBehavior::Merge];

/// How the composition is asked for.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeOptions {
    /// The host passed parent settings that carry a restrictive key. HKCU "applies only when no admin
    /// document is present above it and no host-supplied parent settings supply a restrictive key".
    #[serde(default)]
    pub parent_restrictive: Option<bool>,
}

/// What the policy helper does on this machine.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperPlan {
    /// The helper the highest source that carries a policy key names; absent when none, or its value is invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helper: Option<PolicyHelper>,
    /// The helper runs at launch: its source is an MDM policy or a managed file.
    pub runs_at_launch: bool,
    /// Server-managed settings are the policy, so the helper waits for a fetch that reports them removed.
    pub shadowed_by_remote: bool,
}

/// The managed tier and what made it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedComposition {
    /// The managed tier: the layer to give the policy view.
    pub layer: PolicyLayer,
    /// The behavior read.
    pub behavior: ManagedBehavior,
    /// The sources applied, highest first.
    pub used: Vec<ManagedSource>,
    /// Sources found and not applied.
    pub skipped: Vec<ManagedSource>,
    /// `/status` `Setting sources`: `Enterprise managed settings (file)`; absent when no source delivers a policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<String>,
    /// `/status` `Skipped sources`; absent when none was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_sources: Option<String>,
    /// An administrator deploys managed settings here (`PolicyContext.hasManagedSettings`).
    pub has_managed_settings: bool,
    /// `parentSettingsBehavior` is `"merge"` in the highest-priority admin source (`PolicyContext.parentMerge`).
    pub parent_merge: bool,
    /// The repaired settings of every admin source that delivers a policy key, highest first.
    pub admins: Vec<JsValue>,
    /// Some admin source sets `forceRemoteSettingsRefresh` to `true`. "The startup check runs before the
    /// helper and reads any admin source", so a helper's output does not change it.
    pub force_remote_refresh: bool,
    /// What the policy helper does.
    pub helper: HelperPlan,
    /// A lock or allowlist an admin source sets and another overrides.
    pub not_in_force: Vec<InForceNote>,
    /// What reading and composing reported.
    pub issues: Vec<PluginIssue>,
    /// A present admin document cannot be parsed, or one cannot be read and no source supplies a policy: Claude Code exits.
    pub fatal: bool,
}

fn rank_of(document: &AdminDocument) -> u8 {
    source_traits(document.source).rank
}
fn is_admin(document: &AdminDocument) -> bool {
    source_traits(document.source).admin
}
fn is_remote(document: &AdminDocument) -> bool {
    !source_traits(document.source).on_machine
}

fn member(settings: &JsValue, key: &str) -> JsValue {
    settings.get(key).cloned().unwrap_or_default()
}

fn without(settings: &JsValue, drop: impl Fn(&str) -> bool) -> JsValue {
    let kept: JsObject = settings
        .as_object()
        .map(|members| {
            members
                .iter()
                .filter(|(name, _)| !drop(name))
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    JsValue::Object(in_property_order(kept))
}

/// The documents, highest source first; two of one rank keep the order they came in.
fn by_rank(documents: Vec<AdminDocument>) -> Vec<AdminDocument> {
    let mut ranked = documents;
    ranked.sort_by_key(|document| std::cmp::Reverse(rank_of(document)));
    ranked
}

/// `managedSourcesBehavior` is read "from the highest-priority source that carries either this key
/// or a policy key"; a lower source cannot opt itself in. Anything but `"merge"` is the default.
fn behavior_of(admins: &[&AdminDocument]) -> ManagedBehavior {
    let carrier = admins.iter().find(|document| {
        document.delivers
            || document
                .raw
                .get("managedSourcesBehavior")
                .is_some_and(|value| !value.is_nullish())
    });
    let merge = carrier.and_then(|document| document.raw.get("managedSourcesBehavior"))
        == Some(&JsValue::from("merge"));
    if merge {
        ManagedBehavior::Merge
    } else {
        ManagedBehavior::FirstWins
    }
}

/// `modelOverrides` pairs with `availableModels`: a higher source that sets `availableModels` without it hides every source's.
fn paired_model_overrides(used: &[JsValue], settings: JsValue) -> JsValue {
    let set = |key| {
        used.iter()
            .position(|source| !member(source, key).is_undefined())
    };
    match (set("modelOverrides"), set("availableModels")) {
        (Some(overrides), Some(available)) if available < overrides => {
            without(&settings, |name| name == "modelOverrides")
        }
        _ => settings,
    }
}

fn without_control_keys(settings: &JsValue) -> JsValue {
    without(settings, |name| CONTROL_KEYS.contains(&name))
}

fn policy_of(delivering: &[&AdminDocument], behavior: ManagedBehavior) -> JsValue {
    let merging = behavior == ManagedBehavior::Merge;
    let used = if merging {
        delivering
    } else {
        delivering.get(..1).unwrap_or_default()
    };
    let used_settings: Vec<JsValue> = used
        .iter()
        .map(|document| document.settings.clone())
        .collect();
    let settings = if merging {
        combine_sources(&used_settings)
    } else {
        used_settings
            .first()
            .cloned()
            .unwrap_or_else(|| JsValue::Object(JsObject::new()))
    };
    let crossed = with_cross_source_keys(
        &CrossSourceInput {
            admins: delivering
                .iter()
                .map(|document| document.settings.clone())
                .collect(),
            on_machine: delivering
                .iter()
                .map(|document| !is_remote(document))
                .collect(),
            merging,
        },
        settings,
    );
    let settings_of = |found: Option<&&AdminDocument>| {
        found
            .map(|document| document.settings.clone())
            .unwrap_or_default()
    };
    let machine = settings_of(delivering.iter().find(|document| !is_remote(document)));
    let remote = settings_of(delivering.iter().find(|document| is_remote(document)));
    let login = login_keys_of(&machine, &remote);
    let paired = paired_model_overrides(&used_settings, crossed);
    let JsValue::Object(mut joined) = without_login_keys(&paired) else {
        return JsValue::Object(JsObject::new());
    };
    joined.extend(login.as_object().cloned().unwrap_or_default());
    without_control_keys(&JsValue::Object(joined))
}

fn label_of(document: &AdminDocument) -> String {
    document
        .shape
        .clone()
        .unwrap_or_else(|| source_traits(document.source).status_label.to_owned())
}

/// `/status` when no source applies and "an embedding host supplied restrictive settings".
const PARENT_PROCESS_LINE: &str = "Enterprise managed settings (parent process)";

/// `Enterprise managed settings (remote + file, merged)`.
fn setting_sources_line(used: &[&AdminDocument], merging: bool) -> Option<String> {
    if used.is_empty() {
        return None;
    }
    let merged = if merging && used.len() > 1 {
        ", merged"
    } else {
        ""
    };
    let labels: Vec<String> = used.iter().map(|document| label_of(document)).collect();
    Some(format!(
        "Enterprise managed settings ({}{merged})",
        labels.join(" + ")
    ))
}

fn helper_plan_of(delivering: &[&AdminDocument]) -> (HelperPlan, Vec<PluginIssue>) {
    let top = delivering.first();
    let machine_top = delivering.iter().find(|document| !is_remote(document));
    let value = machine_top
        .map(|document| member(&document.settings, "policyHelper"))
        .unwrap_or_default();
    let (Some(top), false) = (top, value.is_undefined()) else {
        return (HelperPlan::default(), Vec::new());
    };
    match read_policy_helper(&value) {
        HelperKey::Issue { issue } => (HelperPlan::default(), vec![issue]),
        HelperKey::Helper { helper } => {
            let shadowed_by_remote = is_remote(top);
            let plan = HelperPlan {
                helper: Some(helper),
                runs_at_launch: !shadowed_by_remote,
                shadowed_by_remote,
            };
            (plan, Vec::new())
        }
    }
}

/// An unreadable admin source with no policy to fall back on: the issues, and whether it exits.
fn unreadable_issues(admins: &[&AdminDocument]) -> (Vec<PluginIssue>, bool) {
    let verdicts: Vec<_> = admins
        .iter()
        .filter_map(|document| {
            let (cause, at) = (document.unreadable?, document.unreadable_at.as_ref()?);
            Some(unreadable_verdict(cause, &at.label, &at.reason))
        })
        .collect();
    let issues = verdicts
        .iter()
        .map(|verdict| verdict_issue(verdict, "managed:"))
        .collect();
    (issues, verdicts.iter().any(|verdict| verdict.exits))
}

/// Which documents apply: the admin sources that deliver a policy, or the user-writable fallback.
struct Selection<'a> {
    admins: Vec<&'a AdminDocument>,
    delivering: Vec<&'a AdminDocument>,
    behavior: ManagedBehavior,
    /// Some admin document is present, so the HKCU registry is never applied beneath it.
    any_admin: bool,
    /// The HKCU documents that deliver a policy key, applied or not.
    hkcu: Vec<&'a AdminDocument>,
    used: Vec<&'a AdminDocument>,
}

fn select<'a>(ranked: &'a [AdminDocument], options: ComposeOptions) -> Selection<'a> {
    let admins: Vec<&AdminDocument> = ranked
        .iter()
        .filter(|document| is_admin(document) && !document.broken)
        .collect();
    let delivering: Vec<&AdminDocument> = admins
        .iter()
        .copied()
        .filter(|document| document.delivers)
        .collect();
    let behavior = behavior_of(&admins);
    let any_admin = ranked
        .iter()
        .any(|document| is_admin(document) && document.present);
    let hkcu: Vec<&AdminDocument> = ranked
        .iter()
        .filter(|document| !is_admin(document) && document.delivers)
        .collect();
    let fallback = if !any_admin && options.parent_restrictive != Some(true) {
        hkcu.clone()
    } else {
        Vec::new()
    };
    let applied = if behavior == ManagedBehavior::Merge {
        delivering.clone()
    } else {
        delivering.iter().copied().take(1).collect()
    };
    let used = if delivering.is_empty() {
        fallback
    } else {
        applied
    };
    Selection {
        admins,
        delivering,
        behavior,
        any_admin,
        hkcu,
        used,
    }
}

/// The settings of the selection; an invalid `policyHelper` is a dropped entry, so the session starts on the rest.
fn settings_of(selection: &Selection<'_>, helper_issues: &[PluginIssue]) -> JsValue {
    let chosen = if selection.delivering.is_empty() {
        without_control_keys(
            &selection
                .used
                .first()
                .map(|document| document.settings.clone())
                .unwrap_or_default(),
        )
    } else {
        policy_of(&selection.delivering, selection.behavior)
    };
    if helper_issues.is_empty() {
        chosen
    } else {
        without(&chosen, |name| name == "policyHelper")
    }
}

/// The managed tier of the documents the host read.
#[must_use]
pub fn compose_managed(
    documents: Vec<AdminDocument>,
    options: ComposeOptions,
) -> ManagedComposition {
    let ranked = by_rank(documents);
    let selection = select(&ranked, options);
    let (helper, helper_issues) = helper_plan_of(&selection.delivering);
    let settings = settings_of(&selection, &helper_issues);
    let (unreadable, exits) = if selection.delivering.is_empty() {
        unreadable_issues(&selection.admins)
    } else {
        (Vec::new(), false)
    };
    let is_used = |document: &&AdminDocument| {
        selection
            .used
            .iter()
            .any(|used| std::ptr::eq(*used, *document))
    };
    let skipped: Vec<&AdminDocument> = selection
        .delivering
        .iter()
        .chain(&selection.hkcu)
        .copied()
        .filter(|document| !is_used(document))
        .collect();
    let admin_settings: Vec<JsValue> = selection
        .delivering
        .iter()
        .map(|document| document.settings.clone())
        .collect();
    let merging = selection.behavior == ManagedBehavior::Merge;
    let machine = selection
        .delivering
        .iter()
        .find(|document| !is_remote(document));
    let first = admin_settings.first().cloned().unwrap_or_default();
    let mut issues: Vec<PluginIssue> = ranked
        .iter()
        .flat_map(|document| document.issues.iter().cloned())
        .collect();
    issues.extend(helper_issues);
    issues.extend(unreadable);
    ManagedComposition {
        layer: managed_layer(
            settings.clone(),
            machine.map(|document| document.invalid.as_slice()),
        ),
        behavior: selection.behavior,
        used: selection
            .used
            .iter()
            .map(|document| document.source)
            .collect(),
        skipped: skipped.iter().map(|document| document.source).collect(),
        setting_sources: setting_sources_line(
            &selection.used,
            merging && !selection.delivering.is_empty(),
        )
        .or_else(|| {
            (options.parent_restrictive == Some(true)).then(|| PARENT_PROCESS_LINE.to_owned())
        }),
        skipped_sources: (!skipped.is_empty()).then(|| {
            skipped
                .iter()
                .map(|document| label_of(document))
                .collect::<Vec<_>>()
                .join(", ")
        }),
        has_managed_settings: selection.any_admin,
        parent_merge: member(&first, "parentSettingsBehavior") == JsValue::from("merge"),
        force_remote_refresh: admin_settings
            .iter()
            .any(|doc| member(doc, "forceRemoteSettingsRefresh") == JsValue::Bool(true)),
        helper,
        not_in_force: not_in_force(&admin_settings, &settings),
        admins: admin_settings,
        issues,
        fatal: ranked
            .iter()
            .any(|document| document.broken && is_admin(document))
            || exits,
    }
}

/// The managed tier after a policy helper emitted `managedSettings` (settings-reference.md, "Write
/// the helper output"): "that object becomes the only managed settings source for the run", the
/// cross-source keys read from it alone, and parent settings are never merged.
#[must_use]
pub fn with_helper_settings(composition: JsObject, helper_output: &JsValue) -> JsObject {
    // `forceRemoteSettingsRefresh` "has its own startup rule" that runs before the helper and reads
    // the admin sources, so the helper's copy takes no part (`composition.forceRemoteRefresh`).
    let emitted = without(helper_output, |name| name == "forceRemoteSettingsRefresh");
    let input = CrossSourceInput {
        admins: vec![emitted.clone()],
        on_machine: vec![true],
        merging: false,
    };
    let settings = with_cross_source_keys(&input, emitted.clone());
    let layer = PolicyLayer {
        scope: PolicyScope::Managed,
        settings,
        invalid: None,
    };
    let mut next = composition;
    let set = |next: &mut JsObject, key: &str, value: JsValue| {
        next.insert(key.to_owned(), value);
    };
    set(
        &mut next,
        "layer",
        crate::js::value::to_js_value(&layer).unwrap_or_default(),
    );
    set(&mut next, "used", JsValue::Array(Vec::new()));
    set(&mut next, "skipped", JsValue::Array(Vec::new()));
    set(
        &mut next,
        "settingSources",
        JsValue::from("Enterprise managed settings (helper)"),
    );
    next.shift_remove("skippedSources");
    set(&mut next, "hasManagedSettings", JsValue::Bool(true));
    set(&mut next, "parentMerge", JsValue::Bool(false));
    set(&mut next, "admins", JsValue::Array(vec![emitted]));
    next
}

crate::registry::exports! {
    "settings/managed/compose" {
        "MANAGED_BEHAVIORS" => ok || MANAGED_BEHAVIORS;
        "composeManaged" => ok |documents: Vec<AdminDocument>, options: Option<ComposeOptions>|
            compose_managed(documents, options.unwrap_or_default());
        "withHelperSettings" => ok |composition: JsObject, helper_output: JsValue|
            with_helper_settings(composition, &helper_output);
    }
}
