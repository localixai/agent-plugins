//! Port of `src/format/marketplace/entry-merge.ts`, switched.
//!
//! How a plugin entry and the plugin's own `plugin.json` combine into the manifest Claude Code loads
//! (plugins/marketplace-reference.md, "How an entry combines with plugin.json", "Hooks in an entry",
//! "Display fields", "Strict mode"; plugins/manifest-reference.md, "How entry fields combine with
//! `plugin.json`", "Metadata precedence"). Pure over a parsed entry and a parsed `plugin.json`.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::format::defined::defined;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::entry_append::{
    Commands, Hooks, Paths, append_paths, combine_commands, merge_hooks,
};
use crate::format::marketplace::types::{MarketplaceEntry, PluginSource};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

/// What loading the plugin from this entry gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMerge {
    /// The entry name: the install id and the `enabledPlugins` key. `manifest.name` is the namespace.
    pub install_name: String,
    /// False when the plugin's default `skills/` directory is not scanned (root-source entry that lists skills).
    pub scan_default_skills: bool,
    /// The manifest the plugin loads with, or `undefined` when the plugin fails to load (the strict
    /// conflict). Its presence, not the issues' severity, says whether the plugin loads: the
    /// `not yet supported in a marketplace entry` error leaves the plugin loading without those hooks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<JsObject>,
    /// Hook events the entry declares and the manifest also holds: "the entry's matchers for an event
    /// replace the manifest's matchers for that same event". Hook files the manifest names (and
    /// `hooks/hooks.json`) are not read here, so the loader drops their matchers for these events.
    pub replaced_hook_events: Vec<String>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The six component fields the strict-mode table is about.
const STRICT_FIELDS: [&str; 6] = [
    "commands",
    "agents",
    "skills",
    "hooks",
    "outputStyles",
    "themes",
];

/// A member that is there: not missing, not `undefined`.
fn member<'a>(object: &'a JsObject, key: &str) -> Option<&'a JsValue> {
    object.get(key).filter(|value| !value.is_undefined())
}

fn declares_components(entry: &MarketplaceEntry) -> bool {
    STRICT_FIELDS
        .iter()
        .any(|field| member(&entry.components, field).is_some())
}

/// `true` for the source that is the marketplace root itself (`"."`).
fn is_root(source: &PluginSource) -> bool {
    matches!(source, PluginSource::Relative { path, .. } if path == ".")
}

const HOOKS_UNSUPPORTED: &str = "Hooks written as a file path or an array are not yet supported in a marketplace entry; write them as an inline object, or put them in the plugin's hooks/hooks.json or plugin.json";

/// The entry's `hooks` when it is the supported inline object, and the report when it is not.
fn inline_hooks(entry: &MarketplaceEntry) -> (Option<JsObject>, Vec<PluginIssue>) {
    match member(&entry.components, "hooks") {
        None => (None, Vec::new()),
        Some(JsValue::Object(hooks)) => (Some(hooks.clone()), Vec::new()),
        Some(_) => (None, vec![error("hooks", HOOKS_UNSUPPORTED)]),
    }
}

/// `{ ...target, ...source }`: each member of `source` set on `target`, in place when it is there.
fn spread(target: &mut JsObject, source: JsObject) {
    target.extend(source);
}

