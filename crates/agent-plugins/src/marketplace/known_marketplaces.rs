//! Port of `src/marketplace/known-marketplaces.ts`, switched.
//!
//! `known_marketplaces.json`, "each marketplace Claude Code has fetched, with its `source`,
//! `installLocation`, `lastUpdated`, and `autoUpdate`", one per user (plugins/loading.md, "Check
//! which stage a plugin reached"); where a marketplace is on disk ("Find plugins on disk"); and the
//! two ways the registry fails to load (errors.md, "Failed to load marketplace configuration").
//!
//! The SDK types the `source` of an `extraKnownMarketplaces` entry, with `installLocation` and
//! `autoUpdate`; the file adds `lastUpdated`. The kit's `MarketplaceSource` names its member
//! `kind` (`directory`, `file`); the file writes that under `source`, as the settings key does. A
//! host that kept its registry in another shape migrates it to this one itself, once: the kit reads
//! and writes only the shape above and keeps no reader for another.
//! Judgments (spec/judgments.md): the wording of the corrupted-file message after its quoted lead;
//! a registry with any entry that does not match is read as empty, since every command that needs it fails.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::canonical_json::canonical_json;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::marketplace_source::{
    MarketplaceSourceOptions, parse_marketplace_source,
};
use crate::format::marketplace::types::MarketplaceSource;
use crate::js::json_parse::json_parse;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, json_stringify_indented, property_order, trim};
use crate::protocol::is_record::is_record;

/// One entry of the registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownMarketplace {
    /// Where it was added from.
    pub source: MarketplaceSource,
    /// `marketplaces/<name>/`, or for a local `file` or `directory` source the path you gave.
    pub install_location: String,
    /// When Claude Code last fetched it, as the file writes it (ISO text).
    pub last_updated: String,
    /// Whether the marketplace and its installed plugins update on startup; absent means the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
}

/// The registry by marketplace name.
pub type KnownMarketplaces = IndexMap<String, KnownMarketplace>;

/// `unreadable`: "isn't valid JSON or can't be read" (an empty file too); `corrupted`: valid JSON that does not match the schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistryFailureCode {
    /// Not JSON, or not read.
    Unreadable,
    /// JSON that does not match.
    Corrupted,
}

/// Why the registry did not load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegistryFailure {
    /// Which of the two ways.
    pub code: RegistryFailureCode,
    /// What Claude Code says.
    pub message: String,
}

/// The registry read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct KnownMarketplacesRead {
    /// The marketplaces, none when it failed.
    pub marketplaces: KnownMarketplaces,
    /// Why it did not load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<RegistryFailure>,
    /// The issue the failure is.
    pub issues: Vec<PluginIssue>,
}

const PATH: &str = "known_marketplaces.json";

fn read_entry(raw: &JsValue) -> Option<KnownMarketplace> {
    if !is_record(raw) {
        return None;
    }
    let install_location = raw.get("installLocation").and_then(JsValue::as_str)?;
    let last_updated = raw.get("lastUpdated").and_then(JsValue::as_str)?;
    let auto_update = match raw.get("autoUpdate") {
        None | Some(JsValue::Undefined) => None,
        Some(JsValue::Bool(value)) => Some(*value),
        Some(_) => return None,
    };
    let source = raw.get("source").cloned().unwrap_or_default();
    let options = MarketplaceSourceOptions {
        policy: false,
        key: None,
    };
    let source = parse_marketplace_source(&source, &options).source?;
    Some(KnownMarketplace {
        source,
        install_location: install_location.to_owned(),
        last_updated: last_updated.to_owned(),
        auto_update,
    })
}

fn failed(code: RegistryFailureCode, message: String) -> KnownMarketplacesRead {
    KnownMarketplacesRead {
        marketplaces: IndexMap::new(),
        issues: vec![error(PATH, &message)],
        failure: Some(RegistryFailure { code, message }),
    }
}

fn corrupted_message(names: &[&String]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
    let verb = if names.len() == 1 { "does" } else { "do" };
    format!(
        "Marketplace configuration file is corrupted: {} {verb} not match the registry schema",
        quoted.join(", ")
    )
}

const UNREADABLE_LEAD: &str = "Failed to load marketplace configuration";

/// What an empty file reports (errors.md: `JSON Parse error: Unexpected EOF`), the same on every runtime.
const EMPTY_DETAIL: &str = "JSON Parse error: Unexpected EOF";

/// The detail after the lead. An empty (or blank) file is the documented `JSON Parse error:
/// Unexpected EOF`. Judgment (spec/judgments.md): for any other text that is not JSON the detail is
/// the message of the runtime's own `JSON.parse`, which differs between engines (the docs show
/// only the empty file's), so only the lead is a promise.
fn parse_detail(text: &str, thrown: String) -> String {
    if trim(text).is_empty() {
        EMPTY_DETAIL.to_owned()
    } else {
        thrown
    }
}

