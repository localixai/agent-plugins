//! Port of `src/cli/validate-run.ts`, switched.
//!
//! What `claude plugin validate` reports and how it exits (plugins/cli-reference.md, "plugin
//! validate", "Output and exit codes", "Advice on the README"; plugins/manifest-reference.md,
//! "Validate the manifest"). The kit validates a folder through `validatePlugin`; this turns its
//! findings into the verdict line, the exit code and the `--json` report. Pure: a host prints.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::result::ExitCode;
use crate::format::issues::Severity;
use crate::format::validate::finding::ValidationIssue;
use crate::js::{JsObject, JsValue, key_order};

/// The target a run validates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationTarget {
    /// A marketplace.
    Marketplace,
    /// A plugin.
    Plugin,
    /// Component files only.
    Components,
    /// A bare run.
    None,
}

/// The manifest a target is validated through: the report's `manifest`, or none for a bare run.
const fn manifest_file(target: ValidationTarget) -> Option<&'static str> {
    match target {
        ValidationTarget::Marketplace => Some(".claude-plugin/marketplace.json"),
        ValidationTarget::Plugin => Some(".claude-plugin/plugin.json"),
        ValidationTarget::Components | ValidationTarget::None => None,
    }
}

/// How a run ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationVerdict {
    /// `true` when the run exits `0`.
    pub success: bool,
    /// The exit code.
    pub exit_code: ExitCode,
    /// `Validation passed`, `Validation passed with warnings`, `Validation failed` ...
    pub line: String,
}

/// The verdict: an error fails the run; a warning fails it only under `--strict`; "Advice doesn't
/// count, with or without `--strict`", so it is not among the findings.
#[must_use]
pub fn validation_verdict(issues: &[Severity], strict: bool) -> ValidationVerdict {
    let errors = issues.contains(&Severity::Error);
    let warnings = issues.contains(&Severity::Warning);
    if errors {
        return ValidationVerdict {
            success: false,
            exit_code: 1,
            line: "Validation failed".to_owned(),
        };
    }
    if warnings && strict {
        return ValidationVerdict {
            success: false,
            exit_code: 1,
            line: "Validation failed (--strict treats warnings as errors)".to_owned(),
        };
    }
    ValidationVerdict {
        success: true,
        exit_code: 0,
        line: if warnings {
            "Validation passed with warnings"
        } else {
            "Validation passed"
        }
        .to_owned(),
    }
}

/// `Pick<ValidationIssue, 'severity'>`: all the verdict reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct HasSeverity {
    /// How much the finding weighs.
    pub severity: Severity,
}

/// Exit `2`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnexpectedValidation {
    /// `2`.
    pub exit_code: ExitCode,
    /// The message.
    pub stderr: String,
}

/// Exit `2`: "the validator itself failed, such as on an unreadable path"; the message goes to stderr, stdout stays empty.
#[must_use]
pub fn unexpected_validation_error(reason: &str) -> UnexpectedValidation {
    UnexpectedValidation {
        exit_code: 2,
        stderr: format!("Unexpected error during validation: {reason}"),
    }
}

/// A mod hook that can refuse an action (such as `tool.call`), and whether it has a `.catch` handler
/// (J17.15): `module` as `modules` writes it, `pattern` the event pattern given to `on`, `hook` that
/// pattern with its matcher in braces (`tool.call{tool=Bash}`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatingHook {
    /// The module.
    pub module: String,
    /// The event pattern.
    pub pattern: String,
    /// The pattern with its matcher.
    pub hook: String,
    /// Whether it has a `.catch` handler.
    pub has_catch: bool,
}

/// One entry of `errors` or `warnings` (J17.15); a `notes` entry is a plain string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationEntry {
    /// The place.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

/// One file's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationFileResult {
    /// The file.
    pub file: String,
    /// Its errors.
    pub errors: Vec<ValidationEntry>,
    /// Its warnings.
    pub warnings: Vec<ValidationEntry>,
    /// Its notes.
    pub notes: Vec<String>,
    /// Its gating hooks.
    pub gating_hooks: Vec<GatingHook>,
}

/// One entry of `advice`: the README, the message and the line to paste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationAdvice {
    /// The README.
    pub file: String,
    /// The message.
    pub message: String,
    /// The line to paste.
    pub install_line: String,
}

/// The `--json` report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationJson {
    /// The verdict the exit code gives.
    pub success: bool,
    /// Whether warnings failed the run.
    pub strict: bool,
    /// The resolved path validated.
    pub target: String,
    /// The manifest's own result (null with no manifest).
    pub manifest: Option<ValidationFileResult>,
    /// One result per other file.
    pub contents: Vec<ValidationFileResult>,
    /// The advice.
    pub advice: Vec<ValidationAdvice>,
}

fn result_of(
    file: &str,
    issues: &[ValidationIssue],
    notes: Option<&Vec<String>>,
    gating_hooks: Option<&Vec<GatingHook>>,
) -> ValidationFileResult {
    let pick = |severity: Severity| -> Vec<ValidationEntry> {
        issues
            .iter()
            .filter(|issue| issue.severity == severity)
            .map(|issue| ValidationEntry {
                path: issue.path.clone(),
                message: issue.message.clone(),
            })
            .collect()
    };
    ValidationFileResult {
        file: file.to_owned(),
        errors: pick(Severity::Error),
        warnings: pick(Severity::Warning),
        notes: notes.cloned().unwrap_or_default(),
        gating_hooks: gating_hooks.cloned().unwrap_or_default(),
    }
}

