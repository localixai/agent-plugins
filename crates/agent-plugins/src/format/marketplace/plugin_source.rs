//! Port of `src/format/marketplace/plugin-source.ts`, switched.
//!
//! Reading a plugin entry's `source` (plugins/marketplace-reference.md, "Plugin sources", "Relative
//! path plugin source", "Bare names under pluginRoot", "Validation messages"): a relative path
//! string, or an object whose own `source` key names one of the other six types. It is the
//! `SourceParser` the entry reader is given (`createEntryReader(parsePluginSource)`).

use serde::Deserialize;

use super::entries::{EntrySlot, SourceReading};
use super::plugin_source_fields::{SourceSite, invalid_input};
use super::plugin_source_kinds::{OBJECT_PARSERS, ObjectParser, Parsed};
use super::types::{PluginSource, PluginSourceKind, ReadMode};
use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::{PathPlatform, is_absolute_path, root_relative};
use crate::js::{JsObject, JsValue};
use crate::protocol::is_record::is_record;

const MUST_START: &str = "Invalid string: must start with \"./\"";

/// The line an entry whose path does not stay inside the marketplace is listed with (`errors.md`).
fn entry_path_refused(path: &str) -> String {
    format!(
        "Plugin source path refused: {path} does not stay inside its marketplace directory. Check that the marketplace entry has a plain relative path."
    )
}

/// The tag `source: 'unsupported'` is the SDK's placeholder, not an authorable type.
const PLACEHOLDER_MESSAGE: &str =
    "source.source: 'unsupported' is a parse-time placeholder and cannot be authored";
const PLACEHOLDER_TAG: &str = "unsupported";

/// `isObjectKind(tag) ? OBJECT_PARSERS[tag]`: the parser of an object kind, by its tag.
fn object_parser(tag: &str) -> Option<ObjectParser> {
    let tag = PluginSourceKind::deserialize(JsValue::from(tag)).ok()?;
    OBJECT_PARSERS
        .iter()
        .find(|(kind, _)| *kind == tag)
        .map(|(_, parser)| *parser)
}

/// A parser's answer as a reading: a `PluginSource` (it has a `kind`), or the issues.
fn outcome(parsed: Parsed) -> SourceReading {
    match PluginSource::deserialize(parsed.clone()) {
        Ok(source) => SourceReading {
            source: Some(source),
            issues: Vec::new(),
        },
        Err(_) => refused(Vec::<PluginIssue>::deserialize(parsed).unwrap_or_default()),
    }
}

fn refused(issues: Vec<PluginIssue>) -> SourceReading {
    SourceReading {
        source: None,
        issues,
    }
}

/// `metadata.pluginRoot` "must itself be a relative path inside the marketplace": no `..`, not
/// absolute, not empty. Written with or without `./` (spec/judgments.md).
fn root_path(plugin_root: &str, site: &SourceSite) -> Option<String> {
    let path = if site.platform == PathPlatform::Windows {
        plugin_root.replace('\\', "/")
    } else {
        plugin_root.to_owned()
    };
    let bad =
        path.is_empty() || path.contains("..") || path.contains('\\') || is_absolute_path(&path);
    if bad {
        None
    } else {
        Some(root_relative(&path, PathPlatform::default()))
    }
}

/// A bare name: "a single directory name with no `/`", resolved under `metadata.pluginRoot`.
fn bare_name(raw: &str, site: &SourceSite) -> SourceReading {
    let root = site
        .plugin_root
        .as_deref()
        .and_then(|plugin_root| root_path(plugin_root, site));
    if site.plugin_root.is_none() || raw.contains('/') || raw.is_empty() {
        return refused(vec![error(&site.path, MUST_START)]);
    }
    // A root that climbs out or is absolute leaves the bare name without a base: oracle 2.1.325
    // reports the bare name like one without `./` (spec J15.6).
    let Some(root) = root else {
        return refused(vec![error(&site.path, MUST_START)]);
    };
    let path = if root == "." {
        raw.to_owned()
    } else {
        format!("{root}/{raw}")
    };
    SourceReading {
        source: Some(PluginSource::Relative { path, bare: true }),
        issues: Vec::new(),
    }
}

fn relative(raw: &str, site: &SourceSite) -> SourceReading {
    if raw.contains("..") {
        return refused(vec![error(
            &site.path,
            &format!("Path contains \"..\": {raw}"),
        )]);
    }
    if raw == "." || raw.starts_with("./") {
        return with_separators(raw, site);
    }
    bare_name(raw, site)
}

/// On macOS and Linux a backslash after the leading `./` is refused by the client at install and at
/// load, and "validation does not report it" (spec §15.6.1 items 2 and 4): validating keeps the path
/// as written; loading refuses the entry with the load line of `errors.md`, "Marketplace entry path
/// does not stay inside the marketplace directory". On Windows a backslash is a separator.
const fn refuses_backslash(mode: ReadMode) -> bool {
    match mode {
        ReadMode::Load => true,
        ReadMode::Validate => false,
    }
}

fn with_separators(raw: &str, site: &SourceSite) -> SourceReading {
    let rest = if raw == "." {
        ""
    } else {
        raw.get(2..).unwrap_or_default()
    };
    if site.platform == PathPlatform::Posix
        && rest.contains('\\')
        && refuses_backslash(site.mode.unwrap_or(ReadMode::Load))
    {
        return refused(vec![error(&site.path, &entry_path_refused(raw))]);
    }
    let path = root_relative(rest, site.platform);
    SourceReading {
        source: Some(PluginSource::Relative { path, bare: false }),
        issues: Vec::new(),
    }
}

fn from_object(raw: &JsObject, site: &SourceSite) -> SourceReading {
    let tag = raw.get("source");
    if tag.and_then(JsValue::as_str) == Some(PLACEHOLDER_TAG) {
        return refused(vec![error(&site.path, PLACEHOLDER_MESSAGE)]);
    }
    let Some(parser) = tag.and_then(JsValue::as_str).and_then(object_parser) else {
        return refused(invalid_input(site));
    };
    outcome(parser(raw, site))
}

/// The plugin source of `plugins[i]`: the source, or `undefined` with the issues that say why.
#[must_use]
pub fn parse_plugin_source(raw: &JsValue, slot: &EntrySlot) -> SourceReading {
    let site = SourceSite {
        path: format!("plugins[{}].source", slot.index),
        platform: slot.context.platform.unwrap_or(PathPlatform::Posix),
        mode: Some(slot.mode.unwrap_or(ReadMode::Load)),
        plugin_root: slot.plugin_root.clone(),
    };
    if let JsValue::String(text) = raw {
        return relative(text, &site);
    }
    match raw.as_object() {
        Some(members) if is_record(raw) => from_object(members, &site),
        _ => refused(invalid_input(&site)),
    }
}

crate::registry::exports! {
    "format/marketplace/plugin-source" {
        "parsePluginSource" => ok |raw: JsValue, slot: EntrySlot| parse_plugin_source(&raw, &slot);
    }
}
