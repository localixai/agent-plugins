//! Port of `src/loading/resolve/without-plugin.ts`, switched.
//!
//! Judgment (spec/judgments.md): a handler of no plugin (a settings file's) has no plugin folder
//! and no data folder, so the two plugin variables stay as written instead of naming the folder of
//! a plugin that is not there. The docs show these two only in plugin hooks. Every method
//! delegates, since a host's `Locations` may be a class instance.
//!
//! Across the boundary the result is a handle whose methods the host calls; those the TypeScript
//! answers at once stay sync for the host, which answers the host's own `Locations` as they ask.

use crate::ports::locations::Locations;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Handle, Port};

/// `${CLAUDE_PLUGIN_ROOT}`, as written.
const PLUGIN_ROOT: &str = "${CLAUDE_PLUGIN_ROOT}";
/// `${CLAUDE_PLUGIN_DATA}`, as written.
const PLUGIN_DATA: &str = "${CLAUDE_PLUGIN_DATA}";

/// The host's `Locations` with no plugin: the two plugin folders stay their variables.
#[derive(Debug)]
pub struct WithoutPlugin<L>(L);

/// `locations` for a handler of no plugin.
pub const fn locations_without_plugin<L: Locations>(locations: L) -> WithoutPlugin<L> {
    WithoutPlugin(locations)
}

impl<L: Locations> Locations for WithoutPlugin<L> {
    fn home_dir(&self) -> BoxFuture<'_, String> {
        self.0.home_dir()
    }

    fn project_dir(&self) -> BoxFuture<'_, String> {
        self.0.project_dir()
    }

    fn plugin_root<'a>(&'a self, _: &'a str) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(PLUGIN_ROOT.to_owned()))
    }

    fn plugin_data<'a>(&'a self, _: &'a str, _: Option<&'a Signal>) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(PLUGIN_DATA.to_owned()))
    }

    fn join_path<'a>(&'a self, base: &'a str, relative_path: &'a str) -> BoxFuture<'a, String> {
        self.0.join_path(base, relative_path)
    }

    fn existing_directory<'a>(
        &'a self,
        candidates: &'a [String],
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        self.0.existing_directory(candidates, signal)
    }
}

crate::registry::methods! {
    WithoutPlugin<Port> {
        "homeDir" => sync async ok |this| this.home_dir().await;
        "projectDir" => sync async ok |this| this.project_dir().await;
        "pluginRoot" => ok |_this| PLUGIN_ROOT;
        "pluginData" => async ok |this| this.plugin_data("", None).await;
        "joinPath" => sync async ok |this, base: String, relative_path: String|
            this.join_path(&base, &relative_path).await;
        "existingDirectory" => async ok |this, candidates: Vec<String>, signal: Option<Signal>|
            this.existing_directory(&candidates, signal.as_ref()).await;
    }
}

crate::registry::exports! {
    "loading/resolve/without-plugin" {
        "locationsWithoutPlugin" => ok |locations: Port| Handle::new(locations_without_plugin(locations));
    }
}