/// The registry's text; no file is no marketplaces.
#[must_use]
pub fn parse_known_marketplaces(text: Option<&str>) -> KnownMarketplacesRead {
    let Some(text) = text else {
        return KnownMarketplacesRead {
            marketplaces: IndexMap::new(),
            failure: None,
            issues: Vec::new(),
        };
    };
    let parsed = match json_parse(text) {
        Ok(parsed) => parsed,
        Err(thrown) => {
            let message = format!("{UNREADABLE_LEAD}: {}", parse_detail(text, thrown));
            return failed(RegistryFailureCode::Unreadable, message);
        }
    };
    let Some(members) = parsed.as_object().filter(|_| is_record(&parsed)) else {
        return failed(
            RegistryFailureCode::Corrupted,
            "Marketplace configuration file is corrupted".to_owned(),
        );
    };
    // Own-property reads and `Object.fromEntries`: an entry named `__proto__` is an entry like any other.
    let read: Vec<(&String, Option<KnownMarketplace>)> = property_order(members)
        .into_iter()
        .map(|(name, raw)| (name, read_entry(raw)))
        .collect();
    let mismatched: Vec<&String> = read
        .iter()
        .filter(|(_, entry)| entry.is_none())
        .map(|(name, _)| *name)
        .collect();
    if !mismatched.is_empty() {
        return failed(
            RegistryFailureCode::Corrupted,
            corrupted_message(&mismatched),
        );
    }
    KnownMarketplacesRead {
        marketplaces: read
            .into_iter()
            .filter_map(|(name, entry)| Some((name.clone(), entry?)))
            .collect(),
        failure: None,
        issues: Vec::new(),
    }
}

/// A source in the shape the file writes: its kind under `source`.
fn wire(source: &JsValue) -> JsValue {
    let mut written = JsObject::new();
    written.insert(
        "source".to_owned(),
        source.get("kind").cloned().unwrap_or_default(),
    );
    if let Some(members) = source.as_object() {
        for (key, value) in property_order(members) {
            if key != "kind" {
                written.insert(key.clone(), value.clone());
            }
        }
    }
    JsValue::Object(written)
}

/// The registry's text, two-space indented with a final newline (a judgment, as for `installed_plugins.json`).
///
/// `marketplaces` is taken as the caller's object, so its members are written in the order the
/// caller built each entry, as `{ ...known, source }` keeps it.
#[must_use]
pub fn serialize_known_marketplaces(marketplaces: &JsObject) -> String {
    let written: JsObject = property_order(marketplaces)
        .into_iter()
        .map(|(name, known)| {
            let mut entry = known.as_object().cloned().unwrap_or_default();
            let source = wire(known.get("source").unwrap_or(&JsValue::Undefined));
            entry.insert("source".to_owned(), source);
            (name.clone(), JsValue::Object(entry))
        })
        .collect();
    let text = json_stringify_indented(&JsValue::Object(written), 2).unwrap_or_default();
    format!("{text}\n")
}

/// The sources with no copy under the plugins root: the registry points at the path you gave.
fn local_path(source: &MarketplaceSource) -> Option<&str> {
    match source {
        MarketplaceSource::File { path } | MarketplaceSource::Directory { path } => Some(path),
        MarketplaceSource::Url { .. }
        | MarketplaceSource::Github { .. }
        | MarketplaceSource::Git { .. }
        | MarketplaceSource::Npm { .. }
        | MarketplaceSource::Settings { .. }
        | MarketplaceSource::SkillsDir {}
        | MarketplaceSource::HostPattern { .. }
        | MarketplaceSource::PathPattern { .. } => None,
    }
}

/// Where the registry says a marketplace is: `marketplaces/<name>/` for one added from GitHub,
/// another Git host or a URL; for a local `file` or `directory` source "no copy here", so the path
/// you gave (plugins/loading.md, "Find plugins on disk"). Judgment (spec/judgments.md): the other
/// kinds, which the docs do not place, are kept like a clone.
///
/// `marketplace` is `layout.marketplace(name)`: the layout joins with the host's function and the
/// TypeScript answers at once, so the caller asks it (docs/RUST.md, "Effects").
#[must_use]
pub fn install_location_for(source: &MarketplaceSource, marketplace: &str) -> String {
    local_path(source).unwrap_or(marketplace).to_owned()
}

/// "Two sources match when they have the same type and the same value in every field" (plugins/
/// troubleshooting.md, "already added from a different source"): an entry that sets a `ref` you
/// did not pass differs, and so does a `github` entry from the same repository as an https URL
/// (recorded as `git`). Field order does not matter; list order does.
#[must_use]
pub fn same_marketplace_source(a: &MarketplaceSource, b: &MarketplaceSource) -> bool {
    let text =
        |source: &MarketplaceSource| canonical_json(&to_js_value(source).unwrap_or_default());
    text(a) == text(b)
}

crate::registry::exports! {
    "marketplace/known-marketplaces" {
        "parseKnownMarketplaces" => ok |text: Option<String>| parse_known_marketplaces(text.as_deref());
        "serializeKnownMarketplaces" => ok |marketplaces: JsObject| serialize_known_marketplaces(&marketplaces);
        "installLocationFor" => ok |source: MarketplaceSource, marketplace: String| install_location_for(&source, &marketplace);
        "sameMarketplaceSource" => ok |a: MarketplaceSource, b: MarketplaceSource| same_marketplace_source(&a, &b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_file_is_the_documented_eof() {
        let read = parse_known_marketplaces(Some("  "));
        let failure = read.failure.map(|failure| failure.message);
        assert_eq!(
            failure.as_deref(),
            Some("Failed to load marketplace configuration: JSON Parse error: Unexpected EOF")
        );
    }

    #[test]
    fn the_file_writes_the_kind_under_source() {
        let source = JsValue::from(JsObject::from_iter([
            ("kind".to_owned(), JsValue::from("directory")),
            ("path".to_owned(), JsValue::from("/m")),
        ]));
        let known = JsObject::from_iter([("source".to_owned(), source)]);
        let all = JsObject::from_iter([("m".to_owned(), JsValue::Object(known))]);
        assert_eq!(
            serialize_known_marketplaces(&all),
            "{\n  \"m\": {\n    \"source\": {\n      \"source\": \"directory\",\n      \"path\": \"/m\"\n    }\n  }\n}\n"
        );
    }
}
