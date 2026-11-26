//! Port of `src/loading/manifest-gate.ts`, switched.
//!
//! What stops a plugin before anything of it is read (spec §17.18.3, §17.18.5): its own directory
//! that the system could not check (`plugin path could not be checked: . (<code>)`), and a
//! `.claude-plugin/plugin.json` the kit cannot confirm is inside the plugin, or for a plugin of a
//! marketplace inside that marketplace: `Plugin not loaded: its manifest <path> was not read,
//! because <reason>`, the reason ending in the error code when the path could not be checked. The
//! component name `plugin` and the reason `it could not be checked (<code>)` are judgments
//! (spec/judgments.md): the specification gives the form of the message, not these words. A link is
//! judged only when the host passes `roots`, as for every other plugin path.

use serde::Deserialize;

use crate::format::host_errors::{
    manifest_not_read, path_could_not_be_checked, reported_check_code,
};
use crate::format::issues::{PluginIssue, error};
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT};
use crate::format::link_rule::{LinkReach, LinkReachInput, link_reach};
use crate::format::path_escape::LinkRoots;
use crate::format::path_text::{PathPlatform, resolve_from, without_trailing_slashes};
use crate::js::promise_all_pair;
use crate::ports::Signal;
use crate::ports::files::{FileStat, PluginFiles};
use crate::registry::Port;

/// What the gate may be given.
#[derive(Debug, Default, Deserialize)]
pub struct GateOptions {
    /// Where the plugin and its marketplace really are.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Whose path rules apply; `posix` by default.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

const FOLDER: &str = ".claude-plugin";

/// Where a link among the manifest's path leads: through its folder, or the file itself.
fn link_target(folder: Option<&FileStat>, file: Option<&FileStat>) -> Option<String> {
    if let Some(folder) = folder
        && folder.is_symlink
        && let Some(real) = &folder.real_path
    {
        return Some(format!("{}/plugin.json", without_trailing_slashes(real)));
    }
    file.filter(|file| file.is_symlink)
        .and_then(|file| file.real_path.clone())
}

fn outside_reason(target: &str, roots: &LinkRoots, platform: PathPlatform) -> Option<String> {
    let real = resolve_from(&roots.plugin_root, target, platform);
    let reach = link_reach(&LinkReachInput {
        target: real.clone(),
        plugin_root: roots.plugin_root.clone(),
        marketplace_root: roots.marketplace_root.clone(),
    });
    if reach != LinkReach::Outside {
        return None;
    }
    let marketplace = if roots.marketplace_root.is_none() {
        ""
    } else {
        " and its marketplace directory"
    };
    Some(format!(
        "it resolves to {real}, outside the plugin directory{marketplace}"
    ))
}

/// The plugin folder itself, when the system could not check it.
async fn root_issue(files: &dyn PluginFiles, signal: Option<&Signal>) -> Option<PluginIssue> {
    let stat = files.stat(".", signal).await?;
    let code = stat.check_failed?;
    Some(error(
        "plugin",
        &path_could_not_be_checked("plugin", ".", &code),
    ))
}

/// Why the manifest is not read: a check that failed, or a link that leads outside.
fn manifest_reason(
    folder: Option<&FileStat>,
    file: Option<&FileStat>,
    options: &GateOptions,
) -> Option<String> {
    let failed = folder
        .and_then(|stat| stat.check_failed.as_ref())
        .or_else(|| file.and_then(|stat| stat.check_failed.as_ref()));
    if let Some(failed) = failed {
        return Some(format!(
            "it could not be checked ({})",
            reported_check_code(failed)
        ));
    }
    let target = link_target(folder, file)?;
    let roots = options.roots.as_ref()?;
    outside_reason(
        &target,
        roots,
        options.platform.unwrap_or(PathPlatform::Posix),
    )
}

/// The issue that keeps the whole plugin from loading, or `None` when it may load.
pub async fn manifest_gate(files: &dyn PluginFiles, options: &GateOptions) -> Option<PluginIssue> {
    let signal = options.signal.as_ref();
    if let Some(root) = root_issue(files, signal).await {
        return Some(root);
    }
    let location = STANDARD_LAYOUT.of(ComponentKind::Manifest).location;
    let (folder, file) =
        promise_all_pair(files.stat(FOLDER, signal), files.stat(location, signal)).await;
    let reason = manifest_reason(folder.as_ref(), file.as_ref(), options)?;
    let shown = match &options.roots {
        None => location.to_owned(),
        Some(roots) => format!("{}/{location}", roots.plugin_root),
    };
    Some(error(location, &manifest_not_read(&shown, &reason)))
}

crate::registry::exports! {
    "loading/manifest-gate" {
        "manifestGate" => async ok |files: Port, options: Option<GateOptions>|
            manifest_gate(&files, &options.unwrap_or_default()).await;
    }
}
