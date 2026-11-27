//! Port of `src/loading/validate/marketplace-target.ts`, switched.
//!
//! Validating a marketplace folder: `.claude-plugin/marketplace.json`, then the `plugin.json` of
//! each entry whose source is a folder inside the marketplace.

use serde::Serialize;

use crate::format::issues::{PluginIssue, Severity};
use crate::format::marketplace::entries::EntrySlot;
use crate::format::marketplace::marketplace::{EntryReader, validate_marketplace};
use crate::format::marketplace::marketplace_name_lists::OFFICIAL_NAMES;
use crate::format::marketplace::plugin_source::parse_plugin_source;
use crate::format::marketplace::types::{MarketplaceContext, PluginSource};
use crate::format::names::NameContext;
use crate::format::read_json::{JsonRead, invalid_json_syntax, read_json};
use crate::format::validate::finding::{ValidationIssue, error_in, in_file, warning_in};
use crate::format::validate::plugin_manifest::{ManifestOptions, check_plugin_manifest};
use crate::js::{JsValue, promise_all};
use crate::loading::validate::links::is_link;
use crate::loading::validate::paths::{child_of, plugin_path};
use crate::loading::validate::plugin_target::{ClaimSite, claim_issues};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Map, Port};

const MARKETPLACE_FILE: &str = ".claude-plugin/marketplace.json";
const ENTRY_MANIFEST: &str = ".claude-plugin/plugin.json";

/// What an entry's own `plugin.json` turned out to be.
enum EntryManifest {
    Manifest { dir: String, value: JsValue },
    Unparsed { dir: String, problem: String },
    NotADirectory { dir: String },
    Linked { via: LinkedVia },
    None,
}

/// What of an entry's folder is a link.
#[derive(Clone, Copy)]
enum LinkedVia {
    Path,
    File,
}

/// The folder of a source inside the marketplace; the other kinds are read from elsewhere.
fn local_path(source: &PluginSource) -> Option<String> {
    if let PluginSource::Relative { path, .. } = source {
        Some(plugin_path(path))
    } else {
        None
    }
}

/// The local folder an entry's `source` names, as a `PluginFiles` path.
fn local_dir(entry: &JsValue, index: usize, plugin_root: Option<&str>) -> Option<String> {
    let JsValue::Object(entry) = entry else {
        return None;
    };
    let slot = EntrySlot {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        plugin_root: plugin_root.map(str::to_owned),
        context: MarketplaceContext::default(),
        mode: None,
    };
    let source = entry.get("source").unwrap_or(&JsValue::Undefined);
    let read = parse_plugin_source(source, &slot);
    read.source.as_ref().and_then(local_path)
}

async fn read_entry_manifest(
    files: &dyn PluginFiles,
    dir: Option<String>,
    signal: Option<&Signal>,
) -> EntryManifest {
    let Some(dir) = dir else {
        return EntryManifest::None;
    };
    let stat = files.stat(&dir, signal).await;
    if stat.is_some_and(|stat| stat.is_file) {
        return EntryManifest::NotADirectory { dir };
    }
    let path = child_of(&dir, ENTRY_MANIFEST);
    for folder in [dir.clone(), child_of(&dir, ".claude-plugin")] {
        if is_link(files, &folder, signal).await {
            return EntryManifest::Linked {
                via: LinkedVia::Path,
            };
        }
    }
    if is_link(files, &path, signal).await {
        return EntryManifest::Linked {
            via: LinkedVia::File,
        };
    }
    if !files.exists(&path, signal).await {
        return EntryManifest::None;
    }
    match read_json(files, &path, signal).await {
        JsonRead::Value { value } => EntryManifest::Manifest { dir, value },
        JsonRead::Problem { problem, .. } => EntryManifest::Unparsed { dir, problem },
    }
}

fn file_link() -> String {
    format!(
        "{ENTRY_MANIFEST} is a symlink or is not a regular file, so it was not validated. Install reads it anyway. Validate it separately."
    )
}

fn path_link(source: &str) -> String {
    format!(
        "Local source \"{source}\" is or traverses a symlink, so {ENTRY_MANIFEST} was not read — validation never follows one. Install dereferences symlinks that stay inside the marketplace, so this entry still loads. Validate the real path separately."
    )
}

fn prefix_of(index: usize) -> String {
    format!("plugins[{index}] plugin.json → ")
}

