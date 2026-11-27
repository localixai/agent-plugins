//! Port of `src/loading/core/manifest-step.ts`, switched.
//!
//! The first step of a core load (core §5.1: the manifest is loaded and validated before any
//! component is discovered): a root `plugin.json` that is missing, not a regular file, outside the
//! plugin root (core §4.1, boundary 1), not JSON or invalid by core §5 rejects the package.

use super::locations::{FixedJson, fixed_json};
use crate::format::core::manifest::{CoreManifestResult, read_core_manifest};
use crate::format::issues::{PluginIssue, error};
use crate::format::locate::LocateContext;

const FILE: &str = "plugin.json";

/// The manifest of a core package, or why the package is rejected.
pub async fn core_manifest_step(
    context: &LocateContext,
    implemented: &[String],
) -> CoreManifestResult {
    match fixed_json(context, FILE, "The package is rejected").await {
        FixedJson::Absent { .. } => CoreManifestResult {
            manifest: None,
            version: None,
            issues: vec![error(
                FILE,
                "A core package needs a plugin.json at its root (core §4.1)",
            )],
        },
        FixedJson::Issue { issue } => CoreManifestResult {
            manifest: None,
            version: None,
            issues: vec![PluginIssue {
                path: FILE.to_owned(),
                ..issue
            }],
        },
        FixedJson::Value { value } => read_core_manifest(&value, implemented),
    }
}

crate::registry::exports! {
    "loading/core/manifest-step" {
        "coreManifestStep" => async ok |context: LocateContext, implemented: Vec<String>|
            core_manifest_step(&context, &implemented).await;
    }
}
