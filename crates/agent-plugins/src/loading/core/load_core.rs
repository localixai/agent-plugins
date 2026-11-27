//! Port of `src/loading/core/load-core.ts`, switched.
//!
//! Loading a core package: one crossing per load, the host's files read through one load's view of
//! them (`format/load_files.rs`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::assets::{core_extension_directories, core_icons};
use super::config::{read_core_hooks_file, read_core_mcp_file};
use super::locations::core_locate;
use super::manifest_step::core_manifest_step;
use super::skills::read_core_skills;
use crate::format::core::hooks::CoreHooks;
use crate::format::core::manifest::CoreManifest;
use crate::format::core::mcp::CoreMcpOptions;
use crate::format::core::mcp_server::{CoreMcpServer, CoreTransport};
use crate::format::core::versions::CoreVersion;
use crate::format::issues::PluginIssue;
use crate::format::load_files::{Files, load_files};
use crate::js::promise_all_pair;
use crate::loading::readers_text::PluginTextFile;
use crate::ports::Signal;

/// What a core package's load may be told (`CoreLoadOptions` of `src/loading/core/model.ts`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreLoadOptions {
    /// The extension namespaces the host implements.
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// The transports the host can start.
    #[serde(default)]
    pub transports: Option<Vec<CoreTransport>>,
    /// The icon formats the host shows.
    #[serde(default)]
    pub icon_formats: Option<Vec<String>>,
}

/// What a core package's load is given (`LoadCorePackageInput`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCorePackageInput {
    /// The package root.
    pub files: Files,
    /// The extension namespaces the host implements.
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// The transports the host can start.
    #[serde(default)]
    pub transports: Option<Vec<CoreTransport>>,
    /// The icon formats the host shows.
    #[serde(default)]
    pub icon_formats: Option<Vec<String>>,
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// A core package that loaded (`CorePackage`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorePackage {
    /// Its name.
    pub name: String,
    /// The version its manifest declares.
    pub version: CoreVersion,
    /// The manifest.
    pub manifest: CoreManifest,
    /// The skills.
    pub skills: Vec<PluginTextFile>,
    /// The MCP servers, by name.
    pub mcp_servers: IndexMap<String, CoreMcpServer>,
    /// The hooks.
    pub hooks: CoreHooks,
    /// The icons.
    pub icons: Vec<String>,
    /// The extension directories.
    pub extension_directories: Vec<String>,
}

/// What a core package's load gives (`LoadedCorePackage`).
#[derive(Clone, Debug, Serialize)]
pub struct LoadedCorePackage {
    /// The package; none when it is rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<CorePackage>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// Loads a core package (Agent Plugins 1.1.0, core §4–§10; docs/API.md, "Core packages"): the root
/// `plugin.json` first, which rejects the package when it fails (core §5.1–§5.3); then each
/// portable component type from its fixed location, each failing on its own narrowest boundary
/// (core §4.1, §6.2, §7.1, §7.2.2, §7.3.2, §11.3): an invalid type, server, event member, handler
/// or skill is reported and skipped while the rest loads. Nothing of the Claude Code layout is
/// read: a core package has no other component location (core §6.1). Never throws.
pub async fn load_core_package(input: LoadCorePackageInput) -> LoadedCorePackage {
    let signal = input.signal;
    let context = core_locate(load_files(input.files, signal.clone()), signal);
    let implemented = input.extensions.unwrap_or_default();
    let step = core_manifest_step(&context, &implemented).await;
    let (Some(manifest), Some(version)) = (step.manifest, step.version) else {
        return LoadedCorePackage {
            package: None,
            issues: step.issues,
        };
    };
    let mcp_options = CoreMcpOptions {
        implemented: Some(implemented.clone()),
        transports: input.transports,
    };
    let formats = input.icon_formats.unwrap_or_default();
    let ((skills, mcp), (hooks, (icons, extension_directories))) = promise_all_pair(
        promise_all_pair(
            Box::pin(read_core_skills(&context, &manifest.name)),
            Box::pin(read_core_mcp_file(&context, version, &mcp_options)),
        ),
        promise_all_pair(
            Box::pin(read_core_hooks_file(&context, version, &implemented)),
            promise_all_pair(
                Box::pin(core_icons(&context, &formats)),
                Box::pin(core_extension_directories(&context, &implemented)),
            ),
        ),
    )
    .await;
    let mut issues = step.issues;
    issues.extend(skills.issues);
    issues.extend(mcp.issues);
    issues.extend(hooks.issues);
    LoadedCorePackage {
        package: Some(CorePackage {
            name: manifest.name.clone(),
            version,
            manifest,
            skills: skills.skills,
            mcp_servers: mcp.servers,
            hooks: hooks.hooks,
            icons,
            extension_directories,
        }),
        issues,
    }
}

crate::registry::exports! {
    "loading/core/load-core" {
        "loadCorePackage" => async ok |input: LoadCorePackageInput| load_core_package(input).await;
    }
}
