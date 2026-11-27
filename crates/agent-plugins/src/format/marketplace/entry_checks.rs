//! Port of `src/format/marketplace/entry-checks.ts`, switched.
//!
//! The checks of a plugin entry beyond its field types (plugins/marketplace-reference.md,
//! "Validation messages"): the name, a name used twice, the archive-only header fields, and a
//! version that plugin.json overrides.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::header_names::is_routing_header;
use super::ids::{IdPart, desktop_name_issues, plugin_id_issues};
use super::marketplace_name_lists::OFFICIAL_NAMES;
use super::types::{MarketplaceEntry, PluginSource, ReadMode};
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::names::{NameContext, name_reserved_issues};
use crate::js::{JsValue, property_order};

/// A kept entry and the place it had in `plugins`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct IndexedEntry {
    /// The place in `plugins`.
    pub index: usize,
    /// The entry.
    pub entry: MarketplaceEntry,
}

/// The entries that stay and what the duplicates drew.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WithoutDuplicates {
    /// The entries kept.
    pub entries: Vec<MarketplaceEntry>,
    /// A report per entry dropped.
    pub issues: Vec<PluginIssue>,
}

/// Two entries with one name: "Duplicate plugin name "x" found in marketplace". The first keeps
/// the name; a later one is dropped alone (the docs say each entry is validated on its own and
/// leave the pair open: spec/judgments.md).
#[must_use]
pub fn without_duplicates(read: &[IndexedEntry]) -> WithoutDuplicates {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    for IndexedEntry { index, entry } in read {
        if seen.insert(entry.name.as_str()) {
            entries.push(entry.clone());
        } else {
            issues.push(error(
                &format!("plugins[{index}].name"),
                &format!(
                    "Duplicate plugin name \"{}\" found in marketplace",
                    entry.name
                ),
            ));
        }
    }
    WithoutDuplicates { entries, issues }
}

/// What `claude plugin validate` says of a name that passes as one of Anthropic's own. Loading
/// does not: "Claude Code still installs and loads a plugin whose name they refuse"
/// (manifest-reference.md, "`name`").
fn reserved(mode: ReadMode, name: &str, path: &str, marketplace: Option<&str>) -> Vec<PluginIssue> {
    match mode {
        ReadMode::Load => Vec::new(),
        // The kebab-case advice is the manifest's own; entries may use capitals (the id grammar
        // allows them). An official marketplace lists Anthropic's own plugins, so its entries pass.
        ReadMode::Validate => {
            if OFFICIAL_NAMES
                .iter()
                .any(|official| Some(*official) == marketplace)
            {
                Vec::new()
            } else {
                name_reserved_issues(name, Some(NameContext::Entry))
                    .into_iter()
                    .map(|found| PluginIssue {
                        path: path.to_owned(),
                        ..found
                    })
                    .collect()
            }
        }
    }
}

/// The reports on a name, in two parts.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NameReports {
    /// What is wrong with its shape, which an entry cannot be read past.
    pub shape: Vec<PluginIssue>,
    /// The reserved-name ruling, which leaves the entry otherwise sound.
    pub reserved: Vec<PluginIssue>,
}

/// The reports on a name, in two parts: what is wrong with its shape, which an entry cannot be
/// read past, and the reserved-name ruling, which leaves the entry otherwise sound.
#[must_use]
pub fn name_reports(
    name: &str,
    path: &str,
    mode: ReadMode,
    marketplace: Option<&str>,
) -> NameReports {
    if name.is_empty() {
        return NameReports {
            shape: vec![error(path, "Plugin must have a name")],
            reserved: Vec::new(),
        };
    }
    let mut shape = plugin_id_issues(IdPart::Plugin, name, path);
    shape.extend(desktop_name_issues(IdPart::Plugin, name, path));
    NameReports {
        shape,
        reserved: reserved(mode, name, path, marketplace),
    }
}

/// Which sources `headers` and `headersHelper` apply to: "only apply to "archive" sources".
const fn takes_headers(source: &PluginSource) -> bool {
    matches!(source, PluginSource::Archive { .. })
}

/// What an entry carries of the header fields.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFields {
    /// The plugin's name.
    pub name: String,
    /// Whether the entry is strict.
    pub strict: bool,
    /// `headers`, as written.
    #[serde(default)]
    pub headers: JsValue,
    /// `headersHelper`, as written.
    #[serde(default)]
    pub headers_helper: JsValue,
}

