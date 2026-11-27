//! Port of `src/host/core-view.ts`, switched.
//!
//! A core package as the session's plugin machinery sees it (Agent Plugins Extended §18.1.5, J18.15):
//! a plugin to every part of the session that is not a component, known by its id, enabled,
//! restricted, named in conflicts and depended on like any plugin. Its view is a `Plugin` with the
//! package's name and skills and no Claude Code component at all: its `commands/`, `agents/`,
//! `.mcp.json`, `hooks/hooks.json`, `bin/` and `settings.json` are ordinary files (§18.1.1 item 3),
//! no `userConfig` is declared (§18.1.3 item 5) and the manifest is not Claude Code's. Its MCP
//! servers and hook handlers run from the package itself (`core`).
//!
//! Each `Plugin` and `CorePackage` object the host gives crosses once, as a [`GivenPlugin`] the
//! edge keeps by the object's identity (`src/host/core-view.ts`, as an engine numbers its
//! handlers): the core tells a core package from a plugin by the copy, never by the object.

use std::sync::{Arc, OnceLock};

use serde::Serialize;

use super::catalog::PluginCatalog;
use crate::js::{JsObject, JsValue};
use crate::registry::Handle;

/// The core's copy of one plugin the host gave: the `Plugin` as data without its Markdown (a core
/// package's whole view), the package for a core package, and the catalog of its Markdown, read at
/// the first use from the Markdown the edge gives then (`readCatalog`, `createHostState`), so a
/// plugin that never runs has its Markdown neither read nor copied.
#[derive(Debug, Default)]
pub struct GivenPlugin {
    /// The `Plugin` (for a core package, its view).
    pub plugin: JsValue,
    /// The `CorePackage` the view is of; `None` for a Claude Code plugin (`coreOf`).
    pub core: Option<JsValue>,
    catalog: OnceLock<Arc<PluginCatalog>>,
}

impl GivenPlugin {
    /// The plugin's manifest name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.plugin
            .get("name")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
    }

    /// The catalog, when it was read.
    #[must_use]
    pub fn known_catalog(&self) -> Option<Arc<PluginCatalog>> {
        self.catalog.get().cloned()
    }

    /// The catalog, read once by `read`.
    pub(crate) fn catalog(
        &self,
        read: impl FnOnce() -> Result<PluginCatalog, String>,
    ) -> Result<Arc<PluginCatalog>, String> {
        if let Some(known) = self.catalog.get() {
            return Ok(Arc::clone(known));
        }
        let made = Arc::new(read()?);
        Ok(Arc::clone(self.catalog.get_or_init(|| made)))
    }
}

crate::registry::methods! {
    GivenPlugin {}
}

/// The view of a package: a plugin with the package's name and skills and no component.
#[must_use]
pub fn core_plugin_view(core: &JsValue) -> JsValue {
    let empty_list = || JsValue::Array(Vec::new());
    let empty_map = || JsValue::Object(JsObject::new());
    let member = |name: &str| core.get(name).cloned().unwrap_or_default();
    JsValue::Object(JsObject::from([
        ("name".to_owned(), member("name")),
        ("manifest".to_owned(), JsValue::Undefined),
        ("skills".to_owned(), member("skills")),
        ("commands".to_owned(), empty_list()),
        ("agents".to_owned(), empty_list()),
        ("hooks".to_owned(), empty_map()),
        ("mcpServers".to_owned(), empty_map()),
        ("mcpBundles".to_owned(), empty_map()),
        ("lspServers".to_owned(), empty_map()),
        ("monitors".to_owned(), empty_list()),
        ("outputStyles".to_owned(), empty_list()),
        ("workflows".to_owned(), empty_list()),
        ("themes".to_owned(), empty_list()),
        ("settings".to_owned(), empty_map()),
        ("userConfig".to_owned(), empty_map()),
        ("bin".to_owned(), empty_list()),
        ("hasBin".to_owned(), JsValue::Bool(false)),
    ]))
}

/// A core package's copy and its view, as the edge keeps them.
#[derive(Debug, Serialize)]
pub struct CoreGiven {
    /// The core's copy.
    pub given: Handle,
    /// The view the edge's `HostPlugin` carries.
    pub view: JsValue,
}

/// The core's copy of a plugin the host gave.
#[must_use]
pub fn plugin_data(plugin: JsValue) -> Handle {
    Handle::own(GivenPlugin {
        plugin,
        ..GivenPlugin::default()
    })
}

/// The core's copy of a core package the host gave, with its view.
#[must_use]
pub fn core_package_data(package: JsValue) -> CoreGiven {
    let view = core_plugin_view(&package);
    let given = Handle::own(GivenPlugin {
        plugin: view.clone(),
        core: Some(package),
        ..GivenPlugin::default()
    });
    CoreGiven { given, view }
}

crate::registry::exports! {
    "host/core-view" {
        "pluginData" => ok |plugin: JsValue| plugin_data(plugin);
        "corePackageData" => ok |package: JsValue| core_package_data(package);
    }
}
