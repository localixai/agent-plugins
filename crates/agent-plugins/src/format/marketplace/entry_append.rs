//! Ported from `src/format/marketplace/entry-append.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! How one component field of an entry joins the same field of plugin.json under strict mode
//! (plugins/manifest-reference.md, "How entry fields combine with `plugin.json`"): paths are
//! appended, a `commands` map adds its commands, and for `hooks` the entry's matchers replace the
//! manifest's per event.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsObject, in_property_order};

/// One path or a list of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Paths {
    /// One path.
    One(String),
    /// Several.
    Many(Vec<String>),
}

/// `PluginManifest['commands']`: paths, or a map of command name to source or content (the
/// content of a command is a `JsValue` until the manifest schema is in the core).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Commands {
    /// Paths.
    Paths(Paths),
    /// A map of command name to source or content.
    Map(JsObject),
}

/// A file of a hooks list: a path, or hooks written inline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookFile {
    /// A path.
    File(String),
    /// Hooks written inline.
    Inline(JsObject),
}

/// `PluginManifest['hooks']`: a file, hooks written inline, or a list of both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Hooks {
    /// A file.
    File(String),
    /// Hooks written inline.
    Inline(JsObject),
    /// A list of files and inline hooks.
    List(Vec<HookFile>),
}

fn list_of(value: &Paths) -> Vec<String> {
    match value {
        Paths::One(path) => vec![path.clone()],
        Paths::Many(paths) => paths.clone(),
    }
}

/// The manifest's paths followed by the entry's; when only one holds the field it stays as written.
#[must_use]
pub fn append_paths(manifest: Option<&Paths>, entry: Option<&Paths>) -> Option<Paths> {
    match (manifest, entry) {
        (Some(from_manifest), Some(from_entry)) => Some(Paths::Many(
            list_of(from_manifest)
                .into_iter()
                .chain(list_of(from_entry))
                .collect(),
        )),
        (Some(only), None) | (None, Some(only)) => Some(only.clone()),
        (None, None) => None,
    }
}

const MIXED_COMMANDS: &str = "The entry's \"commands\" are a map and plugin.json's are paths, or the other way round; they cannot be combined, so plugin.json's apply";

/// What `combineCommands` answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CombinedCommands {
    /// The field once combined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Commands>,
    /// What was reported.
    pub issues: Vec<PluginIssue>,
}

/// `commands` takes paths or a map of command name to source or content. Two lists append; two maps
/// merge, the entry's command replacing a manifest command of the same name (spec/judgments.md); a
/// list and a map cannot be held in one field, so the manifest's stays and the entry's is reported.
#[must_use]
pub fn combine_commands(manifest: Option<&Commands>, entry: Option<&Commands>) -> CombinedCommands {
    let quiet = |value| CombinedCommands {
        value,
        issues: Vec::new(),
    };
    match (manifest, entry) {
        (None, other) | (other, None) => quiet(other.cloned()),
        (Some(Commands::Paths(from_manifest)), Some(Commands::Paths(from_entry))) => {
            quiet(append_paths(Some(from_manifest), Some(from_entry)).map(Commands::Paths))
        }
        (Some(Commands::Map(from_manifest)), Some(Commands::Map(from_entry))) => {
            let mut merged = from_manifest.clone();
            merged.extend(from_entry.clone());
            quiet(Some(Commands::Map(in_property_order(merged))))
        }
        (Some(kept), Some(_)) => CombinedCommands {
            value: Some(kept.clone()),
            issues: vec![warning("commands", MIXED_COMMANDS)],
        },
    }
}

/// The inline hooks without the events `replaced` names.
fn without(inline: &JsObject, replaced: &JsObject) -> JsObject {
    in_property_order(
        inline
            .iter()
            .filter(|(event, _)| !replaced.contains_key(*event))
            .map(|(event, value)| (event.clone(), value.clone()))
            .collect(),
    )
}

/// The hooks of a plugin once the entry's are laid over them: for an event both declare, the
/// entry's matchers replace the manifest's; an event only one declares keeps its own. Hook files the
/// manifest names cannot be read here, so they stay and the entry's object follows them (the caller
/// reports which events it replaces).
#[must_use]
pub fn merge_hooks(manifest: Option<&Hooks>, entry: &JsObject) -> Hooks {
    match manifest {
        None => Hooks::Inline(entry.clone()),
        Some(Hooks::File(file)) => Hooks::List(vec![
            HookFile::File(file.clone()),
            HookFile::Inline(entry.clone()),
        ]),
        Some(Hooks::List(items)) => Hooks::List(
            items
                .iter()
                .map(|item| match item {
                    HookFile::Inline(inline) => HookFile::Inline(without(inline, entry)),
                    HookFile::File(file) => HookFile::File(file.clone()),
                })
                .chain(std::iter::once(HookFile::Inline(entry.clone())))
                .collect(),
        ),
        Some(Hooks::Inline(inline)) => {
            let mut merged = without(inline, entry);
            merged.extend(entry.clone());
            Hooks::Inline(in_property_order(merged))
        }
    }
}