/// `headers`/`headersHelper` on a source that ignores them: "only apply to "archive" sources".
fn ignored_header_issues(entry: &HeaderFields, base: &str) -> Vec<PluginIssue> {
    [
        ("headers", &entry.headers),
        ("headersHelper", &entry.headers_helper),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_undefined())
    .map(|(field, _)| {
        warning(
            &format!("{base}.{field}"),
            &format!(
                "Plugin \"{}\" sets headers/headersHelper, which only apply to \"archive\" sources; they have no effect on this entry.",
                entry.name
            ),
        )
    })
    .collect()
}

/// "Header "x" is a request-routing/identity header that catalog entries may not set".
fn routing_header_issues(headers: &JsValue, base: &str) -> Vec<PluginIssue> {
    let JsValue::Object(members) = headers else {
        return Vec::new();
    };
    property_order(members)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| is_routing_header(name))
        .map(|name| {
            warning(
                &format!("{base}.headers.{name}"),
                &format!(
                    "Header \"{name}\" is a request-routing/identity header that catalog entries may not set; Claude Code drops it at download time."
                ),
            )
        })
        .collect()
}

/// A helper without `"strict": false` is an error; one on an archive without a pin is a warning.
fn helper_issues(entry: &HeaderFields, source: &PluginSource, base: &str) -> Vec<PluginIssue> {
    if entry.headers_helper.is_undefined() {
        return Vec::new();
    }
    let mut found = Vec::new();
    if entry.strict {
        found.push(error(
            &format!("{base}.headersHelper"),
            &format!(
                "Plugin \"{}\" sets headersHelper but is not \"strict\": false",
                entry.name
            ),
        ));
    }
    let pinned = matches!(
        source,
        PluginSource::Archive {
            sha256: Some(_),
            ..
        }
    );
    if !pinned {
        found.push(warning(
            &format!("{base}.source.sha256"),
            &format!(
                "Plugin \"{}\" fetches its archive with a headersHelper but sets no sha256 pin",
                entry.name
            ),
        ));
    }
    found
}

/// The header reports of an entry: the fields on a source that ignores them, or the ones that
/// apply to an archive.
#[must_use]
pub fn header_issues(entry: &HeaderFields, source: &PluginSource, base: &str) -> Vec<PluginIssue> {
    if !takes_headers(source) {
        return ignored_header_issues(entry, base);
    }
    let mut found = routing_header_issues(&entry.headers, base);
    found.extend(helper_issues(entry, source, base));
    found
}

/// Where plugin.json sits for a source whose files are inside the marketplace; other sources are
/// not read before install.
fn manifest_file(source: &PluginSource) -> Option<String> {
    match source {
        PluginSource::Relative { path, .. } => Some(format!(
            "{}.claude-plugin/plugin.json",
            if path == "." {
                String::new()
            } else {
                format!("{path}/")
            }
        )),
        PluginSource::Github { .. }
        | PluginSource::Url { .. }
        | PluginSource::GitSubdir { .. }
        | PluginSource::Npm { .. }
        | PluginSource::Archive { .. }
        | PluginSource::Command { .. } => None,
    }
}

/// "Entry declares version "x" but `<path>/plugin.json` says "y". At install time, plugin.json
/// wins", for a relative-path entry, "whose plugin files are inside the marketplace itself", when
/// the two versions differ.
#[must_use]
pub fn version_issues(
    version: Option<&str>,
    source: &PluginSource,
    manifest: &JsValue,
    base: &str,
) -> Vec<PluginIssue> {
    let file = manifest_file(source);
    let (Some(version), Some(file)) = (version, file) else {
        return Vec::new();
    };
    let Some(theirs) = manifest
        .get("version")
        .and_then(JsValue::as_str)
        .filter(|_| manifest.is_object())
    else {
        return Vec::new();
    };
    if theirs == version {
        return Vec::new();
    }
    vec![warning(
        &format!("{base}.version"),
        &format!(
            "Entry declares version \"{version}\" but {file} says \"{theirs}\". At install time, plugin.json wins (calculatePluginVersion precedence) \u{2014} the entry version is silently ignored. Update this entry to \"{theirs}\" to match."
        ),
    )]
}

crate::registry::exports! {
    "format/marketplace/entry-checks" {
        "withoutDuplicates" => ok |read: Vec<IndexedEntry>| without_duplicates(&read);
    }
}
