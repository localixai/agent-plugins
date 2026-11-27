//! The core's counterpart of `src/ports/mcp-bundle.ts` at sha256:114d47b03dc7, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Opening an MCP bundle (plugins/manifest-reference.md, "mcpServers"): the host extracts or
//! downloads it and reads its manifest into a server config.

use serde::{Deserialize, Serialize};

use super::{BoxFuture, PortFailure, Signal};
use crate::js::JsValue;

/// Where an MCP bundle comes from: a path inside the plugin (relative to the plugin root,
/// `/`-separated, no `./`) or an `https://` URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum McpBundleSource {
    /// A path inside the plugin.
    Path {
        /// Root-relative.
        path: String,
    },
    /// A URL.
    Url {
        /// `https://`.
        url: String,
    },
}

/// What the host read from a bundle.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBundle {
    /// The `name` in the bundle's manifest: the server takes this name.
    pub name: String,
    /// The server config, read as any `mcpServers` entry.
    pub server: JsValue,
    /// The bundle's `user_config` block as its manifest writes it.
    #[serde(default)]
    pub user_config: JsValue,
}

/// The host's bundle extractor.
pub trait McpBundleReader: Send + Sync {
    /// The bundle at `source`, opened under `cache_dir`; `None` when there is nothing there.
    fn open<'a>(
        &'a self,
        source: &'a McpBundleSource,
        cache_dir: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Result<McpBundle, PortFailure>>>;
}
