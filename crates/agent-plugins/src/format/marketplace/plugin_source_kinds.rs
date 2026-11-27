//! Port of `src/format/marketplace/plugin-source-kinds.ts`, switched.
//!
//! The object plugin sources, one parser each (plugins/marketplace-reference.md, "Plugin sources"
//! and its sub-sections): the field rules the docs state. A parser returns the source, or the
//! issues that say why the object matches no type. What a field means at fetch time is the fetch
//! plan's.

use indexmap::IndexMap;

use super::archive_url::archive_url_issues;
use super::plugin_source_fields::{
    Checked, RawSource, SourceSite, field_error, invalid_input, is_text, optional_digest,
    optional_text, read_git_pin, refined, required,
};
use super::types::{CommandMode, PluginSource, PluginSourceKind};
use crate::format::issues::{PluginIssue, error};
use crate::js::value::to_js_value;
use crate::js::{JsValue, code_unit_length};
use crate::registry::Function;

/// `Parsed`: a `PluginSource`, or the issues that say why the object matches no type.
pub type Parsed = JsValue;

/// `ObjectParser`.
pub type ObjectParser = fn(&RawSource, &SourceSite) -> Parsed;

/// The issues as a parse's answer.
fn issues_of(issues: &[PluginIssue]) -> Parsed {
    to_js_value(&issues).unwrap_or_default()
}

/// A source with the issues of its fields (`refined`).
fn refined_source(source: &PluginSource, issues: &[PluginIssue]) -> Parsed {
    refined(&to_js_value(source).unwrap_or_default(), issues)
}

fn github(raw: &RawSource, site: &SourceSite) -> Parsed {
    let repo = required(raw, "repo");
    let pin = read_git_pin(raw, site);
    let (Some(repo), Some(pin)) = (repo, pin) else {
        return issues_of(&invalid_input(site));
    };
    let source = PluginSource::Github {
        repo,
        reference: pin.value.reference,
        sha: pin.value.sha,
    };
    refined_source(&source, &pin.issues)
}

fn url(raw: &RawSource, site: &SourceSite) -> Parsed {
    let address = required(raw, "url");
    let pin = read_git_pin(raw, site);
    let (Some(address), Some(pin)) = (address, pin) else {
        return issues_of(&invalid_input(site));
    };
    let source = PluginSource::Url {
        url: address,
        reference: pin.value.reference,
        sha: pin.value.sha,
    };
    refined_source(&source, &pin.issues)
}

fn git_subdir(raw: &RawSource, site: &SourceSite) -> Parsed {
    let address = required(raw, "url");
    let path = required(raw, "path");
    let pin = read_git_pin(raw, site);
    let (Some(address), Some(path), Some(pin)) = (address, path, pin) else {
        return issues_of(&invalid_input(site));
    };
    let source = PluginSource::GitSubdir {
        url: address,
        path,
        reference: pin.value.reference,
        sha: pin.value.sha,
    };
    refined_source(&source, &pin.issues)
}

/// "An `npm` `package` containing `..`" is `Invalid input` ("Invalid input on a source").
fn npm(raw: &RawSource, site: &SourceSite) -> Parsed {
    let pkg = required(raw, "package");
    let version = optional_text(raw, "version");
    let registry = optional_text(raw, "registry");
    let (Some(pkg), Some(version), Some(registry)) = (pkg, version, registry) else {
        return issues_of(&invalid_input(site));
    };
    if pkg.contains("..") {
        return issues_of(&invalid_input(site));
    }
    let source = PluginSource::Npm {
        package: pkg,
        version: version.value,
        registry: registry.value,
    };
    to_js_value(&source).unwrap_or_default()
}

fn archive(raw: &RawSource, site: &SourceSite) -> Parsed {
    let address = required(raw, "url");
    let sha256 = optional_digest(raw, site);
    let (Some(address), Some(sha256)) = (address, sha256) else {
        return issues_of(&invalid_input(site));
    };
    let mut issues = archive_url_issues(&address, site);
    issues.extend(sha256.issues);
    let source = PluginSource::Archive {
        url: address,
        sha256: sha256.value,
    };
    refined_source(&source, &issues)
}

const MAX_COMMAND_LENGTH: usize = 500;
const COMMAND_TEXT: &str = "command must be printable ASCII (letters, digits, punctuation, single spaces) with no runs of 4 or more spaces";
const COMMAND_LENGTH: &str = "command must not be longer than the install consent UI can display";

/// `/^[\x20-\x7E]*$/`.
fn printable_ascii(text: &str) -> bool {
    text.chars()
        .all(|character| matches!(character, '\x20'..='\x7E'))
}

