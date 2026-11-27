//! Port of `src/skills/command-entries.ts`, switched.
//!
//! The manifest's `commands` object map (plugins/manifest-reference.md, "commands"; plugins/
//! components.md, "Define commands in the manifest"): each entry becomes a command named after the
//! plugin prefix and its key, with its body from inline `content` or from the `source` file, read
//! through the `PluginFiles` port. An entry that cannot load is an error issue and the others
//! still load: a plugin error is a value, never a throw.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::command::{CommandEntry, CommandEntryContext, read_command_entry};
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::path_escape::{DeclaredPath, escaped_path_issue};
use crate::format::path_text::PathPlatform;
use crate::js::{JsObject, promise_all};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;
use crate::rules::parse::{RuleOrigin, parse_rule};
use crate::rules::types::{PermissionBehavior, RuleSource};

/// The commands of the map and what was found loading them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandEntriesLoaded {
    /// In the map's key order.
    pub commands: Vec<JsObject>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The rules an entry's `allowedTools` grants, and the ones it drops.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Grants {
    /// The rules that parse, as written.
    pub rules: Vec<String>,
    /// Why the others grant nothing.
    pub issues: Vec<PluginIssue>,
}

/// The rules of `allowedTools` that parse as permission rules (permissions.md, "Permission rule
/// syntax"), as written. Judgment (spec/judgments.md): a rule that does not parse is dropped with
/// a warning and grants nothing, and the command still loads; a grant is read like a session rule.
#[must_use]
pub fn command_entry_grants(key: &str, rules: &[String]) -> Grants {
    let at = RuleOrigin {
        behavior: PermissionBehavior::Allow,
        source: RuleSource::Session,
    };
    let path = format!("commands.{key}.allowedTools");
    let mut kept = Vec::new();
    let mut issues = Vec::new();
    for rule in rules {
        let parsed = parse_rule(rule, &at);
        if parsed.rule.is_some() {
            kept.push(rule.clone());
        }
        issues.extend(parsed.issues.iter().map(|found| {
            warning(
                &path,
                &format!("{} (the rule grants nothing)", found.message),
            )
        }));
    }
    Grants {
        rules: kept,
        issues,
    }
}

struct Loaded {
    command: Option<JsObject>,
    issues: Vec<PluginIssue>,
}

/// `source` as a path of the plugin folder: a manifest path must start with `./` and hold no `..`
/// ("Path rules"), which also keeps a read inside the folder. Judgment (spec/judgments.md): the
/// port gets the path without the `./`, and an entry whose source breaks the rules loads nothing.
fn port_path(source: &str) -> &str {
    let mut path = source;
    while let Some(rest) = path.strip_prefix("./") {
        path = rest;
    }
    path
}

/// What refuses a `source`: no `./` start, or a path that leaves the plugin as written (`..`, an
/// absolute path, a backslash). Judgment: judged as on macOS and Linux, since the port takes `/`
/// paths.
fn source_issues(source: &str, field: &str) -> Vec<PluginIssue> {
    let mut issues = Vec::new();
    if !source.starts_with("./") {
        issues.push(error(
            field,
            &format!("{field} path must start with \"./\": {source}"),
        ));
    }
    let declared = DeclaredPath {
        component: field.to_owned(),
        written: source.to_owned(),
        platform: PathPlatform::Posix,
        resolves_to: None,
        roots: None,
        own: None,
    };
    issues.extend(escaped_path_issue(&declared));
    issues
}

struct Reader<'a> {
    files: &'a dyn PluginFiles,
    ctx: &'a CommandEntryContext,
    signal: &'a Signal,
}

/// The `source` file's text, or `None` for an entry that reads none; a refused path is an issue.
enum Source {
    Refused(Vec<PluginIssue>),
    Text(Option<String>),
}

async fn read_source(key: &str, entry: &CommandEntry, reader: &Reader<'_>) -> Source {
    let Some(source) = entry.source.as_deref().filter(|_| entry.content.is_none()) else {
        return Source::Text(None);
    };
    let refused = source_issues(source, &format!("commands.{key}.source"));
    if !refused.is_empty() {
        return Source::Refused(refused);
    }
    Source::Text(
        reader
            .files
            .read_text(port_path(source), Some(reader.signal))
            .await,
    )
}

async fn load_entry(
    key: &str,
    entry: &CommandEntry,
    reader: &Reader<'_>,
) -> Result<Loaded, String> {
    let grants = entry
        .allowed_tools
        .as_ref()
        .map(|rules| command_entry_grants(key, rules));
    let checked = match &grants {
        None => entry.clone(),
        Some(grants) => CommandEntry {
            allowed_tools: Some(grants.rules.clone()),
            ..entry.clone()
        },
    };
    let text = match read_source(key, entry, reader).await {
        Source::Refused(issues) => {
            return Ok(Loaded {
                command: None,
                issues,
            });
        }
        Source::Text(text) => text,
    };
    let read = read_command_entry(key, &checked, text.as_deref(), reader.ctx)?;
    let mut issues = grants.map(|grants| grants.issues).unwrap_or_default();
    issues.extend(read.issues);
    Ok(Loaded {
        command: read.command,
        issues,
    })
}

/// Load every entry of a `commands` object map. `files` is the plugin folder; a `source` is read
/// from it only when the entry sets `source` alone.
///
/// # Errors
///
/// When the context's origin is no `SkillOrigin`.
pub async fn load_command_entries(
    map: &IndexMap<String, CommandEntry>,
    files: &dyn PluginFiles,
    ctx: &CommandEntryContext,
    signal: &Signal,
) -> Result<CommandEntriesLoaded, String> {
    let reader = Reader { files, ctx, signal };
    let reader = &reader;
    let loaded = promise_all(
        map.iter()
            .map(|(key, entry)| -> BoxFuture<'_, _> { Box::pin(load_entry(key, entry, reader)) })
            .collect(),
    )
    .await;
    let mut all = CommandEntriesLoaded {
        commands: Vec::new(),
        issues: Vec::new(),
    };
    for one in loaded {
        let one = one?;
        all.commands.extend(one.command);
        all.issues.extend(one.issues);
    }
    Ok(all)
}

/// The options `loadCommandEntries` takes.
#[derive(Deserialize)]
struct LoadOptions {
    signal: Signal,
}

crate::registry::exports! {
    "skills/command-entries" {
        "commandEntryGrants" => ok |key: String, rules: Vec<String>| command_entry_grants(&key, &rules);
        "loadCommandEntries" => async fallible |map: IndexMap<String, CommandEntry>, files: Port, ctx: CommandEntryContext, options: LoadOptions|
            load_command_entries(&map, &files, &ctx, &options.signal).await;
    }
}
