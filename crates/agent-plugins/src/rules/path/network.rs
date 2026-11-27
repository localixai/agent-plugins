//! Port of `src/rules/path/network.ts`, switched.
//!
//! Network paths (permissions.md, "Network paths"): "A network path is one that can reach another
//! computer: on Windows, a UNC path such as `\\server\share\file`, and on macOS and Linux, a `/net`
//! automount path such as `/net/fileserver/notes.txt`." Reading one always prompts, whatever an
//! allow rule or a hook says; the host asks `readsNetworkPath` before it trusts a rule.

use super::posix::{HostPaths, Platform, absolute_path};
use super::tools::{path_text, path_tool, requested_path};
use crate::rules::types::{PathContext, PreparedCall};

/// `IS_NETWORK[platform](path)`.
#[must_use]
pub fn is_network_path(path: &str, platform: Platform) -> bool {
    match platform {
        Platform::Windows => path.starts_with("\\\\") || path.starts_with("//"),
        Platform::Posix => path == "/net" || path.starts_with("/net/"),
    }
}

/// Whether `call` is a read (Read, Grep, Glob) of a network path, by the path it names or any path
/// it resolves to. Other tools have their own checks.
#[must_use]
pub fn reads_network_path(call: &PreparedCall, paths: &PathContext) -> bool {
    if !path_tool(&call.call.tool_name).is_some_and(|tool| tool.reads) {
        return false;
    }
    // As written (a UNC path keeps its two slashes only there) and as resolved (`../../net/x`).
    let named = [path_text(&call.call), requested_path(&call.call, paths)];
    let resolved = call.resolved_paths.as_deref().unwrap_or_default();
    let host = HostPaths::from(paths);
    named
        .into_iter()
        .flatten()
        .chain(resolved.iter().cloned())
        .chain(resolved.iter().map(|path| absolute_path(path, &host)))
        .any(|path| is_network_path(&path, paths.platform))
}

crate::registry::exports! {
    "rules/path/network" {
        "isNetworkPath" => ok |path: String, platform: Platform| is_network_path(&path, platform);
        "readsNetworkPath" => ok |call: PreparedCall, paths: PathContext| reads_network_path(&call, &paths);
    }
}
