//! The data members of `PluginHostOptions` (`src/host/types.ts`, which holds only types): what the
//! core reads of the host's options besides the plugins, the settings and the ports, which stay the
//! host's (`host/state.rs` keeps them in the state).

use serde::Deserialize;

use crate::js::JsValue;

/// The data members of `PluginHostOptions`: what the core reads of the host's options besides the
/// plugins, the settings and the ports (which stay the host's).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFacts {
    /// Paths and shells are Windows'.
    #[serde(default)]
    pub windows: Option<bool>,
    /// Which shells the host has.
    #[serde(default)]
    pub shells: JsValue,
    /// The host's process id.
    #[serde(default)]
    pub process_id: Option<f64>,
    /// HIPAA mode.
    #[serde(default)]
    pub hipaa: Option<bool>,
    /// The hook engine's shell and folders.
    #[serde(default)]
    pub hook_execution: JsValue,
    /// Where the shells are.
    #[serde(default)]
    pub shell_paths: JsValue,
    /// How a skill's shell commands are run.
    #[serde(default)]
    pub shell_arguments: JsValue,
    /// The host's extra scope folders.
    #[serde(default)]
    pub extra_scope_folders: Option<Vec<String>>,
    /// The launch environment wins over the settings `env`.
    #[serde(default)]
    pub launch_owns_environment: Option<bool>,
    /// The workflow context.
    #[serde(default)]
    pub workflows: JsValue,
}
