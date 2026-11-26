//! Port of `src/loading/resolve/variables.ts`, switched.
//!
//! The three path variables Claude Code provides to plugin components (plugins/manifest-reference.md,
//! "Environment variables"). `${CLAUDE_PLUGIN_ROOT}` is the installed version, `${CLAUDE_PLUGIN_DATA}`
//! the persistent folder, `${CLAUDE_PROJECT_DIR}` the project root.

use serde::{Deserialize, Serialize};

use super::data_id::plugin_data_id;
use crate::ports::Signal;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// The three path variables.
pub const PLUGIN_VARIABLES: [PluginVariable; 3] = [
    PluginVariable::ClaudePluginRoot,
    PluginVariable::ClaudePluginData,
    PluginVariable::ClaudeProjectDir,
];

/// One of `PLUGIN_VARIABLES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginVariable {
    /// `CLAUDE_PLUGIN_ROOT`.
    ClaudePluginRoot,
    /// `CLAUDE_PLUGIN_DATA`.
    ClaudePluginData,
    /// `CLAUDE_PROJECT_DIR`.
    ClaudeProjectDir,
}

impl PluginVariable {
    /// The variable's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaudePluginRoot => "CLAUDE_PLUGIN_ROOT",
            Self::ClaudePluginData => "CLAUDE_PLUGIN_DATA",
            Self::ClaudeProjectDir => "CLAUDE_PROJECT_DIR",
        }
    }
}

/// Whether a name is one of the three.
#[must_use]
pub fn is_plugin_variable(name: &str) -> bool {
    PLUGIN_VARIABLES
        .iter()
        .any(|variable| variable.as_str() == name)
}

/// What a variable's value is read from: the plugin and the host's locations
/// (`SubstitutionScope` of `src/loading/resolve/scope.ts`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableScope<L> {
    /// The plugin's identifier.
    pub plugin_id: String,
    /// Where the host keeps things.
    pub locations: L,
    /// Cancels the work.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// Where each variable comes from, as the host reports it (native paths). The data folder is the
/// only one that is created: "created on first reference", so the host is asked here, and only when
/// a caller needs the value. Judgment: the port gets the sanitized `<id>` (`pluginDataId`), the
/// part of the path the docs define, so a host joins it and cannot disagree.
pub async fn plugin_variable_value<L: Locations>(
    variable: PluginVariable,
    scope: &VariableScope<L>,
) -> String {
    match variable {
        PluginVariable::ClaudePluginRoot => scope.locations.plugin_root(&scope.plugin_id).await,
        PluginVariable::ClaudePluginData => {
            let id = plugin_data_id(&scope.plugin_id);
            scope
                .locations
                .plugin_data(&id, scope.signal.as_ref())
                .await
        }
        PluginVariable::ClaudeProjectDir => scope.locations.project_dir().await,
    }
}

/// How a variable is written into text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathRendering {
    /// The path, with forward slashes on Windows.
    Normalized,
    /// The path as the host reports it.
    Native,
    /// `${env:NAME}`.
    EnvironmentReference,
    /// `${NAME}`.
    ShellReference,
}

/// Whether a rendering writes the variable's path into the text. A reference the shell or PowerShell
/// expands (`${CLAUDE_PLUGIN_DATA}`, `${env:CLAUDE_PLUGIN_DATA}`) does not, so it never asks the host
/// for the data folder.
#[must_use]
pub const fn rendering_needs_value(rendering: PathRendering) -> bool {
    match rendering {
        PathRendering::Normalized | PathRendering::Native => true,
        PathRendering::EnvironmentReference | PathRendering::ShellReference => false,
    }
}

/// Writes a variable into text (plugins/manifest-reference.md, "Quoting and path separators").
/// `value` is the variable's path; a rendering that does not need it (`renderingNeedsValue`) ignores
/// it. `variable` is one of the three or a path another layer owns (`SubstitutionScope.extraPaths`).
#[must_use]
pub fn render_plugin_variable(
    rendering: PathRendering,
    variable: &str,
    value: &str,
    windows: bool,
) -> String {
    match rendering {
        PathRendering::Normalized => {
            if windows {
                value.replace('\\', "/")
            } else {
                value.to_owned()
            }
        }
        PathRendering::Native => value.to_owned(),
        PathRendering::EnvironmentReference => format!("${{env:{variable}}}"),
        // Braced, so text right after the reference (`"${CLAUDE_PLUGIN_ROOT}"_old`) is not read as part of the name.
        PathRendering::ShellReference => format!("${{{variable}}}"),
    }
}

crate::registry::exports! {
    "loading/resolve/variables" {
        "PLUGIN_VARIABLES" => ok || PLUGIN_VARIABLES;
        "isPluginVariable" => ok |name: String| is_plugin_variable(&name);
        "pluginVariableValue" => async ok |variable: PluginVariable, scope: VariableScope<Port>|
            plugin_variable_value(variable, &scope).await;
        "renderingNeedsValue" => ok |rendering: PathRendering| rendering_needs_value(rendering);
        "renderPluginVariable" => ok |rendering: PathRendering, variable: String, value: String, windows: bool|
            render_plugin_variable(rendering, &variable, &value, windows);
    }
}