/// What an entry's checks need beside its manifest.
struct EntryOptions<'a> {
    official: bool,
    source: &'a str,
    entry_name: Option<&'a str>,
    signal: Option<&'a Signal>,
}

/// The reports of one entry's own manifest, prefixed with the entry as the oracle does.
async fn entry_issues(
    files: &dyn PluginFiles,
    index: usize,
    manifest: &EntryManifest,
    options: &EntryOptions<'_>,
) -> Vec<ValidationIssue> {
    let prefix = prefix_of(index);
    let (dir, value) = match manifest {
        EntryManifest::None => return Vec::new(),
        EntryManifest::Linked { via } => {
            let message = match via {
                LinkedVia::File => file_link(),
                LinkedVia::Path => path_link(options.source),
            };
            let at = format!("plugins[{index}].source");
            return vec![warning_in(MARKETPLACE_FILE, &at, &message)];
        }
        EntryManifest::NotADirectory { dir } => {
            let message = format!(
                "A path component of {} is not a directory, so the file cannot be read. The entry will fail to load at install time.",
                child_of(dir, ENTRY_MANIFEST)
            );
            return vec![error_in(
                MARKETPLACE_FILE,
                &format!("{prefix}file"),
                &message,
            )];
        }
        EntryManifest::Unparsed { dir, problem } => {
            let cross_check = format!(
                "Could not parse {} for version cross-check: {problem}",
                child_of(dir, ENTRY_MANIFEST)
            );
            return vec![
                warning_in(
                    MARKETPLACE_FILE,
                    &format!("plugins[{index}].source"),
                    &cross_check,
                ),
                error_in(
                    MARKETPLACE_FILE,
                    &format!("{prefix}json"),
                    &invalid_json_syntax(problem),
                ),
            ];
        }
        EntryManifest::Manifest { dir, value } => (dir, value),
    };
    let checks = check_plugin_manifest(
        value,
        &ManifestOptions {
            official: Some(options.official),
            context: Some(NameContext::Entry),
            entry_name: options.entry_name.map(str::to_owned),
        },
    );
    let site = ClaimSite {
        file: MARKETPLACE_FILE,
        dir: Some(dir),
        signal: options.signal,
    };
    let claimed = claim_issues(files, &checks.claims, &site).await;
    let mut issues = in_file(MARKETPLACE_FILE, &checks.findings, &prefix);
    issues.extend(claimed.into_iter().map(|issue| ValidationIssue {
        path: format!("{prefix}{}", issue.path),
        ..issue
    }));
    issues
}

/// The schema's own errors name their place with dots (`plugins.0.source`), the validator's checks
/// with brackets (`plugins[0].source`); the kit reads both through one reader, so a schema error is
/// recognised by its wording (oracle 2.1.319).
const SCHEMA_ERROR: [&str; 5] = [
    "Invalid input",
    "Invalid string",
    "Invalid key",
    "Unrecognized key",
    "Plugin name cannot contain spaces",
];

/// `path.replaceAll(/\[(\d+)\]/g, '.$1')`: each bracketed index written with a dot.
fn with_dots(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(open) = rest.find('[') {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let after = from.get(1..).unwrap_or("");
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let close = after
            .get(digits..)
            .is_some_and(|tail| tail.starts_with(']'));
        if digits > 0 && close {
            out.push('.');
            out.push_str(after.get(..digits).unwrap_or(""));
            rest = after.get(digits + 1..).unwrap_or("");
        } else {
            out.push('[');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn dotted(issue: PluginIssue) -> ValidationIssue {
    let schema = issue.severity == Severity::Error
        && SCHEMA_ERROR
            .iter()
            .any(|start| issue.message.starts_with(start))
        && issue.path.starts_with("plugins[");
    ValidationIssue {
        severity: issue.severity,
        file: MARKETPLACE_FILE.to_owned(),
        path: if schema {
            with_dots(&issue.path)
        } else {
            issue.path
        },
        message: issue.message,
    }
}

/// Whether the marketplace's own report puts a schema error on entry `index`: its manifest is then
/// not read. The name and source checks are made apart from the schema and do not stop it.
fn broken_entry(issues: &[ValidationIssue], index: usize) -> bool {
    let apart = [
        format!("plugins[{index}].name"),
        format!("plugins[{index}].source"),
        format!("plugins.{index}.name"),
        format!("plugins.{index}.source"),
    ];
    let bracketed = format!("plugins[{index}]");
    let dotted = format!("plugins.{index}.");
    issues.iter().any(|issue| {
        if issue.severity != Severity::Error {
            return false;
        }
        let own = issue.path.starts_with(&bracketed) || issue.path.starts_with(&dotted);
        own && !apart.iter().any(|own| issue.path.starts_with(own.as_str()))
    })
}

/// What validating a marketplace folder found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MarketplaceFolder {
    /// The findings.
    pub issues: Vec<ValidationIssue>,
    /// The marketplace is one of Anthropic's own.
    pub official: bool,
}

/// The text of an entry's field, when it is text.
fn text_of<'a>(entry: Option<&'a JsValue>, key: &str) -> Option<&'a str> {
    entry.and_then(|entry| entry.as_object()?.get(key)?.as_str())
}