fn object_of<const N: usize>(members: [(&str, JsValue); N]) -> JsObject {
    members
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn text(value: Option<&String>) -> JsValue {
    value.map_or(JsValue::Undefined, |text| text.as_str().into())
}

/// The entry's fields that outrank plugin.json whatever `strict` says: `defaultEnabled` and the display fields.
fn outranking(entry: &MarketplaceEntry) -> JsObject {
    let mut fields = defined(object_of([
        (
            "defaultEnabled",
            entry
                .default_enabled
                .map_or(JsValue::Undefined, JsValue::Bool),
        ),
        ("displayName", text(entry.display_name.as_ref())),
        ("description", text(entry.description.as_ref())),
    ]));
    spread(&mut fields, entry.display.clone().into_iter().collect());
    fields
}

/// A list of dependencies as the manifest holds it, or nothing for an empty one.
fn dependencies_of(lists: &[Option<&[JsValue]>]) -> JsValue {
    let all: Vec<JsValue> = lists
        .iter()
        .flatten()
        .flat_map(|list| list.iter().cloned())
        .collect();
    if all.is_empty() {
        JsValue::Undefined
    } else {
        JsValue::Array(all)
    }
}

/// A member read as one of the entry-append shapes; a value of another shape is not one.
fn shape_of<T: DeserializeOwned>(value: Option<&JsValue>) -> Option<T> {
    value.and_then(|value| T::deserialize(value.clone()).ok())
}

fn value_of<T: Serialize>(value: Option<T>) -> JsValue {
    value
        .and_then(|value| to_js_value(&value).ok())
        .unwrap_or_default()
}

/// The entry as a manifest: no plugin.json, so every field the entry accepts applies.
fn entry_as_manifest(entry: &MarketplaceEntry, hooks: Option<JsObject>) -> JsObject {
    let mut manifest = object_of([("name", entry.name.as_str().into())]);
    spread(
        &mut manifest,
        defined(object_of([
            ("version", text(entry.version.as_ref())),
            (
                "dependencies",
                dependencies_of(&[entry.dependencies.as_deref()]),
            ),
        ])),
    );
    spread(&mut manifest, outranking(entry));
    let mut components: JsObject = entry.components.clone().into_iter().collect();
    components.insert(
        "hooks".to_owned(),
        hooks.map_or(JsValue::Undefined, JsValue::Object),
    );
    spread(&mut manifest, defined(components));
    manifest
}

/// The entry's `themes` added to the key of plugin.json that is read: `experimental.themes` when it
/// is set (it wins over the top-level key, spec J6.10), else the top-level `themes` (spec §6.2.2
/// item 3: with strict the entry's themes add to the manifest's, never into a key that is shadowed).
fn with_themes(manifest: &JsObject, themes: Option<&Paths>) -> JsObject {
    if let Some(JsValue::Object(experimental)) = member(manifest, "experimental")
        && let Some(own) = member(experimental, "themes")
    {
        let mut experimental = experimental.clone();
        let joined = append_paths(shape_of::<Paths>(Some(own)).as_ref(), themes);
        experimental.insert("themes".to_owned(), value_of(joined));
        return object_of([("experimental", JsValue::Object(experimental))]);
    }
    let own = shape_of::<Paths>(member(manifest, "themes"));
    defined(object_of([(
        "themes",
        value_of(append_paths(own.as_ref(), themes)),
    )]))
}

/// `manifest.version ?? entry.version`.
fn version_of(manifest: &JsObject, entry: &MarketplaceEntry) -> JsValue {
    match manifest.get("version") {
        Some(version) if !version.is_nullish() => version.clone(),
        _ => text(entry.version.as_ref()),
    }
}

fn manifest_dependencies(manifest: &JsObject) -> Option<&[JsValue]> {
    manifest
        .get("dependencies")
        .and_then(JsValue::as_array)
        .map(Vec::as_slice)
}

/// plugin.json is the manifest; the entry completes it (strict) and outranks it for the display fields.
fn complete_manifest(
    entry: &MarketplaceEntry,
    manifest: &JsObject,
    hooks: Option<&JsObject>,
) -> (JsObject, Vec<PluginIssue>) {
    let components = &entry.components;
    let paths = |key: &str| shape_of::<Paths>(member(components, key));
    let dependencies = dependencies_of(&[
        manifest_dependencies(manifest),
        entry.dependencies.as_deref(),
    ]);
    let mut issues = Vec::new();
    let mut merged = manifest.clone();
    spread(&mut merged, outranking(entry));
    if !entry.strict {
        let completed = object_of([
            ("version", version_of(manifest, entry)),
            ("dependencies", dependencies),
        ]);
        spread(&mut merged, defined(completed));
        return (merged, issues);
    }
    let commands = combine_commands(
        shape_of::<Commands>(member(manifest, "commands")).as_ref(),
        shape_of::<Commands>(member(components, "commands")).as_ref(),
    );
    issues.extend(commands.issues);
    let own = |key: &str| shape_of::<Paths>(member(manifest, key));
    let joined = |key: &str| value_of(append_paths(own(key).as_ref(), paths(key).as_ref()));
    let merged_hooks = match hooks {
        None => manifest.get("hooks").cloned().unwrap_or_default(),
        Some(hooks) => value_of(Some(merge_hooks(
            shape_of::<Hooks>(member(manifest, "hooks")).as_ref(),
            hooks,
        ))),
    };
    let completed = object_of([
        ("version", version_of(manifest, entry)),
        ("dependencies", dependencies),
        ("commands", value_of(commands.value)),
        ("agents", joined("agents")),
        ("skills", joined("skills")),
        ("outputStyles", joined("outputStyles")),
        ("hooks", merged_hooks),
    ]);
    spread(&mut merged, defined(completed));
    spread(&mut merged, with_themes(manifest, paths("themes").as_ref()));
    (merged, issues)
}

/// The manifest a plugin loads with, given its marketplace entry and its own `plugin.json`
/// (`None` when it has none).
#[must_use]
pub fn merge_entry_with_manifest(
    entry: &MarketplaceEntry,
    manifest: Option<&JsObject>,
) -> EntryMerge {
    let (hooks, hook_issues) = inline_hooks(entry);
    let install_name = entry.name.clone();
    let scan_default_skills =
        !(member(&entry.components, "skills").is_some() && is_root(&entry.source));
    let Some(manifest) = manifest else {
        return EntryMerge {
            install_name,
            scan_default_skills,
            manifest: Some(entry_as_manifest(entry, hooks)),
            replaced_hook_events: Vec::new(),
            issues: hook_issues,
        };
    };
    if !entry.strict && declares_components(entry) {
        let message = format!(
            "Plugin {} has conflicting manifests: both plugin.json and marketplace entry specify components",
            entry.name
        );
        return EntryMerge {
            install_name,
            scan_default_skills,
            manifest: None,
            replaced_hook_events: Vec::new(),
            issues: vec![error("plugin", &message)],
        };
    }
    let (merged, issues) = complete_manifest(entry, manifest, hooks.as_ref());
    let replaced_hook_events = match &hooks {
        Some(hooks) if entry.strict => hooks.keys().cloned().collect(),
        _ => Vec::new(),
    };
    EntryMerge {
        install_name,
        scan_default_skills,
        manifest: Some(merged),
        replaced_hook_events,
        issues: hook_issues.into_iter().chain(issues).collect(),
    }
}

/// The name users see: the display name the merge settled on (the entry's, then plugin.json's), "when
/// neither ... sets one, users see the plugin's `name`" (marketplace-reference.md, `displayName`),
/// read as the name of the merged manifest (spec/judgments.md), the entry name when it fails to load.
#[must_use]
pub fn effective_display_name(merge: &EntryMerge) -> String {
    let read = |key: &str| {
        merge
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.get(key))
            .filter(|value| !value.is_nullish())
    };
    read("displayName")
        .or_else(|| read("name"))
        .map_or_else(|| merge.install_name.clone(), crate::js::string_of)
}

crate::registry::exports! {
    "format/marketplace/entry-merge" {
        "mergeEntryWithManifest" => ok |entry: MarketplaceEntry, manifest: Option<JsObject>|
            merge_entry_with_manifest(&entry, manifest.as_ref());
        "effectiveDisplayName" => ok |merge: EntryMerge| effective_display_name(&merge);
    }
}
