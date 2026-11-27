//! Port of `src/format/marketplace/marketplace.ts`, switched.
//!
//! Reading `marketplace.json` as Claude Code loads it (plugins/marketplace-reference.md, "Top-level
//! fields" and "Validation messages"; plugins/create-marketplace.md, "Problems that validation
//! reports"): the top-level fields and the marketplace's name are checked here; each entry is handed
//! to an `EntryReader` on its own, because "each entry is validated on its own, so one invalid
//! entry doesn't fail the marketplace".

use super::entries::{EntryReading, EntrySlot, read_entry};
use super::entry_checks::{IndexedEntry, without_duplicates};
use super::ids::{IdPart, desktop_name_issues, plugin_id_issues};
use super::marketplace_names::{NameOrigin, marketplace_name_issues, validation_name_issues};
use super::marketplace_schema::{TopLevel, mismatch, read_top_level};
use super::plugin_source::parse_plugin_source;
use super::renames::validate_renames;
use super::types::{
    Marketplace, MarketplaceContext, MarketplaceEntry, MarketplaceResult, ReadMode,
};
use crate::format::issues::{PluginIssue, blocks_loading, warning};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::protocol::is_record::is_record;
use crate::registry::Callback;

/// Reads one raw `plugins[i]` into an entry: the host's (tests of the top level pass a stand-in),
/// or, when none is given, the default one, each entry through the entry reader over the plugin
/// source parser (`defaultEntryReader`).
#[derive(Clone, Copy, Debug)]
pub enum EntryReader {
    /// `createEntryReader(parsePluginSource)`.
    Default,
    /// The host's `(raw, slot) => EntryReading`.
    Host(Callback),
}

impl EntryReader {
    async fn read(self, raw: &JsValue, slot: &EntrySlot) -> EntryReading {
        match self {
            Self::Default => read_entry(raw, slot, &parse_plugin_source).await,
            Self::Host(reader) => {
                let slot = to_js_value(slot).unwrap_or_default();
                reader.call(vec![raw.clone(), slot]).await
            }
        }
    }
}

const NO_DESCRIPTION: &str = "No marketplace description provided. Adding a description helps users understand what this marketplace offers";

/// The name's reports: the grammar, Claude Desktop, and the reserved names (`validate` leaves out what only adding or loading refuses).
fn name_issues(name: &str, mode: ReadMode, context: &MarketplaceContext) -> Vec<PluginIssue> {
    let mut found = plugin_id_issues(IdPart::Marketplace, name, "name");
    found.extend(desktop_name_issues(IdPart::Marketplace, name, "name"));
    found.extend(match mode {
        ReadMode::Load => marketplace_name_issues(
            name,
            &NameOrigin {
                origin: context
                    .origin
                    .as_ref()
                    .and_then(|origin| to_js_value(origin).ok())
                    .unwrap_or_default(),
                hosted_on_claude_ai: context.hosted_on_claude_ai,
            },
        ),
        ReadMode::Validate => validation_name_issues(name),
    });
    found
}

/// The warnings of a marketplace whose schema passes: no description anywhere, no plugins. A
/// marketplace the schema rejects gets neither (oracle 2.1.319).
fn completeness_issues(top: &TopLevel) -> Vec<PluginIssue> {
    let mut found = Vec::new();
    if top.plugins.is_empty() {
        found.push(warning("plugins", "Marketplace has no plugins defined"));
    }
    let metadata_description = top.metadata.as_ref().and_then(|m| m.description.as_ref());
    if top.description.is_none() && metadata_description.is_none() {
        found.push(warning("description", NO_DESCRIPTION));
    }
    found
}

/// What only `claude plugin validate` reports of the top level: the `renames` chains, against the names in `plugins[]`.
fn rename_issues(top: &TopLevel, mode: ReadMode) -> Vec<PluginIssue> {
    if mode != ReadMode::Validate {
        return Vec::new();
    }
    let names: Vec<String> = top
        .plugins
        .iter()
        .filter(|raw| is_record(raw))
        .filter_map(|raw| raw.get("name").and_then(JsValue::as_str).map(str::to_owned))
        .collect();
    validate_renames(&top.renames, &names)
}

/// What reading the entries needs of the top level: they are read whether or not it is sound.
struct EntryHost<'a> {
    name: Option<&'a str>,
    plugins: &'a [JsValue],
    plugin_root: Option<&'a str>,
}

/// The entries read, and what was found.
struct EntriesRead {
    entries: Vec<MarketplaceEntry>,
    issues: Vec<PluginIssue>,
    schema_sound: bool,
}

async fn read_entries(
    top: &EntryHost<'_>,
    context: &MarketplaceContext,
    read_entry: EntryReader,
    mode: ReadMode,
) -> EntriesRead {
    let mut read: Vec<IndexedEntry> = Vec::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    let mut schema_sound = true;
    for (index, raw) in top.plugins.iter().enumerate() {
        let mut slot_context = context.clone();
        if let Some(name) = top.name {
            slot_context.marketplace_name = Some(name.to_owned());
        }
        let slot = EntrySlot {
            index: u32::try_from(index).unwrap_or(u32::MAX),
            plugin_root: top.plugin_root.map(str::to_owned),
            context: slot_context,
            mode: Some(mode),
        };
        let reading = read_entry.read(raw, &slot).await;
        issues.extend(reading.issues);
        if reading.schema_sound == Some(false) {
            schema_sound = false;
        }
        if let Some(entry) = reading.entry {
            read.push(IndexedEntry { index, entry });
        }
    }
    let unique = without_duplicates(&read);
    issues.extend(unique.issues);
    EntriesRead {
        entries: unique.entries,
        issues,
        schema_sound,
    }
}

