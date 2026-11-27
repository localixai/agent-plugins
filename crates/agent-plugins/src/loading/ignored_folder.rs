//! Port of `src/loading/ignored-folder.ts`, switched.
//!
//! The warning for a default folder that exists and that a replacing manifest key leaves unscanned
//! (plugins/manifest-reference.md, "How each key combines with its default location"): "To avoid the
//! warning, set the key to a path inside that folder", and listing the folder itself keeps it. Only a
//! kind whose key replaces its default has a folder to ignore.

use serde::Deserialize;

use crate::format::issues::{PluginIssue, warning};
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT, declared_key};
use crate::format::messages::default_folder_ignored;
use crate::format::path_text::{PathPlatform, is_within, root_relative};
use crate::js::JsValue;
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// What discovery asks of its context here: the manifest, the folder, the platform and the signal
/// (`DiscoverContext`).
#[derive(Debug, Deserialize)]
pub struct DiscoverContext {
    /// The manifest, when there is one.
    #[serde(default)]
    pub manifest: JsValue,
    /// The plugin folder.
    pub files: Port,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The warning for a default folder that exists and that a replacing manifest key leaves unscanned.
pub async fn ignored_folder_issues(
    kind: ComponentKind,
    declared_paths: &[String],
    context: &DiscoverContext,
) -> Vec<PluginIssue> {
    let Some(folder) = STANDARD_LAYOUT.of(kind).folder else {
        return Vec::new();
    };
    let Some(declared) = declared_key(&context.manifest, kind) else {
        return Vec::new();
    };
    let present = context.files.stat(folder, context.signal.as_ref()).await;
    if !present.is_some_and(|stat| stat.is_directory) {
        return Vec::new();
    }
    let keeps = declared_paths
        .iter()
        .any(|path| is_within(folder, &root_relative(path, context.platform)));
    if keeps {
        Vec::new()
    } else {
        vec![warning(
            declared.key,
            &default_folder_ignored(folder, declared.key),
        )]
    }
}

crate::registry::exports! {
    "loading/ignored-folder" {
        "ignoredFolderIssues" => async ok |kind: ComponentKind, declared_paths: Vec<String>, context: DiscoverContext|
            ignored_folder_issues(kind, &declared_paths, &context).await;
    }
}
