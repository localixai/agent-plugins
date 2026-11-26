//! Port of `src/loading/core/form.ts`, switched.
//!
//! The form of a plugin root, decided before any component is read (Agent Plugins Extended §1.3):
//!
//! | `.claude-plugin/plugin.json` | root `plugin.json`            | form                          |
//! | ---------------------------- | ----------------------------- | ----------------------------- |
//! | present                      | absent, or no core `$schema`  | Claude Code plugin            |
//! | present                      | with a core `$schema`         | Claude Code plugin, dual      |
//! | absent                       | with a core `$schema`         | core package                  |
//! | absent                       | absent, or no core `$schema`  | Claude Code plugin, no manifest |
//!
//! A root `plugin.json` whose `$schema` is no canonical core identifier, that is not JSON or not an
//! object, is an ordinary file (J1.1). A dual package loads as a Claude Code plugin only (§1.3.1,
//! J1.2); `dual` says that it also carries a core manifest, which a host shows in its details.
//!
//! Judgment (spec/judgments.md): a root `plugin.json` that is a link leading outside the root is not
//! read, so it is not a core manifest: the root is then a Claude Code plugin, which Claude Code would
//! load the same way since it never reads that file.

use serde::{Deserialize, Serialize};

use crate::format::core::versions::is_core_manifest_value;
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT};
use crate::format::path_text::{PathPlatform, leaves_folder};
use crate::format::read_json::{JsonRead, read_json};
use crate::js::promise_all_pair;
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// Both forms, in the package's order.
pub const PLUGIN_FORMS: [PluginForm; 2] = [PluginForm::ClaudeCode, PluginForm::Core];

/// One of `PLUGIN_FORMS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginForm {
    /// `claude-code`.
    #[serde(rename = "claude-code")]
    ClaudeCode,
    /// `core`.
    #[serde(rename = "core")]
    Core,
}

/// What the form of a root is, and whether it carries both manifests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PluginFormDecision {
    /// The form.
    pub form: PluginForm,
    /// The root carries both manifests (a dual package); its form is then `claude-code`.
    pub dual: bool,
}

const CORE_MANIFEST: &str = "plugin.json";

/// Whether the root holds a core manifest: a contained regular file with a canonical core `$schema`.
pub async fn has_core_manifest(files: &dyn PluginFiles, signal: Option<&Signal>) -> bool {
    let Some(stat) = files.stat(CORE_MANIFEST, signal).await else {
        return false;
    };
    if !stat.is_file || stat.check_failed.is_some() {
        return false;
    }
    if stat.is_symlink
        && stat
            .real_path
            .as_deref()
            .is_some_and(|real| leaves_folder(real, PathPlatform::Posix))
    {
        return false;
    }
    match read_json(files, CORE_MANIFEST, signal).await {
        JsonRead::Value { value } => is_core_manifest_value(&value),
        JsonRead::Problem { .. } => false,
    }
}

/// Decides the form of the plugin root `files` reads (§1.3).
pub async fn detect_plugin_form(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> PluginFormDecision {
    let (claude, core) = promise_all_pair(
        files.exists(STANDARD_LAYOUT.of(ComponentKind::Manifest).location, signal),
        Box::pin(has_core_manifest(files, signal)),
    )
    .await;
    if claude {
        return PluginFormDecision {
            form: PluginForm::ClaudeCode,
            dual: core,
        };
    }
    PluginFormDecision {
        form: if core {
            PluginForm::Core
        } else {
            PluginForm::ClaudeCode
        },
        dual: false,
    }
}

/// What the form is decided from.
#[derive(Debug, Deserialize)]
pub struct DetectInput {
    /// The plugin root.
    pub files: Port,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

crate::registry::exports! {
    "loading/core/form" {
        "PLUGIN_FORMS" => ok || PLUGIN_FORMS;
        "detectPluginForm" => async ok |input: DetectInput|
            detect_plugin_form(&input.files, input.signal.as_ref()).await;
    }
}
