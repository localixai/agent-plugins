//! The core's counterpart of `src/marketplace/types.ts` at sha256:0b08d4511a4b., which stays TypeScript: it declares
//! types only, so it has nothing to run (docs/RUST.md, "What stays TypeScript"). Only the types
//! the ported modules of this folder use are declared here; the rest follow with the modules that
//! use them.
//!
//! Contracts of the marketplace module (layer 3): the plans a host executes and the small values
//! several pieces share (docs/API.md, "Marketplace"). Planning is pure; these are what it returns.
//! Pages: plugins/marketplace-reference.md ("Plugin sources", "Marketplace sources"),
//! plugins/host-marketplace.md ("Stay within the download limits", "Rename or remove a plugin"),
//! plugins/dependencies.md.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Header names are lower case, as on the ports (`HttpRequest`).
pub type HeaderMap = IndexMap<String, String>;

/// Which of the marketplace repository's git submodules a local plugin needs (marketplace-
/// reference.md, "Git submodules in the marketplace repository"): `none` for a sparse checkout or a
/// marketplace that was not cloned, `plugin-subtree` for a plugin in a subdirectory (the submodules
/// it is in, holds or links into), `whole-repository` for a plugin at the root (all of them or none).
/// `selectSubmodules` applies the rule to the repository's files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmoduleRule {
    /// `none`.
    #[serde(rename = "none")]
    None,
    /// `plugin-subtree`.
    #[serde(rename = "plugin-subtree")]
    PluginSubtree,
    /// `whole-repository`.
    #[serde(rename = "whole-repository")]
    WholeRepository,
}

/// A dependency with its defaults applied (plugins/dependencies.md): the marketplace is the
/// declaring one when omitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedDependency {
    /// The plugin's name.
    pub name: String,
    /// Its marketplace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<String>,
    /// Its node-semver range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

/// Where a plugin name stands in a marketplace with a `renames` map (`RenameStatus`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum RenameStatus {
    /// It is listed.
    Current,
    /// It loads under another name.
    Renamed {
        /// The new name.
        to: String,
    },
    /// It is gone.
    Removed,
    /// Claude Code has no answer.
    Unknown,
}
