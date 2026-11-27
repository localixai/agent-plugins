//! Port of `src/loading/validate/plugin-target.ts`, switched.
//!
//! Validating a plugin folder: its manifest `.claude-plugin/plugin.json`, whether the paths it
//! claims exist, then the folder's contents.

use crate::format::read_json::{JsonRead, invalid_json_syntax, read_json};
use crate::format::validate::finding::{ValidationIssue, error_at, error_in, in_file};
use crate::format::validate::path_claims::PathClaim;
use crate::format::validate::plugin_manifest::{ManifestOptions, check_plugin_manifest};
use crate::js::JsValue;
use crate::loading::validate::paths::{child_of, plugin_path};
use crate::loading::validate::plugin_contents::{PluginManifestSite, plugin_contents};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// Where a plugin's manifest is.
pub const MANIFEST_FILE: &str = ".claude-plugin/plugin.json";

const NOT_FOUND: &str = "The runtime loader will report this as a load failure.";
const SKIPPED: &str = "The plugin loader skips a missing MCP servers file without reporting it, so none of its servers will load.";

/// What a path claim found on disk, as the finding it earns, if any.
async fn claim_issue(
    files: &dyn PluginFiles,
    claim: &PathClaim,
    dir: &str,
    signal: Option<&Signal>,
) -> Option<String> {
    let Some(stat) = files
        .stat(&child_of(dir, &plugin_path(&claim.value)), signal)
        .await
    else {
        let tail = if claim.skipped_when_missing {
            SKIPPED
        } else {
            NOT_FOUND
        };
        return Some(format!("Path not found: {}. {tail}", claim.value));
    };
    if !claim.directory || !stat.is_file {
        return None;
    }
    let parent = claim
        .value
        .strip_suffix("/SKILL.md")
        .unwrap_or(&claim.value);
    let prefix = "Path is a file; skills entries must be directories containing SKILL.md";
    Some(if parent == claim.value {
        format!("{prefix}: {}", claim.value)
    } else {
        format!(
            "{prefix} — point to the parent directory '{parent}' instead: {}",
            claim.value
        )
    })
}

/// Where a manifest's claims are checked and reported.
pub struct ClaimSite<'a> {
    /// The file the findings are about.
    pub file: &'a str,
    /// The plugin folder the claimed paths are relative to; default the folder validated.
    pub dir: Option<&'a str>,
    /// Stops the reads.
    pub signal: Option<&'a Signal>,
}

/// The existence findings of a manifest's path claims, in claim order.
pub async fn claim_issues(
    files: &dyn PluginFiles,
    claims: &[PathClaim],
    site: &ClaimSite<'_>,
) -> Vec<ValidationIssue> {
    let mut found = Vec::new();
    for claim in claims {
        let message = claim_issue(files, claim, site.dir.unwrap_or("."), site.signal).await;
        if let Some(message) = message {
            found.push(error_in(site.file, &claim.path, &message));
        }
    }
    found
}

/// Validates the plugin whose manifest is `.claude-plugin/plugin.json`: the manifest, then its contents.
pub async fn validate_plugin_folder(
    files: &dyn PluginFiles,
    options: &ManifestOptions,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let value = match read_json(files, MANIFEST_FILE, signal).await {
        JsonRead::Problem { problem, .. } => {
            // The manifest's message ends with a period (spec §2.1 item 2); the marketplace file's does not (§15.2).
            let message = format!("{}.", invalid_json_syntax(&problem));
            return vec![error_in(MANIFEST_FILE, "json", &message)];
        }
        JsonRead::Value { value } => value,
    };
    let JsValue::Object(manifest) = &value else {
        return in_file(
            MANIFEST_FILE,
            &[error_at("root", "manifest must be an object")],
            "",
        );
    };
    let checks = check_plugin_manifest(&value, options);
    let site = ClaimSite {
        file: MANIFEST_FILE,
        dir: None,
        signal,
    };
    let mut issues = in_file(MANIFEST_FILE, &checks.findings, "");
    issues.extend(claim_issues(files, &checks.claims, &site).await);
    let contents = PluginManifestSite {
        manifest,
        file: MANIFEST_FILE,
        inline: checks.inline,
    };
    issues.extend(plugin_contents(files, &contents, signal).await);
    issues
}

crate::registry::exports! {
    "loading/validate/plugin-target" {
        "MANIFEST_FILE" => ok || MANIFEST_FILE;
        "validatePluginFolder" => async ok |files: Port, options: Option<ManifestOptions>, signal: Option<Signal>|
            validate_plugin_folder(&files, &options.unwrap_or_default(), signal.as_ref()).await;
    }
}
