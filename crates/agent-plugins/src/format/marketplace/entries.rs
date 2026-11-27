//! Port of `src/format/marketplace/entries.ts`, switched.
//!
//! Reading one plugin entry of `marketplace.json` (plugins/marketplace-reference.md, "Plugin
//! entries" and "Validation messages"): an entry with an error is dropped alone, a warning keeps it.
//! `source` is handed to the plugin source parser (`parsePluginSource`), which this reader takes as a
//! parameter so the two are written and tested apart.
//!
//! The parser is a `SourceParser`: a Rust function answers at once, the host's (a `Callback`) as a
//! need, so the reader is `async` and its exports stay synchronous for the host. `EntrySlot` and
//! `EntryReading` are `src/format/marketplace/marketplace.ts`'s types, declared here, where the
//! core first needs them.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::entry_checks::{HeaderFields, NameReports, header_issues, name_reports, version_issues};
use super::entry_schema::{ENTRY_KEYS, EntryFields, is_entry_key, read_entry_fields};
use super::marketplace_schema::{mismatch, type_name, unknown_field};
use super::relevance::read_relevance;
use super::schema_issues::zod_issues;
use super::types::{MarketplaceContext, MarketplaceEntry, PluginSource, ReadMode, Relevance};
use crate::format::defined::defined;
use crate::format::issues::{PluginIssue, blocks_loading, error, warning};
use crate::format::manifest_schema::MANIFEST_SCHEMAS;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, property_order};
use crate::ports::BoxFuture;
use crate::protocol::is_record::is_record;
use crate::registry::{Callback, Function};

/// What reading one entry gave: the entry when it loads, and what was found either way.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryReading {
    /// The entry, when it loads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<MarketplaceEntry>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
    /// The entry's fields pass the schema; absent is taken as true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_sound: Option<bool>,
}

/// Where an entry stands in its marketplace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySlot {
    /// Its index in `plugins`.
    pub index: u32,
    /// `metadata.pluginRoot`: the directory bare relative sources resolve under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_root: Option<String>,
    /// What the host knows about the marketplace.
    pub context: MarketplaceContext,
    /// `load` unless the marketplace is being validated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ReadMode>,
}

/// What a plugin source parser returns: the source, or `undefined` with the issues that say why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceReading {
    /// The source, when it parsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PluginSource>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// `SourceParser`: `(raw, slot) => SourceReading`.
pub trait SourceParser: Send + Sync {
    /// The source `raw` describes at `slot`.
    fn parse<'a>(&'a self, raw: &'a JsValue, slot: &'a EntrySlot) -> BoxFuture<'a, SourceReading>;
}

impl<F: Fn(&JsValue, &EntrySlot) -> SourceReading + Send + Sync> SourceParser for F {
    fn parse<'a>(&'a self, raw: &'a JsValue, slot: &'a EntrySlot) -> BoxFuture<'a, SourceReading> {
        Box::pin(std::future::ready(self(raw, slot)))
    }
}

impl SourceParser for Callback {
    fn parse<'a>(&'a self, raw: &'a JsValue, slot: &'a EntrySlot) -> BoxFuture<'a, SourceReading> {
        let slot = to_js_value(slot).unwrap_or_default();
        Box::pin(self.call(vec![raw.clone(), slot]))
    }
}

fn dropped(issues: Vec<PluginIssue>) -> EntryReading {
    EntryReading {
        entry: None,
        issues,
        schema_sound: Some(false),
    }
}

/// `name`: required, and an install id (letters, digits, `.`, `_`, `-`). A wrong type is the field reader's report.
fn read_name(raw: &JsObject, base: &str, slot: &EntrySlot) -> NameReports {
    let path = format!("{base}.name");
    let name = raw.get("name").unwrap_or(&JsValue::Undefined);
    if name.is_undefined() {
        return NameReports {
            shape: vec![error(
                &path,
                "Invalid input: expected string, received undefined",
            )],
            reserved: Vec::new(),
        };
    }
    let Some(name) = name.as_str() else {
        return NameReports {
            shape: Vec::new(),
            reserved: Vec::new(),
        };
    };
    name_reports(
        name,
        &path,
        slot.mode.unwrap_or(ReadMode::Load),
        slot.context.marketplace_name.as_deref(),
    )
}

/// `source`: required, a string or an object; what it means is the parser's.
async fn read_source(
    raw: &JsValue,
    slot: &EntrySlot,
    base: &str,
    parse: &dyn SourceParser,
) -> SourceReading {
    if raw.is_string() || is_record(raw) {
        return parse.parse(raw, slot).await;
    }
    SourceReading {
        source: None,
        issues: vec![error(&format!("{base}.source"), "Invalid input")],
    }
}

