//! Port of `src/cli/plugin-details.ts`, switched.
//!
//! `claude plugin details <name>` (plugins/cli-reference.md, "plugin details"): the plugin's name,
//! version, description and source, then `Component inventory`, `Projected token cost` and
//! `Per-component (rounded)`; a plugin that is not loaded prints a message and exits `1`. Pure: the
//! host loads the plugin and estimates the tokens. The line layout is the kit's (spec/judgments.md);
//! the section names and the two messages are the docs'.

use serde::Deserialize;

use super::result::Rendered;
use crate::js::{math_round, number_to_string};

/// The kind of a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    /// `skill`.
    Skill,
    /// `agent`.
    Agent,
    /// `command`.
    Command,
}

impl ComponentKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
            Self::Command => "command",
        }
    }
}

/// What one component costs.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentCost {
    /// Its kind.
    pub kind: ComponentKind,
    /// Its name.
    pub name: String,
    /// Tokens the component adds to every session.
    pub always_on: f64,
    /// Tokens it adds when invoked.
    pub on_invoke: f64,
}

/// The names of what the plugin contributes, by kind.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    /// Skills.
    pub skills: Vec<String>,
    /// Agents.
    pub agents: Vec<String>,
    /// Hooks.
    pub hooks: Vec<String>,
    /// MCP servers.
    pub mcp_servers: Vec<String>,
    /// LSP servers.
    pub lsp_servers: Vec<String>,
}

/// A loaded plugin.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDetails {
    /// Its name.
    pub name: String,
    /// Its version.
    pub version: String,
    /// Its description.
    #[serde(default)]
    pub description: Option<String>,
    /// Where it comes from.
    pub source: String,
    /// The names of what the plugin contributes, by kind.
    pub inventory: Inventory,
    /// The always-on tokens the plugin adds to every session.
    pub projected_tokens: f64,
    /// What each component costs.
    pub components: Vec<ComponentCost>,
}

/// "rounded": to the nearest ten tokens above 100 and the nearest token below.
#[must_use]
pub fn round_tokens(tokens: f64) -> f64 {
    if tokens < 100.0 {
        math_round(tokens)
    } else {
        math_round(tokens / 10.0) * 10.0
    }
}

fn text_of(details: &PluginDetails) -> Vec<String> {
    let inventory = &details.inventory;
    let labelled: [(&str, &Vec<String>); 5] = [
        ("Skills", &inventory.skills),
        ("Agents", &inventory.agents),
        ("Hooks", &inventory.hooks),
        ("MCP servers", &inventory.mcp_servers),
        ("LSP servers", &inventory.lsp_servers),
    ];
    let mut head = vec![
        format!("Name: {}", details.name),
        format!("Version: {}", details.version),
    ];
    if let Some(description) = &details.description {
        head.push(format!("Description: {description}"));
    }
    head.push(format!("Source: {}", details.source));
    head.push(String::new());
    head.push("Component inventory".to_owned());
    for (label, names) in labelled {
        let listed = if names.is_empty() {
            String::new()
        } else {
            format!("  {}", names.join(", "))
        };
        head.push(format!("  {label} ({}){listed}", names.len()));
    }
    head.push(String::new());
    head.push("Projected token cost".to_owned());
    head.push(format!(
        "  Always-on: ~{} tokens per session",
        number_to_string(round_tokens(details.projected_tokens))
    ));
    if details.components.is_empty() {
        return head;
    }
    head.push(String::new());
    head.push("Per-component (rounded)".to_owned());
    head.extend(details.components.iter().map(|one| {
        format!(
            "  {} {}: always-on ~{}, on-invoke ~{}",
            one.kind.as_str(),
            one.name,
            number_to_string(round_tokens(one.always_on)),
            number_to_string(round_tokens(one.on_invoke))
        )
    }));
    head
}

fn not_found(name: &str) -> String {
    format!(
        "Plugin \"{name}\" not found. Run `claude plugin list` to see installed plugins, or pass --plugin-dir <path> to load one from disk."
    )
}

fn builtin_not_found(name: &str) -> String {
    format!(
        "Plugin \"{name}\" not found. Built-in plugins don't appear in `claude plugin list`. To see the ones available in a session, run /plugin in Claude Code and open the Installed tab."
    )
}

/// What the command prints: the details of a loaded plugin, or the message for one that is not loaded.
#[must_use]
pub fn plan_plugin_details(requested: &str, found: Option<&PluginDetails>) -> Rendered {
    if let Some(found) = found {
        return Rendered {
            stdout: text_of(found),
            stderr: Vec::new(),
            exit_code: 0,
        };
    }
    let message = if requested.ends_with("@builtin") {
        builtin_not_found(requested)
    } else {
        not_found(requested)
    };
    Rendered {
        stdout: Vec::new(),
        stderr: vec![message],
        exit_code: 1,
    }
}

crate::registry::exports! {
    "cli/plugin-details" {
        "roundTokens" => ok |tokens: f64| round_tokens(tokens);
        "planPluginDetails" => ok |requested: String, found: Option<PluginDetails>| plan_plugin_details(&requested, found.as_ref());
    }
}
