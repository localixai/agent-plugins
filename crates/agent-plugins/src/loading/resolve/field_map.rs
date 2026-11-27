//! Port of `src/loading/resolve/field-map.ts`, switched.
//!
//! The table "Where each variable resolves" (plugins/manifest-reference.md): per plugin component,
//! the fields where `${...}` resolves inline and what the process receives in its environment.
//! Each resolver reads its field list here, so a component Claude Code adds is one row.

use serde::{Deserialize, Serialize};

/// The components that resolve variables.
pub const VARIABLE_COMPONENTS: [VariableComponent; 6] = [
    VariableComponent::HookCommand,
    VariableComponent::MonitorCommand,
    VariableComponent::McpStdio,
    VariableComponent::McpRemote,
    VariableComponent::Lsp,
    VariableComponent::MarkdownContent,
];

/// One of `VARIABLE_COMPONENTS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VariableComponent {
    /// A hook's command.
    HookCommand,
    /// A monitor's command.
    MonitorCommand,
    /// A stdio MCP server.
    McpStdio,
    /// A remote MCP server.
    McpRemote,
    /// A language server.
    Lsp,
    /// The body of a Markdown component.
    MarkdownContent,
}

/// A field `${...}` resolves in; `body` is "anywhere in the Markdown body".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableField {
    /// `command`.
    Command,
    /// `args`.
    Args,
    /// `env`.
    Env,
    /// `url`.
    Url,
    /// `headers`.
    Headers,
    /// `headersHelper`.
    HeadersHelper,
    /// `workspaceFolder`.
    WorkspaceFolder,
    /// Anywhere in the Markdown body.
    Body,
}

/// The name of the option variables, `CLAUDE_PLUGIN_OPTION_<KEY>`, as the table writes it.
pub const OPTION_VARIABLES: &str = "CLAUDE_PLUGIN_OPTION_<KEY>";

/// The fields of a component and the variables its process receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ComponentVariables {
    /// Where `${...}` resolves.
    pub fields: &'static [VariableField],
    /// What the process receives.
    pub exported: &'static [&'static str],
}

const PATHS: &[&str] = &[
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PLUGIN_DATA",
    "CLAUDE_PROJECT_DIR",
];

/// `VARIABLE_FIELDS`: one row per component, keyed by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct VariableFields {
    /// A hook's command.
    pub hook_command: ComponentVariables,
    /// A monitor's command.
    pub monitor_command: ComponentVariables,
    /// A stdio MCP server.
    pub mcp_stdio: ComponentVariables,
    /// A remote MCP server.
    pub mcp_remote: ComponentVariables,
    /// A language server.
    pub lsp: ComponentVariables,
    /// The body of a Markdown component.
    pub markdown_content: ComponentVariables,
}

/// The table, row by row.
pub const VARIABLE_FIELDS: VariableFields = VariableFields {
    hook_command: ComponentVariables {
        fields: &[VariableField::Command, VariableField::Args],
        exported: &[
            "CLAUDE_PLUGIN_ROOT",
            "CLAUDE_PLUGIN_DATA",
            "CLAUDE_PROJECT_DIR",
            OPTION_VARIABLES,
        ],
    },
    monitor_command: ComponentVariables {
        fields: &[VariableField::Command],
        exported: &[],
    },
    mcp_stdio: ComponentVariables {
        fields: &[
            VariableField::Command,
            VariableField::Args,
            VariableField::Env,
        ],
        exported: PATHS,
    },
    // `http`, `sse` and `ws`: the process variables are "not applicable".
    mcp_remote: ComponentVariables {
        fields: &[
            VariableField::Url,
            VariableField::Headers,
            VariableField::HeadersHelper,
        ],
        exported: &[],
    },
    lsp: ComponentVariables {
        fields: &[
            VariableField::Command,
            VariableField::Args,
            VariableField::Env,
            VariableField::WorkspaceFolder,
        ],
        exported: PATHS,
    },
    markdown_content: ComponentVariables {
        fields: &[VariableField::Body],
        exported: &[],
    },
};

crate::registry::exports! {
    "loading/resolve/field-map" {
        "VARIABLE_COMPONENTS" => ok || VARIABLE_COMPONENTS;
        "OPTION_VARIABLES" => ok || OPTION_VARIABLES;
        "VARIABLE_FIELDS" => ok || VARIABLE_FIELDS;
    }
}