/// A free-form object field: anything else is a warning and the value is dropped, as Claude Code does.
fn object_or_warning(
    value: &JsValue,
    path: &str,
    expected: &str,
) -> (Option<JsObject>, Vec<PluginIssue>) {
    if value.is_undefined() {
        return (None, Vec::new());
    }
    if let Some(members) = value.as_object().filter(|_| is_record(value)) {
        return (Some(members.clone()), Vec::new());
    }
    let key = path.rsplit_once('.').map_or(path, |(_, key)| key);
    let message = format!(
        "'{key}' must be {expected}; got {}. It will be ignored at load time.",
        type_name(value)
    );
    (None, vec![warning(path, &message)])
}

/// `experimental`: an object of component declarations, `themes`, `monitors` and `evals`.
fn read_experimental(raw: &JsValue, path: &str) -> (Option<JsValue>, Vec<PluginIssue>) {
    let (object, issues) =
        object_or_warning(raw, path, "an object containing component declarations");
    let Some(object) = object else {
        return (None, issues);
    };
    let Some(schema) = MANIFEST_SCHEMAS.get("experimental") else {
        return (None, Vec::new());
    };
    match schema.safe_parse(&JsValue::Object(object)) {
        Ok(parsed) => (Some(parsed), Vec::new()),
        Err(failure) => (None, zod_issues(&failure, path)),
    }
}

/// The fields an entry reads with a reader of their own.
struct Apart {
    metadata: Option<JsObject>,
    experimental: Option<JsValue>,
    relevance: Option<Relevance>,
    issues: Vec<PluginIssue>,
}

fn read_apart(raw: &JsObject, base: &str) -> Apart {
    let member = |key: &str| raw.get(key).unwrap_or(&JsValue::Undefined);
    let metadata = object_or_warning(
        member("metadata"),
        &format!("{base}.metadata"),
        "a free-form object",
    );
    let experimental = read_experimental(member("experimental"), &format!("{base}.experimental"));
    let relevance = read_relevance(member("relevance"), &format!("{base}.relevance"));
    let mut issues = metadata.1;
    issues.extend(experimental.1);
    issues.extend(relevance.issues);
    Apart {
        metadata: metadata.0,
        experimental: experimental.0,
        relevance: relevance.relevance,
        issues,
    }
}

/// A parsed field as the entry holds it.
fn field<T: DeserializeOwned>(fields: &EntryFields, key: &str) -> Option<T> {
    fields
        .get(key)
        .filter(|value| !value.is_undefined())
        .and_then(|value| T::deserialize(value.clone()).ok())
}

/// `defined({ key: fields.key, … })` over the keys named, in their order.
fn picked(fields: &EntryFields, keys: &[&str]) -> JsObject {
    defined(
        keys.iter()
            .map(|key| {
                let value = fields.get(*key).cloned().unwrap_or(JsValue::Undefined);
                ((*key).to_owned(), value)
            })
            .collect(),
    )
}

const COMPONENT_KEYS: [&str; 15] = [
    "commands",
    "agents",
    "skills",
    "hooks",
    "outputStyles",
    "themes",
    "mcpServers",
    "lspServers",
    "userConfig",
    "channels",
    "settings",
    "workflows",
    "monitors",
    "experimental",
    "types",
];
const DISPLAY_KEYS: [&str; 5] = ["author", "homepage", "repository", "license", "keywords"];

fn build(
    Parts {
        fields,
        name,
        source,
    }: Parts,
    apart: Apart,
) -> MarketplaceEntry {
    // `experimental` is the one component read apart: laid over the fields, in its place.
    let mut components = fields.clone();
    components.insert(
        "experimental".to_owned(),
        apart.experimental.unwrap_or(JsValue::Undefined),
    );
    MarketplaceEntry {
        name,
        source,
        strict: field(&fields, "strict").unwrap_or(true),
        description: field(&fields, "description"),
        version: field(&fields, "version"),
        category: field(&fields, "category"),
        tags: field(&fields, "tags"),
        relevance: apart.relevance,
        dependencies: field(&fields, "dependencies"),
        default_enabled: field(&fields, "defaultEnabled"),
        display_name: field(&fields, "displayName"),
        metadata: apart.metadata,
        headers: field(&fields, "headers"),
        headers_helper: field(&fields, "headersHelper"),
        components: picked(&components, &COMPONENT_KEYS),
        display: picked(&fields, &DISPLAY_KEYS),
    }
}

