//! Port of `src/format/validate/inline-sources.ts`.
//!
//! The keys whose inline form is a strict object the validator reads entry by entry (spec §9.1.3,
//! §9.2.2; oracle 2.1.325): the `lspServers` map and the monitors array, at the top level and under
//! `experimental`. The loader reads the same schemas through the manifest's (manifest-schema.ts).

use super::finding::Finding;
use super::plugin_paths::{PATH_SHAPES, PathKey, path_shape_findings};
use super::zod_findings::zod_findings;
use crate::format::lsp_schema::LSP_SOURCES_SCHEMA;
use crate::format::monitor_schema::MONITOR_SOURCES_SCHEMA;
use crate::js::JsValue;
use crate::zod::Schema;

/// `INLINE_SCHEMAS[key]`: the schema of a key whose inline form is read entry by entry.
fn inline_schema(key: PathKey) -> Option<&'static Schema> {
    match key {
        PathKey::LspServers => Some(&LSP_SOURCES_SCHEMA),
        PathKey::Monitors => Some(&MONITOR_SOURCES_SCHEMA),
        PathKey::Skills
        | PathKey::Commands
        | PathKey::Agents
        | PathKey::Hooks
        | PathKey::McpServers
        | PathKey::OutputStyles
        | PathKey::Workflows
        | PathKey::Themes
        | PathKey::Types => None,
    }
}

/// The findings of one path-valued key: an inline LSP map or monitors array that breaks its schema
/// is reported at the path of each problem (`lspServers.go: Unrecognized key: "foo"`,
/// `experimental.monitors: Monitor names must be unique within a plugin`), or as `Invalid input` at
/// the key when no form fits; otherwise the path rules of `plugin-paths.ts`.
#[must_use]
pub fn source_findings(key: PathKey, path: &str, value: &JsValue) -> Vec<Finding> {
    let parsed = inline_schema(key).map(|schema| schema.safe_parse(value));
    if let Some(parsed @ Err(_)) = &parsed {
        return zod_findings(parsed, path, "");
    }
    path_shape_findings(path, PATH_SHAPES.of(key), value)
}

crate::registry::exports! {
    "format/validate/inline-sources" {
        "sourceFindings" => ok |key: PathKey, path: String, value: JsValue|
            source_findings(key, &path, &value);
    }
}
