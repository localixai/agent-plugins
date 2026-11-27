//! Port of `src/agents/catalog/scopes.ts`, switched.
//!
//! Where subagents come from and which wins a shared name (sub-agents.md, "Choose the subagent
//! scope", the table "Location, Scope, Priority"): managed settings, the `--agents` flag,
//! `.claude/agents/`, `~/.claude/agents/`, then a plugin's `agents/`. A registry typed over the
//! sources, so a new source fails the typecheck until it says its rank and where its folders are.
//! A built-in ranks below all of them (a user or project subagent named `Explore` "overrides the
//! built-in").
//!
//! A folder's `places` is a function in the TypeScript, a key of `SCOPE_PLACES` here, and the rules
//! cross as their data.

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::agent_record::{AGENT_SOURCES, AgentSource};
use crate::foundation::scope_folders::ScopePlaceKind;

/// One folder of a source: below each base directory, `.claude/agents` (and `<extra>/agents`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AgentFolder {
    /// The name `strictPluginOnlyCustomization` knows it by.
    pub origin: &'static str,
    /// Whether Claude Code watches it for changes ("watches `~/.claude/agents/` and
    /// `.claude/agents/`").
    pub watched: bool,
    /// The folders read below the base directories, in precedence order (`SCOPE_PLACES`).
    #[serde(skip)]
    pub places: ScopePlaceKind,
}

/// One source's rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AgentScopeRule {
    /// The Priority column: 1 is the highest.
    pub priority: u32,
    /// The folders read through the folder-scan port; none for the flag and for plugins.
    pub folders: &'static [AgentFolder],
}

/// The folder below a base directory's scope folder (`.claude`, or one of the host's extra
/// folders).
pub const AGENTS_FOLDER: &str = "agents";

/// `AGENT_SCOPE_RULES[source]`.
#[must_use]
pub const fn agent_scope_rule(source: AgentSource) -> AgentScopeRule {
    match source {
        AgentSource::Managed => AgentScopeRule {
            priority: 1,
            folders: &[AgentFolder {
                origin: "managedDirectory",
                watched: false,
                places: ScopePlaceKind::Managed,
            }],
        },
        AgentSource::Flag => AgentScopeRule {
            priority: 2,
            folders: &[],
        },
        AgentSource::Project => AgentScopeRule {
            priority: 3,
            folders: &[
                AgentFolder {
                    origin: "projectAgents",
                    watched: true,
                    places: ScopePlaceKind::Project,
                },
                AgentFolder {
                    // "Claude Code doesn't watch `.claude/agents/` inside directories added with
                    // `--add-dir`".
                    origin: "addDir",
                    watched: false,
                    places: ScopePlaceKind::Added,
                },
            ],
        },
        AgentSource::User => AgentScopeRule {
            priority: 4,
            folders: &[AgentFolder {
                origin: "userAgents",
                watched: true,
                places: ScopePlaceKind::User,
            }],
        },
        AgentSource::Plugin => AgentScopeRule {
            priority: 5,
            folders: &[],
        },
    }
}

/// The priority a built-in has: after every source of the table.
pub const BUILTIN_PRIORITY: u32 = 6;

/// `AGENT_SCOPE_RULES` as it crosses: each rule's data, in the sources' order.
fn agent_scope_rules() -> IndexMap<AgentSource, AgentScopeRule> {
    AGENT_SOURCES
        .into_iter()
        .map(|source| (source, agent_scope_rule(source)))
        .collect()
}

crate::registry::exports! {
    "agents/catalog/scopes" {
        "AGENT_SCOPE_RULES" => ok agent_scope_rules;
    }
}
