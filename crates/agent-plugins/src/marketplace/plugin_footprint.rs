//! Port of `src/marketplace/plugin-footprint.ts`, switched.
//!
//! What an enabled plugin adds to every session (plugins/overview.md, "What an enabled plugin adds to
//! your sessions"): pure over a plain summary of the plugin's components, so no layer above is needed.
//! The token counts are the host's; the kit says which components cost context, which run as processes,
//! and where to look and what to run at each stage.

use serde::{Deserialize, Serialize};

use crate::format::marketplace::marketplace_name_lists::OFFICIAL_NAMES;

/// One skill, agent or command of a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSummary {
    /// Its name.
    pub name: String,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// "Claude can invoke on its own": false for a skill or command with `disable-model-invocation`.
    pub model_invocable: bool,
}

/// A plain summary of a plugin's components.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    /// `<name>@<marketplace>`.
    pub id: String,
    /// Its skills.
    pub skills: Vec<ComponentSummary>,
    /// Its agents.
    pub agents: Vec<ComponentSummary>,
    /// Its commands.
    pub commands: Vec<ComponentSummary>,
    /// Its MCP servers.
    pub mcp_servers: Vec<String>,
    /// The hook events the plugin registers handlers for.
    pub hook_events: Vec<String>,
}

/// What kind of component an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextKind {
    /// A skill.
    Skill,
    /// An agent.
    Agent,
    /// A command.
    Command,
}

/// One component whose name and description are in Claude's context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextEntry {
    /// What it is.
    pub kind: ContextKind,
    /// Its name.
    pub name: String,
    /// Its description.
    pub description: String,
}

/// "Context and usage": "for each skill, agent, and command that Claude can invoke on its own, the name and
/// description are in Claude's context on every turn", in every session, even when nothing from the plugin
/// runs. "The full text of a skill or agent loads only when it's used", so the body is not an entry.
#[must_use]
pub fn context_entries(plugin: &PluginSummary) -> Vec<ContextEntry> {
    let of = |kind: ContextKind, items: &[ComponentSummary]| -> Vec<ContextEntry> {
        items
            .iter()
            .filter(|item| item.model_invocable)
            .map(|item| ContextEntry {
                kind,
                name: item.name.clone(),
                description: item.description.clone().unwrap_or_default(),
            })
            .collect()
    };
    let mut entries = of(ContextKind::Skill, &plugin.skills);
    entries.extend(of(ContextKind::Agent, &plugin.agents));
    entries.extend(of(ContextKind::Command, &plugin.commands));
    entries
}

/// What a plugin runs as processes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Processes {
    /// "MCP servers the plugin defines run alongside each session where it's enabled".
    pub mcp_servers: Vec<String>,
    /// "its hooks fire at their events".
    pub hook_events: Vec<String>,
}

/// What a plugin runs as processes.
#[must_use]
pub fn processes_of(plugin: &PluginSummary) -> Processes {
    Processes {
        mcp_servers: plugin.mcp_servers.clone(),
        hook_events: plugin.hook_events.clone(),
    }
}

/// "What the plugin runs, it runs as you."
pub const RUNS_AS: &str = "the user";

/// "Plugins in Anthropic's official marketplace show a **Context cost** estimate" in the Marketplaces tab.
#[must_use]
pub fn shows_context_cost(marketplace: &str) -> bool {
    OFFICIAL_NAMES.contains(&marketplace)
}

/// Where to look before an install.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeInstall {
    /// Where.
    pub r#where: String,
    /// The marketplace shows a context cost.
    pub context_cost: bool,
}

/// Where to look after an install.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AfterInstall {
    /// Where.
    pub r#where: Vec<String>,
}

/// How to stop a plugin without uninstalling it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StopWithoutUninstalling {
    /// In the panel.
    pub panel: String,
    /// In a shell.
    pub shell: String,
}

/// The three stages the docs name and where to look or what to run at each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FootprintGuide {
    /// Before the install.
    pub before_install: BeforeInstall,
    /// After the install.
    pub after_install: AfterInstall,
    /// To stop without uninstalling.
    pub stop_without_uninstalling: StopWithoutUninstalling,
}

/// The three stages the docs name and where to look or what to run at each.
#[must_use]
pub fn footprint_guide(id: &str, marketplace: &str) -> FootprintGuide {
    FootprintGuide {
        before_install: BeforeInstall {
            r#where: "the Marketplaces tab in /plugin".to_owned(),
            context_cost: shows_context_cost(marketplace),
        },
        after_install: AfterInstall {
            r#where: vec![
                "Measure what a plugin costs".to_owned(),
                "the Installed tab, Not used recently group".to_owned(),
            ],
        },
        stop_without_uninstalling: StopWithoutUninstalling {
            panel: "/plugin".to_owned(),
            shell: format!("claude plugin disable {id}"),
        },
    }
}

crate::registry::exports! {
    "marketplace/plugin-footprint" {
        "contextEntries" => ok |plugin: PluginSummary| context_entries(&plugin);
        "processesOf" => ok |plugin: PluginSummary| processes_of(&plugin);
        "runsAs" => ok || RUNS_AS;
        "showsContextCost" => ok |marketplace: String| shows_context_cost(&marketplace);
        "footprintGuide" => ok |id: String, marketplace: String| footprint_guide(&id, &marketplace);
    }
}
