//! Port of `src/cli/flags/settings.ts`, switched.
//!
//! `--setting-sources` and `--settings` (cli-reference.md, "CLI flags"). `--setting-sources` is "a
//! comma-separated list of setting sources to load (`user`, `project`, `local`)": managed policy is
//! read whatever the list says (agent-sdk/claude-code-features.md, "Control filesystem settings with
//! settingSources"). `--settings` is "a path to a settings JSON file or an inline JSON string": its
//! values "override the same keys in your `settings.json` files ... Keys you omit keep their
//! file-based values"; "the file must be a regular file no larger than 2 MiB". The refusals of the
//! file are errors.md's, "Settings file exceeds the 2MiB limit". Wording the docs do not give is in
//! spec/judgments.md.

use super::types::{Contribution, FlagContext, refuse};
use super::values::{Parsed, parse_json};
use crate::js::value::to_js_value;
use crate::js::{trim, trim_start};
use crate::settings::layers::PolicyScope;
use crate::settings::read_layer::read_settings_layer;

const FILE_SOURCES: [&str; 3] = ["user", "project", "local"];

fn is_file_source(name: &str) -> bool {
    FILE_SOURCES.contains(&name)
}

/// The scopes no list takes away: the policy tier and the `--settings` layer.
const ALWAYS: [PolicyScope; 3] = [PolicyScope::Managed, PolicyScope::Parent, PolicyScope::Flag];

/// `SCOPES_OF`: an `--add-dir` directory's settings are part of what `project` stands for (judgment).
fn scopes_of(source: &str) -> &'static [PolicyScope] {
    match source {
        "user" => &[PolicyScope::User],
        "project" => &[PolicyScope::Project, PolicyScope::AddDir],
        _ => &[PolicyScope::Local],
    }
}

/// `--setting-sources`.
#[must_use]
pub fn setting_sources(value: &str) -> Contribution {
    let names: Vec<&str> = value
        .split(',')
        .map(trim)
        .filter(|name| !name.is_empty())
        .collect();
    let unknown: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| !is_file_source(name))
        .collect();
    if !unknown.is_empty() {
        return refuse(vec![format!(
            "Invalid setting source: {}. Use user, project or local",
            unknown.join(", ")
        )]);
    }
    let chosen = FILE_SOURCES.iter().filter(|source| names.contains(source));
    let scopes = ALWAYS
        .iter()
        .chain(chosen.flat_map(|source| scopes_of(source)))
        .map(|scope| scope.name().to_owned())
        .collect();
    Contribution {
        setting_scopes: Some(scopes),
        ..Contribution::default()
    }
}

/// "The file must be a regular file no larger than 2 MiB."
pub const SETTINGS_FILE_LIMIT: f64 = 2.0 * 1024.0 * 1024.0;

/// The text of the settings, or the refusal that names the place.
fn settings_text(value: &str, context: &FlagContext) -> Result<String, String> {
    if trim_start(value).starts_with('{') {
        return Ok(value.to_owned());
    }
    let Some(file) = context.facts.files.get(value) else {
        return Err(format!("Error: Settings file not found: {value}"));
    };
    if !file.regular {
        return Err(format!(
            "Error: Cannot use settings file (Not a regular file (device, FIFO, or socket)): {value}"
        ));
    }
    if file.bytes > SETTINGS_FILE_LIMIT {
        return Err(format!(
            "Error: Settings file exceeds the 2MiB limit: {value}"
        ));
    }
    Ok(file.text.clone())
}

/// `--settings`.
#[must_use]
pub fn settings(value: &str, context: &FlagContext) -> Contribution {
    let text = match settings_text(value, context) {
        Ok(text) => text,
        Err(refusal) => return refuse(vec![refusal]),
    };
    let parsed = match parse_json(&text) {
        Parsed::Problem { problem, .. } => {
            return refuse(vec![format!(
                "Error: Invalid JSON passed to --settings: {problem}"
            )]);
        }
        Parsed::Value { value } => value,
    };
    let read = read_settings_layer(PolicyScope::Flag, &parsed, None);
    Contribution {
        flag_settings: to_js_value(&read.layer).ok(),
        warnings: Some(read.issues),
        ..Contribution::default()
    }
}

crate::registry::exports! {
    "cli/flags/settings" {
        "SETTINGS_FILE_LIMIT" => ok || SETTINGS_FILE_LIMIT;
    }
}
