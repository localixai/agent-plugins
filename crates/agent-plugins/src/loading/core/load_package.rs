//! Port of `src/loading/core/load-package.ts`, switched.
//!
//! The loader of an Extended client (Agent Plugins Extended §1.2, §1.3; docs/API.md, "Core
//! packages"): the form of the root is decided first, then a Claude Code plugin loads by
//! `loadPlugin` exactly as it does alone, and a core package by `loadCorePackage`. A dual package
//! is a Claude Code plugin: its core manifest, `mcp.json` and `hooks.json` are not read for loading
//! (J1.2). A core package's marketplace entry gives no components: what it configures is reported
//! (J18.15).

use serde::{Deserialize, Serialize};

use super::entry_fields::{MarketplaceEntry, core_entry_issues};
use super::form::{PluginForm, detect_plugin_form};
use super::load_core::{
    CoreLoadOptions, LoadCorePackageInput, LoadedCorePackage, load_core_package,
};
use crate::js::value::to_js_value;
use crate::loading::load_plugin::{LoadPluginInput, LoadedPlugin, load_plugin};
use crate::ports::BoxFuture;

/// What a package's load gives (`LoadedPackage`), by the form of its root.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "form")]
pub enum LoadedPackage {
    /// A Claude Code plugin.
    #[serde(rename = "claude-code")]
    ClaudeCode {
        /// The root also carries a core manifest, which was not loaded (§1.3.1: say so in the
        /// details).
        dual: bool,
        /// The plugin's load.
        loaded: LoadedPlugin,
    },
    /// A core package.
    #[serde(rename = "core")]
    Core {
        /// Always false.
        dual: bool,
        /// The package's load.
        loaded: LoadedCorePackage,
    },
}

/// What a package's load is given (`LoadPluginPackageInput`).
#[derive(Clone, Debug, Deserialize)]
pub struct LoadPluginPackageInput {
    /// What `loadPlugin` reads.
    #[serde(flatten)]
    pub plugin: LoadPluginInput,
    /// What `loadCorePackage` reads besides.
    #[serde(flatten)]
    pub core: CoreLoadOptions,
}

/// One form's loader (`LOADERS[form]`).
type Loader = fn(LoadPluginPackageInput, bool) -> BoxFuture<'static, LoadedPackage>;

fn claude_code(input: LoadPluginPackageInput, dual: bool) -> BoxFuture<'static, LoadedPackage> {
    Box::pin(async move {
        LoadedPackage::ClaudeCode {
            dual,
            loaded: load_plugin(input.plugin).await,
        }
    })
}

fn core(input: LoadPluginPackageInput, _dual: bool) -> BoxFuture<'static, LoadedPackage> {
    Box::pin(async move {
        let package = LoadCorePackageInput {
            files: input.plugin.files,
            extensions: input.core.extensions,
            transports: input.core.transports,
            icon_formats: input.core.icon_formats,
            signal: input.plugin.signal,
        };
        let mut loaded = load_core_package(package).await;
        // The entry as `entry-fields` reads it: its name, `strict` and the component keys it sets.
        let entry = input.plugin.entry.as_ref().and_then(|entry| {
            let value = to_js_value(entry).ok()?;
            MarketplaceEntry::deserialize(value).ok()
        });
        loaded.issues.extend(core_entry_issues(entry.as_ref()));
        LoadedPackage::Core {
            dual: false,
            loaded,
        }
    })
}

/// One loader per form: a registry over `PluginForm` (K1).
const fn loader_of(form: PluginForm) -> Loader {
    match form {
        PluginForm::ClaudeCode => claude_code,
        PluginForm::Core => core,
    }
}

/// Loads a plugin root in the form §1.3 decides. Never throws.
pub async fn load_plugin_package(input: LoadPluginPackageInput) -> LoadedPackage {
    let decision = detect_plugin_form(&input.plugin.files, input.plugin.signal.as_ref()).await;
    loader_of(decision.form)(input, decision.dual).await
}

crate::registry::exports! {
    "loading/core/load-package" {
        "loadPluginPackage" => async ok |input: LoadPluginPackageInput| load_plugin_package(input).await;
    }
}