/// `/ {4}/`.
fn four_spaces(text: &str) -> bool {
    text.contains("    ")
}

/// The rules for a command string ("command plugin source": "Write it as printable ASCII, at most
/// 500 characters, with no run of four or more spaces"), with the two messages of spec §15.6.6 and
/// J15.8 (oracle 2.1.325): over 500 characters the length message; a character outside U+0020 to
/// U+007E, or a run of four spaces, the text message; both, length first, when both break.
#[must_use]
pub fn command_text_issues(command: &str, path: &str) -> Vec<PluginIssue> {
    let mut found = Vec::new();
    if code_unit_length(command) > MAX_COMMAND_LENGTH {
        found.push(error(path, COMMAND_LENGTH));
    }
    if !printable_ascii(command) || four_spaces(command) {
        found.push(error(path, COMMAND_TEXT));
    }
    found
}

const DEFAULT_TIMEOUT_SECONDS: f64 = 60.0;
const MAX_TIMEOUT_SECONDS: f64 = 600.0;

fn timeout_range_issues(value: f64, site: &SourceSite) -> Vec<PluginIssue> {
    if value <= 0.0 {
        return vec![field_error(
            site,
            "timeout",
            "Too small: expected number to be >0",
        )];
    }
    if value <= MAX_TIMEOUT_SECONDS {
        return Vec::new();
    }
    vec![field_error(
        site,
        "timeout",
        "Too big: expected number to be <=600",
    )]
}

/// `timeout`: a whole number of seconds from 1 to 600, 60 when absent; not a whole number is no
/// value at all.
fn read_timeout(raw: &RawSource, site: &SourceSite) -> Option<Checked<f64>> {
    let value = raw.get("timeout").unwrap_or(&JsValue::Undefined);
    if value.is_undefined() {
        return Some(Checked {
            value: DEFAULT_TIMEOUT_SECONDS,
            issues: Vec::new(),
        });
    }
    let value = value
        .as_f64()
        .filter(|number| number.is_finite() && number.trunc() == *number)?;
    Some(Checked {
        value,
        issues: timeout_range_issues(value, site),
    })
}

/// `mode`: `copy` (the default) or `link` ("Copy mode and link mode").
fn read_mode(value: &JsValue) -> Option<CommandMode> {
    if value.is_undefined() {
        return Some(CommandMode::Copy);
    }
    match value.as_str() {
        Some("copy") => Some(CommandMode::Copy),
        Some("link") => Some(CommandMode::Link),
        _ => None,
    }
}

fn read_command(raw: &RawSource, site: &SourceSite) -> Parsed {
    let text = raw.get("command").unwrap_or(&JsValue::Undefined);
    let timeout = read_timeout(raw, site);
    let mode = read_mode(raw.get("mode").unwrap_or(&JsValue::Undefined));
    let (Some(command), Some(timeout), Some(mode)) =
        (text.as_str().filter(|_| is_text(text)), timeout, mode)
    else {
        return issues_of(&invalid_input(site));
    };
    let mut broken = command_text_issues(command, &format!("{}.command", site.path));
    broken.extend(timeout.issues);
    let source = PluginSource::Command {
        command: command.to_owned(),
        timeout_seconds: timeout.value,
        mode,
    };
    refined_source(&source, &broken)
}

/// One parser per object kind: a new `PluginSourceKind` fails the typecheck until it has one.
pub const OBJECT_PARSERS: [(PluginSourceKind, ObjectParser); 6] = [
    (PluginSourceKind::Github, github),
    (PluginSourceKind::Url, url),
    (PluginSourceKind::GitSubdir, git_subdir),
    (PluginSourceKind::Npm, npm),
    (PluginSourceKind::Archive, archive),
    (PluginSourceKind::Command, read_command),
];

/// One entry of `OBJECT_PARSERS`, a function the host calls.
struct Parser(ObjectParser);

crate::registry::methods! {
    Parser {
        "call" => ok |this, raw: RawSource, site: SourceSite| (this.0)(&raw, &site);
    }
}

/// `OBJECT_PARSERS` as the host's table of functions.
fn object_parsers() -> IndexMap<PluginSourceKind, Function> {
    OBJECT_PARSERS
        .iter()
        .map(|(kind, parser)| (*kind, Function::new(Parser(*parser))))
        .collect()
}

crate::registry::exports! {
    "format/marketplace/plugin-source-kinds" {
        "commandTextIssues" => ok |command: String, path: String| command_text_issues(&command, &path);
        "OBJECT_PARSERS" => ok object_parsers;
    }
}
