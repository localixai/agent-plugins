//! The core's counterpart of `src/ports/locations.ts` at sha256:7be87b6b6e61, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! The host's own paths: the home and project folders, where a plugin is and keeps its data, and
//! how two path parts join on its system.

use super::BoxFuture;

/// Where things are on the host.
pub trait Locations: Send + Sync {
    /// The user's home folder.
    fn home_dir(&self) -> String;
    /// The project folder the session runs in.
    fn project_dir(&self) -> String;
    /// The folder a plugin is loaded from.
    fn plugin_root(&self, plugin_id: &str) -> String;
    /// The plugin's data folder (`${CLAUDE_PLUGIN_DATA}`), created when first asked for.
    fn plugin_data<'a>(&'a self, plugin_id: &'a str) -> BoxFuture<'a, String>;
    /// `relative_path` joined to `base` as the host's system joins paths.
    fn join_path(&self, base: &str, relative_path: &str) -> String;
    /// The first of `candidates` that is an existing directory.
    fn existing_directory<'a>(&'a self, candidates: &'a [String]) -> BoxFuture<'a, Option<String>>;
}