/// What `validationJson` is given.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationJsonInput {
    /// What is validated.
    pub target: ValidationTarget,
    /// The resolved path Claude Code validated.
    pub path: String,
    /// The findings.
    pub issues: Vec<ValidationIssue>,
    /// `--strict`.
    pub strict: bool,
    /// The advice.
    #[serde(default)]
    pub advice: Option<Vec<ValidationAdvice>>,
    /// The gating hooks the mods validator found, by the file that declares them.
    #[serde(default)]
    pub gating_hooks: Option<IndexMap<String, Vec<GatingHook>>>,
    /// The mods validator's lines, by the file they belong to.
    #[serde(default)]
    pub notes: Option<IndexMap<String, Vec<String>>>,
}

/// The `--json` report: `success` is the verdict the exit code gives, `strict` whether warnings
/// failed the run, `target` the resolved path validated, `manifest` the manifest's own result (null
/// with no manifest), `contents` one result per other file, only for files with something to report,
/// and `advice`. Each result carries `gatingHooks`, the mod hooks that can refuse an action. For a directory with `marketplace.json` and `plugin.json`, `manifest` is the
/// marketplace result and the plugin's go in `contents`. The shape inside a result is a judgment
/// (spec/judgments.md): the docs name `file`, `errors`, `warnings` and `notes`, not what an entry holds.
#[must_use]
pub fn validation_json(input: &ValidationJsonInput) -> ValidationJson {
    let manifest_file = manifest_file(input.target);
    let empty_gates = IndexMap::new();
    let empty_notes = IndexMap::new();
    let gates = input.gating_hooks.as_ref().unwrap_or(&empty_gates);
    let notes = input.notes.as_ref().unwrap_or(&empty_notes);
    let mut by_file: IndexMap<String, Vec<ValidationIssue>> = IndexMap::new();
    for issue in &input.issues {
        by_file
            .entry(issue.file.clone())
            .or_default()
            .push(issue.clone());
    }
    // Judgment: a file whose only report is a gating hook or a note still has a result (spec/judgments.md).
    for file in key_order(gates.keys())
        .into_iter()
        .chain(key_order(notes.keys()))
    {
        by_file.entry(file.clone()).or_default();
    }
    let result_for = |file: &str| {
        result_of(
            file,
            by_file.get(file).map_or(&[][..], Vec::as_slice),
            notes.get(file),
            gates.get(file),
        )
    };
    let contents = by_file
        .keys()
        .filter(|file| Some(file.as_str()) != manifest_file)
        .map(|file| result_for(file))
        .collect();
    let severities: Vec<Severity> = input.issues.iter().map(|issue| issue.severity).collect();
    ValidationJson {
        success: validation_verdict(&severities, input.strict).success,
        strict: input.strict,
        target: input.path.clone(),
        manifest: manifest_file.map(result_for),
        contents,
        advice: input.advice.clone().unwrap_or_default(),
    }
}

/// Judgment (spec/judgments.md): the fields only `marketplace.json` defines and the ones only
/// `plugin.json` defines, for a file the docs let the validator classify by its fields ("such as
/// `owner`", "such as `author`"). The marketplace set is `owner` and `plugins`; the plugin set is
/// the manifest's own component and identity keys.
const MARKETPLACE_ONLY: [&str; 2] = ["owner", "plugins"];
const PLUGIN_ONLY: [&str; 13] = [
    "author",
    "commands",
    "agents",
    "skills",
    "hooks",
    "mcpServers",
    "lspServers",
    "userConfig",
    "channels",
    "monitors",
    "dependencies",
    "settings",
    "defaultEnabled",
];

/// What a manifest file is validated as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ManifestKind {
    /// A marketplace.
    Marketplace,
    /// A plugin.
    Plugin,
}

/// What `manifestFileKind` is given.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFileInput {
    /// The file's name.
    pub name: String,
    /// Whether it sits in a `.claude-plugin` directory.
    pub in_claude_plugin_directory: bool,
    /// Its parsed value.
    pub value: JsObject,
}

/// What a manifest file is validated as (plugins/cli-reference.md, "Validate a manifest file"):
/// `plugin.json` and `marketplace.json` by name; any other file in a `.claude-plugin` directory is a
/// plugin manifest; elsewhere a marketplace when `plugins` is an array, when `plugins` and `owner` are
/// both present, or when it has a marketplace-only field and no plugin-only one; otherwise a plugin.
#[must_use]
pub fn manifest_file_kind(input: &ManifestFileInput) -> ManifestKind {
    if input.name == "marketplace.json" {
        return ManifestKind::Marketplace;
    }
    if input.name == "plugin.json" || input.in_claude_plugin_directory {
        return ManifestKind::Plugin;
    }
    let has = |key: &&str| input.value.contains_key(*key);
    if matches!(input.value.get("plugins"), Some(JsValue::Array(_))) {
        return ManifestKind::Marketplace;
    }
    if has(&"plugins") && has(&"owner") {
        return ManifestKind::Marketplace;
    }
    if MARKETPLACE_ONLY.iter().any(has) && !PLUGIN_ONLY.iter().any(has) {
        ManifestKind::Marketplace
    } else {
        ManifestKind::Plugin
    }
}

crate::registry::exports! {
    "cli/validate-run" {
        "validationVerdict" => ok |issues: Vec<HasSeverity>, strict: bool| validation_verdict(&issues.iter().map(|issue| issue.severity).collect::<Vec<_>>(), strict);
        "unexpectedValidationError" => ok |reason: String| unexpected_validation_error(&reason);
        "validationJson" => ok |input: ValidationJsonInput| validation_json(&input);
        "manifestFileKind" => ok |input: ManifestFileInput| manifest_file_kind(&input);
    }
}
