//! Ported from `src/format/core/document.ts`; only the core uses it now, so it has no wrapper.
//!
//! The outer object of `mcp.json` and `hooks.json` (core §7.2.1, §7.3.1, §10.1): exactly `$schema`
//! and one member (`mcpServers`, `hooks`) that is an object; a `$schema` the kit implements, of the
//! same Agent Plugins version as `plugin.json`. Any failure disables that component type only (core
//! §7.2.2 item 2, §7.3.2 item 2).

use serde::Serialize;

use super::versions::{
    CORE_VERSIONS, CoreDocument, CoreVersion, core_schema_version, supported_core_version,
};
use crate::js::{JsObject, JsValue, property_order};

/// The member the document holds, or why the whole component is disabled.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentMembers {
    /// The member's object.
    Members(JsObject),
    /// Why the component is disabled.
    Problem(String),
}

/// The member the document holds, or why the whole component is disabled.
#[must_use]
pub fn document_members(
    raw: &JsValue,
    document: CoreDocument,
    member: &str,
    manifest_version: CoreVersion,
) -> DocumentMembers {
    let JsValue::Object(fields) = raw else {
        return DocumentMembers::Problem(format!(
            "{}.json is not a JSON object",
            document.as_str()
        ));
    };
    let schema = raw.get("$schema").unwrap_or(&JsValue::Undefined);
    let Some(named) = core_schema_version(schema, document) else {
        return DocumentMembers::Problem(format!(
            "\"$schema\" must be the canonical {}.json identifier",
            document.as_str()
        ));
    };
    let Some(version) = supported_core_version(Some(&named)) else {
        let implemented: Vec<&str> = CORE_VERSIONS.iter().map(|known| known.as_str()).collect();
        return DocumentMembers::Problem(format!(
            "Unsupported Agent Plugins version {named}: this client implements {}",
            implemented.join(", ")
        ));
    };
    if version != manifest_version {
        return DocumentMembers::Problem(format!(
            "targets Agent Plugins {}, but plugin.json targets {}",
            version.as_str(),
            manifest_version.as_str()
        ));
    }
    let extra = property_order(fields)
        .into_iter()
        .map(|(key, _)| key)
        .find(|key| key.as_str() != "$schema" && key.as_str() != member);
    if let Some(extra) = extra {
        return DocumentMembers::Problem(format!("unknown top-level field \"{extra}\""));
    }
    match fields.get(member) {
        Some(JsValue::Object(members)) => DocumentMembers::Members(members.clone()),
        Some(_) | None => DocumentMembers::Problem(format!("\"{member}\" must be an object")),
    }
}
