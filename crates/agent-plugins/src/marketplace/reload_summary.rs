//! Port of `src/marketplace/reload-summary.ts`, switched.
//!
//! `/reload-plugins` (plugins/cli-reference.md, "/reload-plugins", "Reload summary", "Reloads that
//! change MCP tools", "Sessions without an interactive terminal"; plugins/troubleshooting.md, `Run
//! /reload-plugins to apply.`): the one line it prints, whether the reload is applied or refused,
//! and who may run it. Pure texts and decisions over counts the host measured.
//!
//! Judgments (spec/judgments.md): the clauses are joined to the counts by a space, dependencies
//! before errors, with no pluralization (the docs write `N plugins`, `N errors`); several servers
//! are listed in the order added, then removed, joined by `, `; when a reload changes both an MCP
//! server and the `LSP` tool the line names the MCP servers.

use serde::{Deserialize, Serialize};

use crate::js::{number_to_string, trim};

/// What the reload loaded.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadCounts {
    /// Plugins.
    pub plugins: f64,
    /// `skillCount` of every plugin's skills.
    pub skills: f64,
    /// The agents loaded in the session, "including ones that don't come from plugins".
    pub agents: f64,
    /// Hooks.
    pub hooks: f64,
    /// Plugin MCP servers.
    pub mcp_servers: f64,
    /// Plugin LSP servers.
    pub lsp_servers: f64,
    /// Plugins that failed to load.
    pub errors: f64,
    /// Plugin dependencies this reload installed because they were missing.
    #[serde(default)]
    pub resolved_dependencies: Option<Vec<String>>,
}

/// What a plugin holds, for `skillCount`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSources {
    /// Its `commands/` entries.
    pub commands: f64,
    /// Its `SKILL.md` skills.
    pub skill_files: f64,
}

/// "The skills count covers every skill a plugin provides, both its `commands/` entries and its `SKILL.md` skills."
#[must_use]
pub fn skill_count(plugin: &SkillSources) -> f64 {
    plugin.commands + plugin.skill_files
}

/// The session the reload runs in.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ReloadSession {
    /// It has an interactive terminal.
    pub interactive: bool,
}

/// The one line `/reload-plugins` prints.
#[must_use]
pub fn reload_summary(counts: &ReloadCounts, session: &ReloadSession) -> String {
    let mut parts = vec![
        format!("{} plugins", number_to_string(counts.plugins)),
        format!("{} skills", number_to_string(counts.skills)),
        format!("{} agents", number_to_string(counts.agents)),
        format!("{} hooks", number_to_string(counts.hooks)),
    ];
    if session.interactive {
        parts.push(format!(
            "{} plugin MCP servers",
            number_to_string(counts.mcp_servers)
        ));
    }
    parts.push(format!(
        "{} plugin LSP servers",
        number_to_string(counts.lsp_servers)
    ));
    let resolved = counts.resolved_dependencies.as_deref().unwrap_or_default();
    let dependencies = if resolved.is_empty() {
        None
    } else {
        Some(format!(
            "(+ {} dependencies: {}) resolved",
            resolved.len(),
            resolved.join(", ")
        ))
    };
    let errors = (counts.errors > 0.0).then(|| {
        format!(
            "{} errors during load. Run /plugin for details.",
            number_to_string(counts.errors)
        )
    });
    let mut line = vec![format!("Reloaded: {}", parts.join(" \u{b7} "))];
    line.extend(dependencies);
    line.extend(errors);
    line.join(" ")
}

// ---- the prompt cache ----

/// What a reload does to the LSP tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspChange {
    /// The reload adds the tool.
    Adds,
    /// The reload removes it.
    Removes,
    /// The reload leaves it.
    Unchanged,
}

/// What a reload changes.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadChange {
    /// MCP servers the reload adds.
    pub added_mcp: Vec<String>,
    /// MCP servers the reload removes.
    pub removed_mcp: Vec<String>,
    /// The reload adds or removes the `LSP` tool.
    pub lsp: LspChange,
    /// The change would invalidate the prompt cache.
    pub invalidates_prompt_cache: bool,
}

/// Whether the reload is applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ReloadGate {
    /// `{ apply: true, mcpDeferred }`.
    Apply {
        /// Always true.
        apply: bool,
        /// The MCP servers' changes wait for the next session.
        #[serde(rename = "mcpDeferred")]
        mcp_deferred: bool,
    },
    /// `{ apply: false, message }`.
    Refuse {
        /// Always false.
        apply: bool,
        /// What is printed.
        message: String,
    },
}

