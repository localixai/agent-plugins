//! Port of `src/loading/validate/validate-plugin.ts`, switched.
//!
//! `claude plugin validate <dir>` over a folder: one crossing per validation, the host's files read
//! through one load's view of them (`format/load_files.rs`).

use serde::{Deserialize, Serialize};

use super::components_only::components_only;
use super::marketplace_target::validate_marketplace_folder;
use super::plugin_target::{MANIFEST_FILE, validate_plugin_folder};
use super::report::ValidationTarget;
use crate::format::load_files::{Files, load_files};
use crate::format::validate::finding::{ValidationIssue, error_in};
use crate::format::validate::plugin_manifest::ManifestOptions;
use crate::js::promise_all_pair;
use crate::ports::Signal;
use crate::ports::files::PluginFiles;

/// What a validation is given (`ValidatePluginInput`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatePluginInput {
    /// Read access to the folder: its root is what `claude plugin validate <dir>` is given.
    pub files: Files,
    /// What the report calls the folder when it has nothing to validate; default `.`.
    #[serde(default)]
    pub label: Option<String>,
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// What a validation reports (`ValidationReport` of `src/loading/validate/report.ts`).
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    /// What the folder was validated as.
    pub target: ValidationTarget,
    /// The findings.
    pub issues: Vec<ValidationIssue>,
}

const MARKETPLACE_FILE: &str = ".claude-plugin/marketplace.json";
const NO_MANIFEST: &str = "No manifest found in directory. Expected .claude-plugin/marketplace.json or .claude-plugin/plugin.json";

/// `claude plugin validate <dir>` over a folder (docs/API.md, "Validating"): the marketplace when
/// the folder has `.claude-plugin/marketplace.json` (and its plugin too when it has a manifest),
/// else the plugin, else a bare skills tree; a folder with none of them is an error, and so is one
/// that does not exist. Findings are in the shape the official CLI of the same release reports
/// (docs/SPEC.md, "Sources of truth"). Never throws.
pub async fn validate_plugin(input: ValidatePluginInput) -> ValidationReport {
    let signal = input.signal.as_ref();
    let files = load_files(input.files.clone(), input.signal.clone());
    let label = input.label.as_deref().unwrap_or(".");
    if is_missing(&files, signal).await {
        let message = format!("File not found: {label}");
        return ValidationReport {
            target: ValidationTarget::None,
            issues: vec![error_in(label, "file", &message)],
        };
    }
    let (marketplace, plugin) = promise_all_pair(
        files.exists(MARKETPLACE_FILE, signal),
        files.exists(MANIFEST_FILE, signal),
    )
    .await;
    if marketplace {
        return marketplace_report(&files, plugin, signal).await;
    }
    if plugin {
        let issues = validate_plugin_folder(&files, &ManifestOptions::default(), signal).await;
        return ValidationReport {
            target: ValidationTarget::Plugin,
            issues,
        };
    }
    if let Some(components) = components_only(&files, label, signal).await {
        return ValidationReport {
            target: ValidationTarget::Components,
            issues: components,
        };
    }
    ValidationReport {
        target: ValidationTarget::None,
        issues: vec![error_in(label, "directory", NO_MANIFEST)],
    }
}

/// The folder does not exist: not even an empty one.
async fn is_missing(files: &dyn PluginFiles, signal: Option<&Signal>) -> bool {
    files.stat(".", signal).await.is_none() && files.list(".", signal).await.is_none()
}

/// The marketplace, and the plugin at its root too when the folder has a manifest.
async fn marketplace_report(
    files: &dyn PluginFiles,
    has_plugin: bool,
    signal: Option<&Signal>,
) -> ValidationReport {
    let folder = validate_marketplace_folder(files, signal).await;
    let mut issues = folder.issues;
    if has_plugin {
        let options = ManifestOptions {
            official: Some(folder.official),
            ..ManifestOptions::default()
        };
        issues.extend(validate_plugin_folder(files, &options, signal).await);
    }
    ValidationReport {
        target: ValidationTarget::Marketplace,
        issues,
    }
}

crate::registry::exports! {
    "loading/validate/validate-plugin" {
        "validatePlugin" => async ok |input: ValidatePluginInput| validate_plugin(input).await;
    }
}