fn marketplace_of(top: TopLevel, entries: Vec<MarketplaceEntry>) -> Marketplace {
    Marketplace {
        name: top.name,
        owner: top.owner,
        plugins: entries,
        description: top.description,
        version: top.version,
        metadata: top.metadata,
        force_remove_deleted_plugins: top.force_remove_deleted_plugins,
        allow_cross_marketplace_dependencies_on: top.allow_cross_marketplace_dependencies_on,
        renames: top.renames,
    }
}

/// The entries are read whether or not the top level is sound: the schema reports every one beside its errors (oracle 2.1.319).
fn host_of(raw: &JsValue) -> EntryHost<'_> {
    let metadata = raw.get("metadata").filter(|metadata| is_record(metadata));
    EntryHost {
        name: raw.get("name").and_then(JsValue::as_str),
        plugins: raw
            .get("plugins")
            .and_then(JsValue::as_array)
            .map_or(&[][..], Vec::as_slice),
        plugin_root: metadata
            .and_then(|metadata| metadata.get("pluginRoot"))
            .and_then(JsValue::as_str),
    }
}

async fn assemble(
    raw: &JsValue,
    context: &MarketplaceContext,
    read_entry: EntryReader,
    mode: ReadMode,
) -> MarketplaceResult {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return MarketplaceResult {
            marketplace: None,
            issues: vec![mismatch("marketplace.json", "object", raw)],
        };
    };
    let read = read_top_level(members);
    let top_issues = read.issues;
    let named = match raw.get("name").and_then(JsValue::as_str) {
        Some(name) => name_issues(name, mode, context),
        None => Vec::new(),
    };
    let mut gating = top_issues.clone();
    gating.extend(named.iter().cloned());
    let Some(top) = read.top else {
        let entries = read_entries(&host_of(raw), context, read_entry, mode).await;
        gating.extend(entries.issues);
        return MarketplaceResult {
            marketplace: None,
            issues: gating,
        };
    };
    let host = EntryHost {
        name: Some(&top.name),
        plugins: &top.plugins,
        plugin_root: top.metadata.as_ref().and_then(|m| m.plugin_root.as_deref()),
    };
    let entries = read_entries(&host, context, read_entry, mode).await;
    let blocked = blocks_loading(&gating);
    let sound = entries.schema_sound && !blocked;
    let mut issues = gating;
    if sound {
        issues.extend(completeness_issues(&top));
    }
    issues.extend(entries.issues);
    issues.extend(rename_issues(&top, mode));
    if blocked {
        return MarketplaceResult {
            marketplace: None,
            issues,
        };
    }
    MarketplaceResult {
        marketplace: Some(marketplace_of(top, entries.entries)),
        issues,
    }
}

/// The parsed `marketplace.json` as Claude Code loads it: an error in a top-level field or in the
/// name keeps the marketplace from loading; an entry with an error is dropped alone.
pub async fn read_marketplace(
    raw: &JsValue,
    context: &MarketplaceContext,
    read_entry: EntryReader,
) -> MarketplaceResult {
    assemble(raw, context, read_entry, ReadMode::Load).await
}

/// What `claude plugin validate` reports about a `marketplace.json`.
pub async fn validate_marketplace(
    raw: &JsValue,
    context: &MarketplaceContext,
    read_entry: EntryReader,
) -> Vec<PluginIssue> {
    assemble(raw, context, read_entry, ReadMode::Validate)
        .await
        .issues
}

/// The member `key` of a value, `None` for `null` or `undefined` (`??`).
fn present<'a>(value: Option<&'a JsValue>, key: &str) -> Option<&'a JsValue> {
    value
        .and_then(|value| value.get(key))
        .filter(|found| !matches!(found, JsValue::Null | JsValue::Undefined))
}

/// `description`, or `metadata.description` when the top level has none ("Alternate location").
#[must_use]
pub fn effective_description(marketplace: &JsValue) -> JsValue {
    let found = present(Some(marketplace), "description")
        .or_else(|| present(marketplace.get("metadata"), "description"));
    found.cloned().unwrap_or_default()
}

/// `version`, or `metadata.version` when the top level has none.
#[must_use]
pub fn effective_version(marketplace: &JsValue) -> JsValue {
    let found = present(Some(marketplace), "version")
        .or_else(|| present(marketplace.get("metadata"), "version"));
    found.cloned().unwrap_or_default()
}

fn reader_of(read_entry: Option<Callback>) -> EntryReader {
    read_entry.map_or(EntryReader::Default, EntryReader::Host)
}

crate::registry::exports! {
    "format/marketplace/marketplace" {
        "readMarketplace" => async ok |raw: JsValue, context: Option<MarketplaceContext>, read_entry: Option<Callback>|
            read_marketplace(&raw, &context.unwrap_or_default(), reader_of(read_entry)).await;
        "validateMarketplace" => async ok |raw: JsValue, context: Option<MarketplaceContext>, read_entry: Option<Callback>|
            validate_marketplace(&raw, &context.unwrap_or_default(), reader_of(read_entry)).await;
        "effectiveDescription" => ok |marketplace: JsValue| effective_description(&marketplace);
        "effectiveVersion" => ok |marketplace: JsValue| effective_version(&marketplace);
    }
}