const RE_READ: &str = " \u{2014} your next message will re-read the whole conversation instead of using the cache. Run /reload-plugins --force to apply.";

const fn lsp_line(lsp: LspChange) -> Option<&'static str> {
    match lsp {
        LspChange::Adds => Some("This reload adds the LSP tool"),
        LspChange::Removes => Some("This reload removes the LSP tool"),
        LspChange::Unchanged => None,
    }
}

fn refusal(change: &ReloadChange) -> Option<String> {
    let servers: Vec<&str> = change
        .added_mcp
        .iter()
        .chain(&change.removed_mcp)
        .map(String::as_str)
        .collect();
    if !servers.is_empty() {
        return Some(format!(
            "This reload changes MCP tools ({}){RE_READ}",
            servers.join(", ")
        ));
    }
    lsp_line(change.lsp).map(|line| format!("{line}{RE_READ}"))
}

/// What `reloadGate` takes besides the change.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct GateOptions {
    /// `--force`.
    pub force: bool,
    /// The session has an interactive terminal.
    pub interactive: bool,
}

/// "When the reload would add or remove a plugin MCP server or the `LSP` tool, and that change would
/// invalidate the prompt cache, Claude Code doesn't apply the reload" and prints `This reload changes
/// MCP tools (<server>) — your next message will re-read the whole conversation instead of using the
/// cache. Run /reload-plugins --force to apply.`; `--force` applies it anyway. Without an
/// interactive terminal "the reload doesn't connect or disconnect plugin MCP servers. Those changes
/// take effect in your next session", so it is applied with the servers deferred.
#[must_use]
pub fn reload_gate(change: &ReloadChange, options: &GateOptions) -> ReloadGate {
    if !options.interactive {
        return ReloadGate::Apply {
            apply: true,
            mcp_deferred: true,
        };
    }
    let message = if change.invalidates_prompt_cache && !options.force {
        refusal(change)
    } else {
        None
    };
    match message {
        None => ReloadGate::Apply {
            apply: true,
            mcp_deferred: false,
        },
        Some(message) => ReloadGate::Refuse {
            apply: false,
            message,
        },
    }
}

/// What `/reload-plugins [--force]` was typed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ReloadArguments {
    /// `--force`.
    pub force: bool,
}

/// `/reload-plugins [--force]`; "`force` without dashes works too".
#[must_use]
pub fn parse_reload_arguments(text: &str) -> ReloadArguments {
    ReloadArguments {
        force: trim(text) == "--force" || trim(text) == "force",
    }
}

const REMOTE_REPLY: &str =
    "/reload-plugins isn't available over a remote connection in this session.";

/// How the command arrived.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadOrigin {
    /// The session has an interactive terminal.
    pub interactive: bool,
    /// The user typed it into the session themselves.
    pub typed_by_user: bool,
}

/// Whether the command runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ReloadAllowed {
    /// `{ allowed: true }`.
    Allowed {
        /// Always true.
        allowed: bool,
    },
    /// `{ allowed: false, reply }`.
    Refused {
        /// Always false.
        allowed: bool,
        /// What it replies.
        reply: String,
    },
}

/// In a session without an interactive terminal the command runs "only when you type it into the
/// session yourself"; when it arrives another way it replies and reloads nothing. An interactive
/// terminal runs it whichever way it arrives (the docs restrict only the other sessions).
#[must_use]
pub fn reload_allowed(session: &ReloadOrigin) -> ReloadAllowed {
    if session.interactive || session.typed_by_user {
        ReloadAllowed::Allowed { allowed: true }
    } else {
        ReloadAllowed::Refused {
            allowed: false,
            reply: REMOTE_REPLY.to_owned(),
        }
    }
}

crate::registry::exports! {
    "marketplace/reload-summary" {
        "skillCount" => ok |plugin: SkillSources| skill_count(&plugin);
        "reloadSummary" => ok |counts: ReloadCounts, session: ReloadSession| reload_summary(&counts, &session);
        "reloadGate" => ok |change: ReloadChange, options: GateOptions| reload_gate(&change, &options);
        "parseReloadArguments" => ok |text: String| parse_reload_arguments(&text);
        "reloadAllowed" => ok |session: ReloadOrigin| reload_allowed(&session);
    }
}