/// The entries of the file, each with the manifest its local folder holds.
async fn read_entry_manifests<'a>(
    files: &'a dyn PluginFiles,
    raw: &'a JsValue,
    signal: Option<&'a Signal>,
) -> (&'a [JsValue], Vec<EntryManifest>) {
    let entries = raw
        .as_object()
        .and_then(|raw| raw.get("plugins")?.as_array())
        .map_or(&[][..], Vec::as_slice);
    let root = raw.as_object().and_then(|raw| {
        raw.get("metadata")?
            .as_object()?
            .get("pluginRoot")?
            .as_str()
    });
    let reads: Vec<BoxFuture<'_, EntryManifest>> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| -> BoxFuture<'_, EntryManifest> {
            Box::pin(async move {
                let dir = local_dir(entry, index, root);
                read_entry_manifest(files, dir, signal).await
            })
        })
        .collect();
    (entries, promise_all(reads).await)
}

/// `claude plugin validate` on a marketplace folder: its `marketplace.json` and its local entries.
pub async fn validate_marketplace_folder(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> MarketplaceFolder {
    let raw = match read_json(files, MARKETPLACE_FILE, signal).await {
        JsonRead::Problem { problem, .. } => {
            let message = invalid_json_syntax(&problem);
            return MarketplaceFolder {
                issues: vec![error_in(MARKETPLACE_FILE, "json", &message)],
                official: false,
            };
        }
        JsonRead::Value { value } => value,
    };
    let (entries, manifests) = read_entry_manifests(files, &raw, signal).await;
    let found = manifests
        .iter()
        .enumerate()
        .filter_map(|(index, manifest)| {
            let EntryManifest::Manifest { value, .. } = manifest else {
                return None;
            };
            Some((u32::try_from(index).unwrap_or(u32::MAX), value.clone()))
        })
        .collect();
    let context = MarketplaceContext {
        plugin_manifests: Some(Map { entries: found }),
        ..MarketplaceContext::default()
    };
    let own: Vec<ValidationIssue> = validate_marketplace(&raw, &context, EntryReader::Default)
        .await
        .into_iter()
        .map(dotted)
        .collect();
    let name = raw.get("name").and_then(JsValue::as_str);
    let official = raw.is_object() && name.is_some_and(|name| OFFICIAL_NAMES.contains(&name));
    let mut nested = Vec::new();
    for (index, manifest) in manifests.iter().enumerate() {
        if broken_entry(&own, index) {
            continue;
        }
        let entry = entries.get(index);
        let options = EntryOptions {
            official,
            source: text_of(entry, "source").unwrap_or(""),
            entry_name: text_of(entry, "name"),
            signal,
        };
        nested.extend(entry_issues(files, index, manifest, &options).await);
    }
    MarketplaceFolder {
        issues: own.into_iter().chain(nested).collect(),
        official,
    }
}

crate::registry::exports! {
    "loading/validate/marketplace-target" {
        "validateMarketplaceFolder" => async ok |files: Port, signal: Option<Signal>|
            validate_marketplace_folder(&files, signal.as_ref()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::with_dots;

    #[test]
    fn writes_bracketed_indexes_with_dots() {
        assert_eq!(with_dots("plugins[0].tags[12]"), "plugins.0.tags.12");
        assert_eq!(with_dots("plugins[a][]x[3"), "plugins[a][]x[3");
        assert_eq!(with_dots("[[1]]"), "[.1]");
    }
}