/// What a readable entry is made of: its fields, its name and its source.
struct Parts {
    fields: EntryFields,
    name: String,
    source: PluginSource,
}

/// The reports that need the parsed fields and the parsed source together.
fn cross_issues(parts: &Parts, slot: &EntrySlot, base: &str, sound: bool) -> Vec<PluginIssue> {
    let header = HeaderFields {
        name: parts.name.clone(),
        strict: field(&parts.fields, "strict").unwrap_or(true),
        headers: parts.fields.get("headers").cloned().unwrap_or_default(),
        headers_helper: parts
            .fields
            .get("headersHelper")
            .cloned()
            .unwrap_or_default(),
    };
    let manifest = slot
        .context
        .plugin_manifests
        .as_ref()
        .and_then(|manifests| {
            manifests
                .entries
                .iter()
                .find(|(index, _)| *index == slot.index)
        })
        .map_or(JsValue::Undefined, |(_, manifest)| manifest.clone());
    let mut issues = header_issues(&header, &parts.source, base);
    // An entry the schema already rejects is not compared with its manifest (oracle 2.1.319).
    if sound {
        let version: Option<String> = field(&parts.fields, "version");
        issues.extend(version_issues(
            version.as_deref(),
            &parts.source,
            &manifest,
            base,
        ));
    }
    issues
}

fn unknown_keys(raw: &JsObject, base: &str) -> Vec<PluginIssue> {
    let known: Vec<&str> = ENTRY_KEYS.iter().map(String::as_str).collect();
    property_order(raw)
        .into_iter()
        .filter(|(key, _)| !is_entry_key(key))
        .map(|(key, _)| unknown_field(key, &format!("{base}.{key}"), &known))
        .collect()
}

/// Reads one raw `plugins[i]`; never throws.
pub async fn read_entry(
    raw: &JsValue,
    slot: &EntrySlot,
    parse_source: &dyn SourceParser,
) -> EntryReading {
    let base = format!("plugins[{}]", slot.index);
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return dropped(vec![mismatch(&base, "object", raw)]);
    };
    let read = read_entry_fields(members, &base);
    let source_raw = members.get("source").unwrap_or(&JsValue::Undefined);
    let source = read_source(source_raw, slot, &base, parse_source).await;
    let apart = read_apart(members, &base);
    let named = read_name(members, &base, slot);
    let mut gating: Vec<PluginIssue> = named.shape.clone();
    gating.extend(read.issues.iter().cloned());
    gating.extend(source.issues.iter().cloned());
    gating.extend(apart.issues.iter().cloned());
    let mut issues: Vec<PluginIssue> = named.shape;
    issues.extend(named.reserved);
    issues.extend(read.issues);
    issues.extend(source.issues);
    issues.extend(apart.issues.iter().cloned());
    issues.extend(unknown_keys(members, &base));
    // The reserved-name ruling does not stop the comparison with the entry's manifest (oracle 2.1.319).
    let sound = !blocks_loading(&gating);
    let name: Option<String> = read
        .fields
        .as_ref()
        .and_then(|fields| field(fields, "name"));
    let (Some(fields), Some(name), Some(source)) = (read.fields, name, source.source) else {
        return dropped(issues);
    };
    let parts = Parts {
        fields,
        name,
        source,
    };
    issues.extend(cross_issues(&parts, slot, &base, sound));
    if blocks_loading(&issues) {
        return EntryReading {
            entry: None,
            issues,
            schema_sound: Some(sound),
        };
    }
    EntryReading {
        entry: Some(build(parts, apart)),
        issues,
        schema_sound: None,
    }
}

/// The entry reader of a marketplace, over the plugin source parser (pass it to `readMarketplace`).
struct EntryReader(Callback);

crate::registry::methods! {
    EntryReader {
        "call" => sync async ok |this, raw: JsValue, slot: EntrySlot| read_entry(&raw, &slot, &this.0).await;
    }
}

/// `createEntryReader(parseSource)`: the reader of one entry over the host's parser.
#[must_use]
pub fn create_entry_reader(parse_source: Callback) -> Function {
    Function::new(EntryReader(parse_source))
}

crate::registry::exports! {
    "format/marketplace/entries" {
        "readEntry" => async ok |raw: JsValue, slot: EntrySlot, parse_source: Callback|
            read_entry(&raw, &slot, &parse_source).await;
        "createEntryReader" => ok |parse_source: Callback| create_entry_reader(parse_source);
    }
}
